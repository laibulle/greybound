use greybound::{
    ir::SpeakerStage, AmpControls, AuralithControls, DeviceControls, DeviceSlotConfig,
    DeviceSlotControls, MinotaurControls, SignalChain, SignalChainConfig, SignalChainControls,
    SpringfieldControls,
};
use greybound_plugin_ui::{PluginIcedApp, PluginUiConfig};
use greybound_ui::{
    normalized_gain, preload_render_assets, AppProfile, DeviceModel, GreyboundUi, Message,
    RuntimeDeviceSection, DESIGN_HEIGHT, DESIGN_WIDTH,
};
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

const NOX30_OUTPUT_GAIN: f32 = 0.58;
const OUTPUT_MIN_DB: f32 = -24.0;
const OUTPUT_MAX_DB: f32 = 6.0;
const METER_RMS_SCALE: f64 = 1_000_000_000.0;

pub struct GreyboundPlugin {
    params: Arc<GreyboundParams>,
    meters: Arc<PluginMeters>,
    channels: Vec<SignalChain>,
    speakers: Vec<SpeakerStage>,
    chain_config: SignalChainConfig,
    sample_rate: Option<f32>,
}

#[derive(Params)]
struct GreyboundParams {
    #[id = "gain"]
    gain: FloatParam,
    #[id = "bass"]
    bass: FloatParam,
    #[id = "cut"]
    cut: FloatParam,
    #[id = "tone"]
    tone: FloatParam,
    #[id = "sag"]
    sag: FloatParam,
    #[id = "master"]
    master: FloatParam,
    #[id = "speaker_ir"]
    speaker_ir: BoolParam,
    #[id = "overdrive"]
    overdrive: BoolParam,
    #[id = "overdrive_gain"]
    overdrive_gain: FloatParam,
    #[id = "overdrive_treble"]
    overdrive_treble: FloatParam,
    #[id = "overdrive_output"]
    overdrive_output: FloatParam,
    #[id = "auralith"]
    auralith: BoolParam,
    #[id = "auralith_decay"]
    auralith_decay: FloatParam,
    #[id = "auralith_size"]
    auralith_size: FloatParam,
    #[id = "auralith_texture"]
    auralith_texture: FloatParam,
    #[id = "auralith_tone"]
    auralith_tone: FloatParam,
    #[id = "auralith_low_cut"]
    auralith_low_cut: FloatParam,
    #[id = "auralith_mix"]
    auralith_mix: FloatParam,
}

impl Default for GreyboundPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(GreyboundParams::default()),
            meters: Arc::new(PluginMeters::default()),
            channels: Vec::new(),
            speakers: Vec::new(),
            chain_config: plugin_signal_chain_config(AppProfile::greybound_free(), "nox30"),
            sample_rate: None,
        }
    }
}

impl Default for GreyboundParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Top Boost Volume",
                0.58,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            bass: FloatParam::new("Bass", 0.54, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            cut: FloatParam::new("Cut", 0.43, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            tone: FloatParam::new("Treble", 0.59, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            sag: FloatParam::new("Sag", 0.45, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            master: FloatParam::new(
                "Output Trim",
                0.58,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(output_trim_to_string())
            .with_string_to_value(output_trim_from_string()),
            speaker_ir: BoolParam::new("Speaker IR", true),
            overdrive: BoolParam::new("Minotaur Overdrive", true),
            overdrive_gain: FloatParam::new(
                "Minotaur Gain",
                MinotaurControls::default().gain,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            overdrive_treble: FloatParam::new(
                "Minotaur Treble",
                MinotaurControls::default().treble,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            overdrive_output: FloatParam::new(
                "Minotaur Output",
                MinotaurControls::default().output,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith: BoolParam::new("Auralith Reverb", true),
            auralith_decay: FloatParam::new(
                "Auralith Decay",
                AuralithControls::default().decay,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith_size: FloatParam::new(
                "Auralith Size",
                AuralithControls::default().size,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith_texture: FloatParam::new(
                "Auralith Texture",
                AuralithControls::default().texture,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith_tone: FloatParam::new(
                "Auralith Tone",
                AuralithControls::default().tone,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith_low_cut: FloatParam::new(
                "Auralith Low Cut",
                AuralithControls::default().low_cut,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            auralith_mix: FloatParam::new(
                "Auralith Mix",
                AuralithControls::default().mix,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Plugin for GreyboundPlugin {
    const NAME: &'static str = "Greybound";
    const VENDOR: &'static str = "Greybound";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "dev@localhost";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let meters = self.meters.clone();

        greybound_plugin_ui::create_iced_editor(
            PluginUiConfig {
                title: "Greybound",
                width: DESIGN_WIDTH as u32,
                height: DESIGN_HEIGHT as u32,
                background: iced::Color::from_rgb(0.72, 0.78, 0.91),
            },
            move |context| GreyboundPluginApp::new(params.clone(), meters.clone(), context),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let chain_config = self.chain_config.clone();
        self.sample_rate = Some(buffer_config.sample_rate);
        self.channels = build_signal_chains(buffer_config.sample_rate, 1, &chain_config);
        let sample_rate = buffer_config.sample_rate as u32;
        self.speakers = (0..1)
            .map(|_| {
                SpeakerStage::from_embedded_ir(sample_rate)
                    .unwrap_or_else(|_| SpeakerStage::bypassed())
            })
            .collect();
        context.set_latency_samples(
            (greybound::amp::AMP_LATENCY + greybound::ir::CONVOLUTION_LATENCY) as u32,
        );
        true
    }

    fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
        for speaker in &mut self.speakers {
            speaker.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for mut channel_samples in buffer.iter_samples() {
            let input = channel_samples
                .get_mut(0)
                .map(|sample| *sample)
                .unwrap_or(0.0);
            let controls = AmpControls {
                volume: self.params.gain.smoothed.next(),
                bass: self.params.bass.smoothed.next(),
                cut: self.params.cut.smoothed.next(),
                treble: self.params.tone.smoothed.next(),
                output: NOX30_OUTPUT_GAIN,
                drive: 0.0,
                presence: 0.0,
                sag: self.params.sag.smoothed.next(),
            };
            let overdrive_controls = MinotaurControls {
                gain: self.params.overdrive_gain.smoothed.next(),
                treble: self.params.overdrive_treble.smoothed.next(),
                output: self.params.overdrive_output.smoothed.next(),
            };
            let auralith_controls = AuralithControls {
                decay: self.params.auralith_decay.smoothed.next(),
                size: self.params.auralith_size.smoothed.next(),
                texture: self.params.auralith_texture.smoothed.next(),
                tone: self.params.auralith_tone.smoothed.next(),
                low_cut: self.params.auralith_low_cut.smoothed.next(),
                mix: self.params.auralith_mix.smoothed.next(),
            };
            let device_controls = [
                DeviceSlotControls {
                    bypassed: !self.params.overdrive.value(),
                    controls: DeviceControls::Minotaur(overdrive_controls),
                },
                DeviceSlotControls {
                    bypassed: !self.params.auralith.value(),
                    controls: DeviceControls::Auralith(auralith_controls),
                },
                DeviceSlotControls {
                    bypassed: true,
                    controls: DeviceControls::Springfield(SpringfieldControls::default()),
                },
            ];
            let chain_controls = SignalChainControls {
                amp: controls,
                devices: &device_controls,
            };
            let output_gain = normalized_gain(
                self.params.master.smoothed.next(),
                OUTPUT_MIN_DB,
                OUTPUT_MAX_DB,
            );
            self.meters.record_input(input);
            let amp_output = self.channels[0].process(input, chain_controls);
            let cabbed = self.speakers[0].process(amp_output, self.params.speaker_ir.value());
            let output = cabbed * output_gain;
            self.meters.record_output(output, output);

            for sample in channel_samples.iter_mut() {
                *sample = output;
            }
        }

        ProcessStatus::Normal
    }
}

#[derive(Default)]
struct PluginMeters {
    input_sum_squares: AtomicU64,
    input_count: AtomicU64,
    output_left_sum_squares: AtomicU64,
    output_right_sum_squares: AtomicU64,
    output_left_count: AtomicU64,
    output_right_count: AtomicU64,
}

impl PluginMeters {
    fn record_input(&self, sample: f32) {
        self.input_sum_squares
            .fetch_add(meter_square(sample), Ordering::Relaxed);
        self.input_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_output(&self, left: f32, right: f32) {
        self.output_left_sum_squares
            .fetch_add(meter_square(left), Ordering::Relaxed);
        self.output_right_sum_squares
            .fetch_add(meter_square(right), Ordering::Relaxed);
        self.output_left_count.fetch_add(1, Ordering::Relaxed);
        self.output_right_count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot_levels(&self) -> (f32, f32, f32) {
        (
            meter_from_accumulators(&self.input_sum_squares, &self.input_count),
            meter_from_accumulators(&self.output_left_sum_squares, &self.output_left_count),
            meter_from_accumulators(&self.output_right_sum_squares, &self.output_right_count),
        )
    }
}

fn meter_square(sample: f32) -> u64 {
    let magnitude = sample.abs() as f64;
    (magnitude * magnitude * METER_RMS_SCALE).round() as u64
}

fn meter_from_accumulators(sum_squares: &AtomicU64, count: &AtomicU64) -> f32 {
    let sum = sum_squares.swap(0, Ordering::Relaxed) as f64 / METER_RMS_SCALE;
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

fn plugin_signal_chain_config(app_profile: AppProfile, amp_model: &str) -> SignalChainConfig {
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
    config
}

fn build_signal_chains(
    sample_rate: f32,
    channels: usize,
    config: &SignalChainConfig,
) -> Vec<SignalChain> {
    (0..channels)
        .map(|_| SignalChain::new(sample_rate, config.clone()))
        .collect()
}

fn output_trim_to_string() -> Arc<dyn Fn(f32) -> String + Send + Sync> {
    Arc::new(|value| format!("{:.1}", output_trim_db(value)))
}

fn output_trim_from_string() -> Arc<dyn Fn(&str) -> Option<f32> + Send + Sync> {
    Arc::new(|string| {
        let db: f32 = string
            .trim_end_matches(&[' ', 'd', 'D', 'b', 'B'])
            .parse()
            .ok()?;
        Some(((db - OUTPUT_MIN_DB) / (OUTPUT_MAX_DB - OUTPUT_MIN_DB)).clamp(0.0, 1.0))
    })
}

fn output_trim_db(value: f32) -> f32 {
    OUTPUT_MIN_DB + value.clamp(0.0, 1.0) * (OUTPUT_MAX_DB - OUTPUT_MIN_DB)
}

struct GreyboundPluginApp {
    ui: GreyboundUi,
    params: Arc<GreyboundParams>,
    meters: Arc<PluginMeters>,
    context: Arc<dyn GuiContext>,
}

impl GreyboundPluginApp {
    fn new(
        params: Arc<GreyboundParams>,
        meters: Arc<PluginMeters>,
        context: Arc<dyn GuiContext>,
    ) -> Self {
        preload_render_assets();
        let mut ui = GreyboundUi::new(AppProfile::greybound_free());
        ui.update(Message::WindowResized {
            width: DESIGN_WIDTH as u32,
            height: DESIGN_HEIGHT as u32,
        });
        let app = Self {
            ui,
            params,
            meters,
            context,
        };
        app.sync_params_from_ui();
        app
    }

    fn sync_params_from_ui(&self) {
        let amp = &self.ui.amp;
        unsafe {
            self.context
                .raw_set_parameter_normalized(self.params.gain.as_ptr(), amp.gain);
            self.context
                .raw_set_parameter_normalized(self.params.bass.as_ptr(), amp.bass);
            self.context
                .raw_set_parameter_normalized(self.params.cut.as_ptr(), amp.cut);
            self.context
                .raw_set_parameter_normalized(self.params.tone.as_ptr(), amp.treble);
            self.context
                .raw_set_parameter_normalized(self.params.sag.as_ptr(), amp.sag);
            self.context
                .raw_set_parameter_normalized(self.params.master.as_ptr(), self.ui.output_gain);
            self.context.raw_set_parameter_normalized(
                self.params.speaker_ir.as_ptr(),
                if self.ui.cab.bypassed { 0.0 } else { 1.0 },
            );
        }

        if let Some(minotaur) = self
            .ui
            .devices
            .iter()
            .find(|device| device.model == DeviceModel::Minotaur)
        {
            unsafe {
                self.context.raw_set_parameter_normalized(
                    self.params.overdrive.as_ptr(),
                    if minotaur.bypassed { 0.0 } else { 1.0 },
                );
                self.context.raw_set_parameter_normalized(
                    self.params.overdrive_gain.as_ptr(),
                    minotaur.gain,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.overdrive_treble.as_ptr(),
                    minotaur.treble,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.overdrive_output.as_ptr(),
                    minotaur.master,
                );
            }
        }

        if let Some(auralith) = self
            .ui
            .devices
            .iter()
            .find(|device| device.model == DeviceModel::ReverbFx)
        {
            unsafe {
                self.context.raw_set_parameter_normalized(
                    self.params.auralith.as_ptr(),
                    if auralith.bypassed { 0.0 } else { 1.0 },
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_decay.as_ptr(),
                    auralith.gain,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_size.as_ptr(),
                    auralith.bass,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_texture.as_ptr(),
                    auralith.cut,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_tone.as_ptr(),
                    auralith.treble,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_low_cut.as_ptr(),
                    auralith.presence,
                );
                self.context.raw_set_parameter_normalized(
                    self.params.auralith_mix.as_ptr(),
                    auralith.master,
                );
            }
        }
    }
}

impl PluginIcedApp for GreyboundPluginApp {
    type Message = Message;

    fn on_frame(&mut self) {
        let (input, output_left, output_right) = self.meters.snapshot_levels();
        self.ui.update(Message::MeterLevelsChanged {
            input,
            output_left,
            output_right,
        });
    }

    fn update(&mut self, message: Self::Message) {
        let should_sync = !matches!(
            message,
            Message::MeterProbeTick
                | Message::TunerProbeTick(_)
                | Message::MeterLevelsChanged { .. }
                | Message::AudioDevicesChanged { .. }
                | Message::AudioStatusChanged(_)
        );
        self.ui.update(message);
        if should_sync {
            self.sync_params_from_ui();
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        self.ui.view()
    }
}

impl ClapPlugin for GreyboundPlugin {
    const CLAP_ID: &'static str = "com.greybound.graybox-amp";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Nox30 circuit-informed guitar amp");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Distortion,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for GreyboundPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"GreyboundGrayAmp";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nih_export_clap!(GreyboundPlugin);
nih_export_vst3!(GreyboundPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_uses_free_runtime_devices() {
        let config = plugin_signal_chain_config(AppProfile::greybound_free(), "nox30");

        assert_eq!(config.pre_amp.len(), 1);
        assert_eq!(config.pre_amp[0].device, greybound::DeviceConfig::Minotaur);
        assert!(!config.pre_amp[0].bypassed);
        assert!(config.fx_loop.is_empty());
        assert_eq!(config.post_amp.len(), 2);
        assert_eq!(config.post_amp[0].device, greybound::DeviceConfig::Auralith);
        assert!(!config.post_amp[0].bypassed);
        assert_eq!(
            config.post_amp[1].device,
            greybound::DeviceConfig::Springfield
        );
        assert!(config.post_amp[1].bypassed);
    }

    #[test]
    fn plugin_param_defaults_match_free_ui_snapshot() {
        let ui = GreyboundUi::default();
        let snapshot = ui.runtime_audio_snapshot();
        let params = GreyboundParams::default();

        assert_eq!(params.gain.value(), ui.amp.gain);
        assert_eq!(params.bass.value(), ui.amp.bass);
        assert_eq!(params.cut.value(), ui.amp.cut);
        assert_eq!(params.tone.value(), ui.amp.treble);
        assert_eq!(params.sag.value(), ui.amp.sag);
        assert_eq!(params.master.value(), ui.output_gain);
        assert_eq!(params.speaker_ir.value(), snapshot.cab_mix > 0.0);
        assert!(params.overdrive.value());
        assert!(params.auralith.value());
    }

    #[test]
    fn active_auralith_produces_tail_without_exploding() {
        let config = plugin_signal_chain_config(AppProfile::greybound_free(), "nox30");
        let mut chain = SignalChain::new(48_000.0, config);
        let controls = AmpControls {
            volume: 0.58,
            bass: 0.54,
            treble: 0.59,
            cut: 0.43,
            output: NOX30_OUTPUT_GAIN,
            drive: 0.0,
            presence: 0.0,
            sag: 0.45,
        };
        let devices = [
            DeviceSlotControls {
                bypassed: false,
                controls: DeviceControls::Minotaur(MinotaurControls::default()),
            },
            DeviceSlotControls {
                bypassed: false,
                controls: DeviceControls::Auralith(AuralithControls {
                    decay: 0.52,
                    size: 0.55,
                    texture: 0.68,
                    tone: 0.55,
                    low_cut: 0.32,
                    mix: 0.75,
                }),
            },
            DeviceSlotControls {
                bypassed: true,
                controls: DeviceControls::Springfield(SpringfieldControls::default()),
            },
        ];
        let chain_controls = SignalChainControls {
            amp: controls,
            devices: &devices,
        };
        let mut tail_energy = 0.0_f32;

        for sample_idx in 0..48_000 {
            let input = if sample_idx < 128 {
                (sample_idx as f32 * 0.17).sin() * 0.12
            } else {
                0.0
            };
            let output = chain.process(input, chain_controls);
            assert!(output.is_finite());
            assert!(output.abs() < 16.0, "unstable Auralith output: {output}");
            if sample_idx > 12_000 {
                tail_energy += output.abs();
            }
        }

        assert!(tail_energy > 0.01, "Auralith should produce a reverb tail");
    }

    #[test]
    fn plugin_meters_snapshot_and_reset_rms_levels() {
        let meters = PluginMeters::default();

        meters.record_input(0.5);
        meters.record_input(-0.5);
        meters.record_output(0.25, -0.75);
        let (input, left, right) = meters.snapshot_levels();

        assert!(input > 0.0);
        assert!(left > 0.0);
        assert!(right > left);
        assert_eq!(meters.snapshot_levels(), (0.0, 0.0, 0.0));
    }
}
