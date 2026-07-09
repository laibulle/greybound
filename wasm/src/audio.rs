use greybound::ir::SpeakerStage;
use greybound::{
    AmpControls, DeviceConfig, DeviceControls, DeviceSlotConfig, DeviceSlotControls,
    MinotaurControls, SignalChain, SignalChainConfig, SignalChainControls, SpringfieldControls,
};
use greybound_ui::{AppProfile, GreyboundUi, RuntimeDeviceSection};
use js_sys::{Object, Reflect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContext, AudioProcessingEvent, MediaStream, MediaStreamConstraints, MediaStreamTrack,
    ScriptProcessorNode,
};

thread_local! {
    static WEB_AUDIO_ENGINE: RefCell<Option<WebAudioEngine>> = const { RefCell::new(None) };
}

#[derive(Clone)]
pub struct WebAudioSnapshot {
    controls: RuntimeControls,
    app_profile: AppProfile,
    amp_model: &'static str,
    period_size: u32,
}

impl WebAudioSnapshot {
    pub fn from_ui(ui: &GreyboundUi) -> Self {
        Self {
            controls: RuntimeControls::from_ui(ui),
            app_profile: ui.app_profile,
            amp_model: ui.amp_model_id(),
            period_size: ui.audio_settings.period_size,
        }
    }
}

pub async fn start(snapshot: WebAudioSnapshot) -> Result<String, String> {
    stop();
    let engine = WebAudioEngine::start(snapshot)
        .await
        .map_err(js_error_message)?;
    let status = engine.status();
    WEB_AUDIO_ENGINE.with(|slot| {
        *slot.borrow_mut() = Some(engine);
    });
    Ok(status)
}

pub fn stop() {
    WEB_AUDIO_ENGINE.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

pub fn store_controls_from_ui(ui: &GreyboundUi) {
    let controls = RuntimeControls::from_ui(ui);
    WEB_AUDIO_ENGINE.with(|slot| {
        if let Some(engine) = slot.borrow().as_ref() {
            engine.store_controls(controls);
        }
    });
}

pub fn meter_levels() -> (f32, f32, f32) {
    WEB_AUDIO_ENGINE
        .with(|slot| slot.borrow().as_ref().map(WebAudioEngine::meter_levels))
        .unwrap_or((0.0, 0.0, 0.0))
}

struct WebAudioEngine {
    context: AudioContext,
    stream: MediaStream,
    source: web_sys::MediaStreamAudioSourceNode,
    processor: ScriptProcessorNode,
    _callback: Closure<dyn FnMut(AudioProcessingEvent)>,
    controls: Rc<RefCell<RuntimeControls>>,
    meters: Rc<MeterStats>,
    sample_rate: f32,
    buffer_size: u32,
}

impl WebAudioEngine {
    async fn start(snapshot: WebAudioSnapshot) -> Result<Self, JsValue> {
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("missing browser window"))?;
        let media_devices = window.navigator().media_devices()?;
        let constraints = MediaStreamConstraints::new();
        constraints.set_audio(&raw_audio_constraints()?);
        constraints.set_video(&JsValue::FALSE);
        let stream_value =
            JsFuture::from(media_devices.get_user_media_with_constraints(&constraints)?).await?;
        let stream: MediaStream = stream_value.dyn_into()?;

        let context = AudioContext::new()?;
        let sample_rate = context.sample_rate();
        let buffer_size = supported_script_processor_size(snapshot.period_size);
        let source = context.create_media_stream_source(&stream)?;
        let processor = context
            .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
                buffer_size,
                1,
                2,
            )?;

        let runtime = Rc::new(RefCell::new(WebAudioRuntime::new(
            sample_rate,
            snapshot.amp_model,
            snapshot.app_profile,
        )));
        let controls = Rc::new(RefCell::new(snapshot.controls));
        let meters = Rc::new(MeterStats::default());

        let callback_runtime = runtime.clone();
        let callback_controls = controls.clone();
        let callback_meters = meters.clone();
        let callback = Closure::wrap(Box::new(move |event: AudioProcessingEvent| {
            if let Err(error) = process_audio_event(
                &event,
                &callback_runtime,
                &callback_controls,
                &callback_meters,
            ) {
                web_sys::console::error_1(&error);
            }
        }) as Box<dyn FnMut(_)>);
        processor.set_onaudioprocess(Some(callback.as_ref().unchecked_ref()));

        source.connect_with_audio_node(&processor)?;
        processor.connect_with_audio_node(&context.destination())?;
        let _ = JsFuture::from(context.resume()?).await;

        Ok(Self {
            context,
            stream,
            source,
            processor,
            _callback: callback,
            controls,
            meters,
            sample_rate,
            buffer_size,
        })
    }

    fn status(&self) -> String {
        format!(
            "Running WebAudio live input -> browser output, {:.0} Hz / {} samples",
            self.sample_rate, self.buffer_size
        )
    }

    fn store_controls(&self, controls: RuntimeControls) {
        *self.controls.borrow_mut() = controls;
    }

    fn meter_levels(&self) -> (f32, f32, f32) {
        self.meters.snapshot_levels()
    }
}

impl Drop for WebAudioEngine {
    fn drop(&mut self) {
        self.processor.set_onaudioprocess(None);
        let _ = self.source.disconnect();
        let _ = self.processor.disconnect();
        for track in self.stream.get_tracks().iter() {
            if let Ok(track) = track.dyn_into::<MediaStreamTrack>() {
                track.stop();
            }
        }
        let _ = self.context.close();
    }
}

fn process_audio_event(
    event: &AudioProcessingEvent,
    runtime: &Rc<RefCell<WebAudioRuntime>>,
    controls: &Rc<RefCell<RuntimeControls>>,
    meters: &Rc<MeterStats>,
) -> Result<(), JsValue> {
    let input_buffer = event.input_buffer()?;
    let output_buffer = event.output_buffer()?;
    let input = input_buffer.get_channel_data(0)?;
    let mut left = Vec::with_capacity(input.len());
    let mut right = Vec::with_capacity(input.len());

    let controls = controls.borrow();
    let mut runtime = runtime.borrow_mut();
    for sample in input {
        let (left_sample, right_sample) = runtime.process(sample, &controls, meters);
        left.push(left_sample);
        right.push(right_sample);
    }

    output_buffer.copy_to_channel(&left, 0)?;
    output_buffer.copy_to_channel(&right, 1)?;
    Ok(())
}

fn supported_script_processor_size(period_size: u32) -> u32 {
    match period_size {
        0..=1024 => 1024,
        1025..=2048 => 2048,
        2049..=4096 => 4096,
        4097..=8192 => 8192,
        _ => 16_384,
    }
}

fn raw_audio_constraints() -> Result<JsValue, JsValue> {
    let audio = Object::new();
    Reflect::set(
        &audio,
        &JsValue::from_str("echoCancellation"),
        &JsValue::FALSE,
    )?;
    Reflect::set(
        &audio,
        &JsValue::from_str("noiseSuppression"),
        &JsValue::FALSE,
    )?;
    Reflect::set(
        &audio,
        &JsValue::from_str("autoGainControl"),
        &JsValue::FALSE,
    )?;
    Reflect::set(
        &audio,
        &JsValue::from_str("channelCount"),
        &JsValue::from_f64(1.0),
    )?;
    Ok(audio.into())
}

fn js_error_message(error: JsValue) -> String {
    if let Some(message) = error.as_string() {
        return message;
    }
    js_sys::Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|message| message.as_string())
        .unwrap_or_else(|| "WebAudio could not start".to_string())
}

struct WebAudioRuntime {
    chain: SignalChain,
    speaker: SpeakerStage,
    device_controls: Vec<DeviceSlotControls>,
}

impl WebAudioRuntime {
    fn new(sample_rate: f32, amp_model: &str, app_profile: AppProfile) -> Self {
        let mut config = SignalChainConfig::amp_only(amp_model);
        for slot in app_profile.runtime_devices {
            let device = if slot.bypassed {
                DeviceSlotConfig::bypassed(slot.config)
            } else {
                DeviceSlotConfig::active(slot.config)
            };
            match slot.section {
                RuntimeDeviceSection::PreAmp => config.pre_amp.push(device),
                RuntimeDeviceSection::PostAmp => config.post_amp.push(device),
            }
        }

        Self {
            chain: SignalChain::new(sample_rate, config),
            speaker: SpeakerStage::from_embedded_ir(sample_rate as u32)
                .unwrap_or_else(|_| SpeakerStage::bypassed()),
            device_controls: Vec::with_capacity(2),
        }
    }

    fn process(
        &mut self,
        input: f32,
        controls: &RuntimeControls,
        meters: &MeterStats,
    ) -> (f32, f32) {
        let input = input * controls.input_gain;
        meters.record_input(input);
        controls.load_device_controls_into(&mut self.device_controls);
        let chain_output = self.chain.process_with_amp_enabled(
            input,
            SignalChainControls {
                amp: controls.amp,
                devices: &self.device_controls,
            },
            controls.amp_enabled,
        );
        let wet = self.speaker.process(chain_output, controls.cab_mix > 0.0);
        let output = (chain_output * (1.0 - controls.cab_mix) + wet * controls.cab_mix)
            * controls.output_gain;
        let output = protect_dac(output);
        meters.record_output(output, output);
        (output, output)
    }
}

fn protect_dac(sample: f32) -> f32 {
    sample.clamp(-0.98, 0.98)
}

#[derive(Clone)]
struct RuntimeControls {
    input_gain: f32,
    output_gain: f32,
    amp: AmpControls,
    amp_enabled: bool,
    minotaur_bypassed: bool,
    minotaur_gain: f32,
    minotaur_treble: f32,
    minotaur_output: f32,
    springfield_bypassed: bool,
    springfield_dwell: f32,
    springfield_tone: f32,
    springfield_mix: f32,
    runtime_devices: &'static [greybound_ui::RuntimeDeviceSlot],
    cab_mix: f32,
}

impl RuntimeControls {
    fn from_ui(ui: &GreyboundUi) -> Self {
        let mut controls = Self {
            input_gain: greybound_ui::normalized_gain(ui.input_gain, -24.0, 24.0),
            output_gain: greybound_ui::normalized_gain(ui.output_gain, -24.0, 6.0),
            amp: AmpControls {
                volume: ui.amp.gain,
                bass: ui.amp.bass,
                treble: ui.amp.treble,
                cut: ui.amp.cut,
                output: 0.58,
                drive: ui.amp.drive,
                presence: ui.amp.presence,
                sag: ui.amp.sag,
            },
            amp_enabled: !ui.amp.bypassed,
            minotaur_bypassed: false,
            minotaur_gain: 0.0,
            minotaur_treble: 0.0,
            minotaur_output: 0.0,
            springfield_bypassed: true,
            springfield_dwell: 0.48,
            springfield_tone: 0.58,
            springfield_mix: 0.26,
            runtime_devices: ui.app_profile.runtime_devices,
            cab_mix: if !ui.cab.bypassed {
                ui.cab.master.clamp(0.0, 1.0)
            } else {
                0.0
            },
        };

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Minotaur)
        {
            controls.minotaur_bypassed = device.bypassed;
            controls.minotaur_gain = device.gain;
            controls.minotaur_treble = device.treble;
            controls.minotaur_output = device.master;
        }

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Springfield)
        {
            controls.springfield_bypassed = device.bypassed;
            controls.springfield_dwell = device.gain;
            controls.springfield_tone = device.treble;
            controls.springfield_mix = device.master;
        }

        controls
    }

    fn load_device_controls_into(&self, target: &mut Vec<DeviceSlotControls>) {
        target.clear();
        for slot in self.runtime_devices {
            match slot.config {
                DeviceConfig::Minotaur => target.push(DeviceSlotControls {
                    bypassed: self.minotaur_bypassed,
                    controls: DeviceControls::Minotaur(MinotaurControls {
                        gain: self.minotaur_gain,
                        treble: self.minotaur_treble,
                        output: self.minotaur_output,
                    }),
                }),
                DeviceConfig::Springfield => target.push(DeviceSlotControls {
                    bypassed: self.springfield_bypassed,
                    controls: DeviceControls::Springfield(SpringfieldControls {
                        dwell: self.springfield_dwell,
                        tone: self.springfield_tone,
                        mix: self.springfield_mix,
                    }),
                }),
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct MeterStats {
    input_peak: Cell<f32>,
    output_left_peak: Cell<f32>,
    output_right_peak: Cell<f32>,
}

impl MeterStats {
    fn record_input(&self, sample: f32) {
        self.input_peak.set(self.input_peak.get().max(sample.abs()));
    }

    fn record_output(&self, left: f32, right: f32) {
        self.output_left_peak
            .set(self.output_left_peak.get().max(left.abs()));
        self.output_right_peak
            .set(self.output_right_peak.get().max(right.abs()));
    }

    fn snapshot_levels(&self) -> (f32, f32, f32) {
        let input = self.take_smoothed(&self.input_peak);
        let left = self.take_smoothed(&self.output_left_peak);
        let right = self.take_smoothed(&self.output_right_peak);
        (input, left, right)
    }

    fn take_smoothed(&self, cell: &Cell<f32>) -> f32 {
        let peak = cell.get().clamp(0.0, 4.0);
        cell.set(peak * 0.72);
        (peak * 1.25).clamp(0.0, 1.0)
    }
}
