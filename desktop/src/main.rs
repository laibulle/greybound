use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, Stream, StreamConfig, SupportedStreamConfigRange,
};
use greybound::ir::SpeakerStage;
use greybound::{
    AmpControls, DeviceConfig, DeviceControls, DeviceSlotConfig, DeviceSlotControls,
    MinotaurControls, SignalChain, SignalChainConfig, SignalChainControls, SpringfieldControls,
};
use greybound_ui::{GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use rtrb::{Consumer, RingBuffer};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

const RMS_SCALE: f64 = 1_000_000_000.0;

fn main() -> iced::Result {
    Desktop::run(Settings {
        window: iced::window::Settings {
            size: (DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct Desktop {
    ui: GreyboundUi,
    audio: Option<LiveAudioEngine>,
    audio_error: Option<String>,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut ui = GreyboundUi::default();
        refresh_audio_devices(&mut ui);
        let audio = LiveAudioEngine::start(&ui);
        let (audio, audio_error) = match audio {
            Ok(engine) => {
                ui.update(Message::AudioStatusChanged(engine.status()));
                (Some(engine), None)
            }
            Err(error) => {
                let error = error.to_string();
                ui.update(Message::AudioStatusChanged(error.clone()));
                (None, Some(error))
            }
        };
        (
            Desktop {
                ui,
                audio,
                audio_error,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Message::MeterProbeTick(_) = message {
            if self.audio.is_none() {
                match LiveAudioEngine::start(&self.ui) {
                    Ok(engine) => {
                        self.ui.update(Message::AudioStatusChanged(engine.status()));
                        self.audio = Some(engine);
                        self.audio_error = None;
                    }
                    Err(error) => {
                        let error = error.to_string();
                        self.ui.update(Message::AudioStatusChanged(error.clone()));
                        self.audio_error = Some(error);
                    }
                }
            }

            if let Some(audio) = &self.audio {
                let (input, output) = audio.meter_levels();
                self.ui
                    .update(Message::MeterLevelsChanged { input, output });
            }
            return Command::none();
        }

        let restart_audio = matches!(
            &message,
            Message::AudioInputSelected(_)
                | Message::AudioOutputSelected(_)
                | Message::AudioSampleRateSelected(_)
                | Message::AudioBufferSizeSelected(_)
        );
        self.ui.update(message);
        if restart_audio {
            self.audio = None;
            match LiveAudioEngine::start(&self.ui) {
                Ok(engine) => {
                    self.ui.update(Message::AudioStatusChanged(engine.status()));
                    self.audio = Some(engine);
                    self.audio_error = None;
                }
                Err(error) => {
                    let error = error.to_string();
                    self.ui.update(Message::AudioStatusChanged(error.clone()));
                    self.audio_error = Some(error);
                }
            }
        }
        if let Some(audio) = &self.audio {
            audio.controls.store_from_ui(&self.ui);
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::subscription::events_with(|event, _status| match event {
                iced::Event::Window(iced::window::Event::Resized { width, height }) => {
                    Some(Message::WindowResized { width, height })
                }
                _ => None,
            }),
            iced::time::every(Duration::from_millis(33)).map(Message::MeterProbeTick),
        ])
    }
}

struct LiveAudioEngine {
    _input_stream: Stream,
    _output_stream: Stream,
    controls: SharedRuntimeControls,
    meters: Arc<MeterStats>,
    input_device: String,
    output_device: String,
    sample_rate: u32,
    period_size: u32,
}

impl LiveAudioEngine {
    fn start(ui: &GreyboundUi) -> Result<Self> {
        let host = cpal::default_host();
        let sample_rate = ui.audio_settings.sample_rate;
        let period_size = ui.audio_settings.period_size;
        let input_device =
            selected_or_default_input(&host, ui.audio_settings.selected_input.as_deref())?;
        let output_device =
            selected_or_default_output(&host, ui.audio_settings.selected_output.as_deref())?;
        let input_device_name = device_name(&input_device);
        let output_device_name = device_name(&output_device);
        let output_range = select_config(
            output_device.supported_output_configs()?,
            sample_rate,
            period_size,
            "output",
        )?;
        let input_range = select_config(
            input_device.supported_input_configs()?,
            sample_rate,
            period_size,
            "input",
        )?;
        let output_config = stream_config(&output_range, sample_rate, period_size);
        let input_config = stream_config(&input_range, sample_rate, period_size);
        let input_channels = input_config.channels as usize;
        let output_channels = output_config.channels as usize;
        let (mut producer, consumer) = RingBuffer::<f32>::new(period_size as usize * 16);
        let meters = Arc::new(MeterStats::default());

        let input_meters = meters.clone();
        let input_name = input_device_name.clone();
        let input_stream = input_device.build_input_stream(
            &input_config,
            move |data: &[f32], _| {
                for frame in data.chunks_exact(input_channels) {
                    let sample = frame[0];
                    input_meters.record_input(sample);
                    let _ = producer.push(sample);
                }
            },
            move |error| eprintln!("Greybound input stream error on {input_name}: {error}"),
            None,
        )?;

        let controls = SharedRuntimeControls::new(ui);
        let output_controls = controls.clone();
        let output_meters = meters.clone();
        let output_name = output_device_name.clone();
        let mut runtime = AudioRuntime::new(sample_rate as f32, consumer)?;
        let output_stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _| {
                for frame in data.chunks_exact_mut(output_channels) {
                    let output = runtime.process(&output_controls, &output_meters);
                    frame.fill(0.0);
                    frame[0] = output;
                    if output_channels > 1 {
                        frame[1] = output;
                    }
                }
            },
            move |error| eprintln!("Greybound output stream error on {output_name}: {error}"),
            None,
        )?;

        input_stream.play()?;
        output_stream.play()?;

        Ok(Self {
            _input_stream: input_stream,
            _output_stream: output_stream,
            controls,
            meters,
            input_device: input_device_name,
            output_device: output_device_name,
            sample_rate,
            period_size,
        })
    }

    fn meter_levels(&self) -> (f32, f32) {
        self.meters.snapshot_levels()
    }

    fn status(&self) -> String {
        format!(
            "Running: {} -> {}, {} Hz / {} samples",
            self.input_device, self.output_device, self.sample_rate, self.period_size
        )
    }
}

struct AudioRuntime {
    input: Consumer<f32>,
    chain: SignalChain,
    speaker: SpeakerStage,
    device_controls: Vec<DeviceSlotControls>,
}

impl AudioRuntime {
    fn new(sample_rate: f32, input: Consumer<f32>) -> Result<Self> {
        let mut config = SignalChainConfig::amp_only("nox30");
        config
            .pre_amp
            .push(DeviceSlotConfig::active(DeviceConfig::Minotaur));
        config
            .post_amp
            .push(DeviceSlotConfig::active(DeviceConfig::Springfield));

        Ok(Self {
            input,
            chain: SignalChain::new(sample_rate, config),
            speaker: SpeakerStage::from_embedded_ir(sample_rate as u32)?,
            device_controls: Vec::with_capacity(2),
        })
    }

    fn process(&mut self, controls: &SharedRuntimeControls, meters: &MeterStats) -> f32 {
        let input = self.input.pop().unwrap_or(0.0);
        controls.load_device_controls_into(&mut self.device_controls);
        let output = self.speaker.process(
            self.chain.process(
                input,
                SignalChainControls {
                    amp: controls.load_amp_controls(),
                    devices: &self.device_controls,
                },
            ),
            controls.cab_enabled(),
        );
        meters.record_output(output);
        output
    }
}

#[derive(Clone)]
struct SharedRuntimeControls {
    amp_volume: Arc<AtomicU32>,
    amp_bass: Arc<AtomicU32>,
    amp_treble: Arc<AtomicU32>,
    amp_cut: Arc<AtomicU32>,
    amp_sag: Arc<AtomicU32>,
    minotaur_bypassed: Arc<AtomicBool>,
    minotaur_gain: Arc<AtomicU32>,
    minotaur_treble: Arc<AtomicU32>,
    minotaur_output: Arc<AtomicU32>,
    springfield_bypassed: Arc<AtomicBool>,
    springfield_dwell: Arc<AtomicU32>,
    springfield_tone: Arc<AtomicU32>,
    springfield_mix: Arc<AtomicU32>,
    cab_enabled: Arc<AtomicBool>,
}

impl SharedRuntimeControls {
    fn new(ui: &GreyboundUi) -> Self {
        let controls = Self {
            amp_volume: atomic_f32(0.0),
            amp_bass: atomic_f32(0.0),
            amp_treble: atomic_f32(0.0),
            amp_cut: atomic_f32(0.0),
            amp_sag: atomic_f32(0.0),
            minotaur_bypassed: Arc::new(AtomicBool::new(false)),
            minotaur_gain: atomic_f32(0.0),
            minotaur_treble: atomic_f32(0.0),
            minotaur_output: atomic_f32(0.0),
            springfield_bypassed: Arc::new(AtomicBool::new(false)),
            springfield_dwell: atomic_f32(0.0),
            springfield_tone: atomic_f32(0.0),
            springfield_mix: atomic_f32(0.0),
            cab_enabled: Arc::new(AtomicBool::new(true)),
        };
        controls.store_from_ui(ui);
        controls
    }

    fn store_from_ui(&self, ui: &GreyboundUi) {
        store_f32(&self.amp_volume, ui.amp.gain);
        store_f32(&self.amp_bass, ui.amp.bass);
        store_f32(&self.amp_treble, ui.amp.treble);
        store_f32(&self.amp_cut, ui.amp.cut);
        store_f32(&self.amp_sag, ui.amp.master);
        self.cab_enabled
            .store(!ui.cab.bypassed && ui.cab.master > 0.0, Ordering::Relaxed);

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Minotaur)
        {
            self.minotaur_bypassed
                .store(device.bypassed, Ordering::Relaxed);
            store_f32(&self.minotaur_gain, device.gain);
            store_f32(&self.minotaur_treble, device.treble);
            store_f32(&self.minotaur_output, device.master);
        }

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Springfield)
        {
            self.springfield_bypassed
                .store(device.bypassed, Ordering::Relaxed);
            store_f32(&self.springfield_dwell, device.gain);
            store_f32(&self.springfield_tone, device.treble);
            store_f32(&self.springfield_mix, device.master);
        }
    }

    fn load_amp_controls(&self) -> AmpControls {
        AmpControls {
            volume: load_f32(&self.amp_volume),
            bass: load_f32(&self.amp_bass),
            treble: load_f32(&self.amp_treble),
            cut: load_f32(&self.amp_cut),
            output: 0.58,
            drive: 0.0,
            presence: 0.0,
            sag: load_f32(&self.amp_sag),
        }
    }

    fn load_device_controls_into(&self, target: &mut Vec<DeviceSlotControls>) {
        target.clear();
        target.push(DeviceSlotControls {
            bypassed: self.minotaur_bypassed.load(Ordering::Relaxed),
            controls: DeviceControls::Minotaur(MinotaurControls {
                gain: load_f32(&self.minotaur_gain),
                treble: load_f32(&self.minotaur_treble),
                output: load_f32(&self.minotaur_output),
            }),
        });
        target.push(DeviceSlotControls {
            bypassed: self.springfield_bypassed.load(Ordering::Relaxed),
            controls: DeviceControls::Springfield(SpringfieldControls {
                dwell: load_f32(&self.springfield_dwell),
                tone: load_f32(&self.springfield_tone),
                mix: load_f32(&self.springfield_mix),
            }),
        });
    }

    fn cab_enabled(&self) -> bool {
        self.cab_enabled.load(Ordering::Relaxed)
    }
}

#[derive(Default)]
struct MeterStats {
    input_sum_squares: AtomicU64,
    input_count: AtomicU64,
    output_sum_squares: AtomicU64,
    output_count: AtomicU64,
}

impl MeterStats {
    fn record_input(&self, sample: f32) {
        self.input_sum_squares
            .fetch_add(square(sample), Ordering::Relaxed);
        self.input_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_output(&self, sample: f32) {
        self.output_sum_squares
            .fetch_add(square(sample), Ordering::Relaxed);
        self.output_count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot_levels(&self) -> (f32, f32) {
        let input_level = meter_from_accumulators(&self.input_sum_squares, &self.input_count);
        let output_level = meter_from_accumulators(&self.output_sum_squares, &self.output_count);
        (input_level, output_level)
    }
}

fn select_config(
    configs: impl Iterator<Item = SupportedStreamConfigRange>,
    sample_rate: u32,
    period_size: u32,
    direction: &str,
) -> Result<SupportedStreamConfigRange> {
    let rate = SampleRate(sample_rate);
    configs
        .filter(|config| config.sample_format() == SampleFormat::F32)
        .find(|config| {
            (config.min_sample_rate()..=config.max_sample_rate()).contains(&rate)
                && match config.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => {
                        (*min..=*max).contains(&period_size)
                    }
                    cpal::SupportedBufferSize::Unknown => true,
                }
        })
        .with_context(|| {
            format!(
                "no f32 {direction} configuration supports {sample_rate} Hz / {period_size} samples"
            )
        })
}

fn stream_config(
    range: &SupportedStreamConfigRange,
    sample_rate: u32,
    period_size: u32,
) -> StreamConfig {
    StreamConfig {
        channels: range.channels(),
        sample_rate: SampleRate(sample_rate),
        buffer_size: BufferSize::Fixed(period_size),
    }
}

fn device_name(device: &Device) -> String {
    device
        .name()
        .unwrap_or_else(|_| "unknown device".to_string())
}

fn refresh_audio_devices(ui: &mut GreyboundUi) {
    let host = cpal::default_host();
    let inputs = device_names(host.input_devices());
    let outputs = device_names(host.output_devices());
    let selected_input = ui
        .audio_settings
        .selected_input
        .clone()
        .filter(|selected| inputs.contains(selected))
        .or_else(|| {
            host.default_input_device()
                .map(|device| device_name(&device))
        });
    let selected_output = ui
        .audio_settings
        .selected_output
        .clone()
        .filter(|selected| outputs.contains(selected))
        .or_else(|| {
            host.default_output_device()
                .map(|device| device_name(&device))
        });

    ui.update(Message::AudioDevicesChanged {
        inputs,
        outputs,
        selected_input,
        selected_output,
        status: "Audio devices loaded".to_string(),
    });
}

fn device_names<I>(devices: std::result::Result<I, cpal::DevicesError>) -> Vec<String>
where
    I: Iterator<Item = Device>,
{
    let mut names = devices
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_else(|_| Vec::new());
    names.sort();
    names.dedup();
    names
}

fn selected_or_default_input(host: &cpal::Host, selected: Option<&str>) -> Result<Device> {
    selected_device(host.input_devices()?, selected)
        .or_else(|| host.default_input_device())
        .context("missing default input device")
}

fn selected_or_default_output(host: &cpal::Host, selected: Option<&str>) -> Result<Device> {
    selected_device(host.output_devices()?, selected)
        .or_else(|| host.default_output_device())
        .context("missing default output device")
}

fn selected_device<I>(devices: I, selected: Option<&str>) -> Option<Device>
where
    I: Iterator<Item = Device>,
{
    let selected = selected?;
    devices
        .filter_map(|device| device.name().ok().map(|name| (device, name)))
        .find(|(_, name)| name == selected)
        .map(|(device, _)| device)
}

fn atomic_f32(value: f32) -> Arc<AtomicU32> {
    Arc::new(AtomicU32::new(value.to_bits()))
}

fn store_f32(slot: &AtomicU32, value: f32) {
    slot.store(value.to_bits(), Ordering::Relaxed);
}

fn load_f32(slot: &AtomicU32) -> f32 {
    f32::from_bits(slot.load(Ordering::Relaxed))
}

fn square(sample: f32) -> u64 {
    let magnitude = sample.abs() as f64;
    (magnitude * magnitude * RMS_SCALE).round() as u64
}

fn meter_from_accumulators(sum_squares: &AtomicU64, count: &AtomicU64) -> f32 {
    let sum = sum_squares.swap(0, Ordering::Relaxed) as f64 / RMS_SCALE;
    let count = count.swap(0, Ordering::Relaxed);
    if count == 0 {
        return 0.0;
    }
    rms_to_meter((sum / count as f64).sqrt() as f32)
}

fn rms_to_meter(rms: f32) -> f32 {
    let db = 20.0 * rms.max(0.000_001).log10();
    ((db + 54.0) / 54.0).clamp(0.0, 1.0)
}
