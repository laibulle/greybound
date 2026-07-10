use greybound::MinotaurControls;
use greybound::{
    ir::SpeakerStage, AmpControls, DeviceConfig, DeviceControls, DeviceSlotConfig,
    DeviceSlotControls, MuffinControls, SignalChain, SignalChainConfig, SignalChainControls,
};
use greybound_plugin_ui::{PluginIcedApp, PluginUiConfig};
use greybound_ui::{
    preload_render_assets, AppProfile, DeviceModel, GreyboundUi, Message, DESIGN_HEIGHT,
    DESIGN_WIDTH,
};
use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct GreyboundPlugin {
    params: Arc<GreyboundParams>,
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
    #[id = "master"]
    master: FloatParam,
    #[id = "speaker_ir"]
    speaker_ir: BoolParam,
    #[id = "fuzz"]
    fuzz: BoolParam,
    #[id = "fuzz_sustain"]
    fuzz_sustain: FloatParam,
    #[id = "fuzz_tone"]
    fuzz_tone: FloatParam,
    #[id = "fuzz_level"]
    fuzz_level: FloatParam,
    #[id = "overdrive"]
    overdrive: BoolParam,
    #[id = "overdrive_gain"]
    overdrive_gain: FloatParam,
    #[id = "overdrive_treble"]
    overdrive_treble: FloatParam,
    #[id = "overdrive_output"]
    overdrive_output: FloatParam,
}

impl Default for GreyboundPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(GreyboundParams::default()),
            channels: Vec::new(),
            speakers: Vec::new(),
            chain_config: SignalChainConfig::amp_only("nox30"),
            sample_rate: None,
        }
    }
}

impl Default for GreyboundParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Top Boost Volume",
                0.55,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            bass: FloatParam::new("Bass", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            cut: FloatParam::new("Cut", 0.35, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            tone: FloatParam::new("Treble", 0.6, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            master: FloatParam::new(
                "Output Trim",
                db_to_gain(-9.0),
                FloatRange::Skewed {
                    min: db_to_gain(-36.0),
                    max: db_to_gain(6.0),
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            speaker_ir: BoolParam::new("Speaker IR", false),
            fuzz: BoolParam::new("Muffin Fuzz", false),
            fuzz_sustain: FloatParam::new(
                "Fuzz Sustain",
                MuffinControls::default().sustain,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            fuzz_tone: FloatParam::new(
                "Fuzz Tone",
                MuffinControls::default().tone,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            fuzz_level: FloatParam::new(
                "Fuzz Level",
                MuffinControls::default().level,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            overdrive: BoolParam::new("Minotaur Overdrive", false),
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

        greybound_plugin_ui::create_iced_editor(
            PluginUiConfig {
                title: "Greybound",
                width: DESIGN_WIDTH as u32,
                height: DESIGN_HEIGHT as u32,
                background: iced::Color::from_rgb(0.72, 0.78, 0.91),
            },
            move |context| GreyboundPluginApp::new(params.clone(), context),
        )
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let channels = audio_io_layout
            .main_output_channels
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;
        let mut chain_config = self.chain_config.clone();
        if chain_config.pre_amp.is_empty() {
            chain_config
                .pre_amp
                .push(DeviceSlotConfig::active(DeviceConfig::Minotaur));
            chain_config
                .pre_amp
                .push(DeviceSlotConfig::active(DeviceConfig::Muffin));
        }
        self.sample_rate = Some(buffer_config.sample_rate);
        self.channels = build_signal_chains(buffer_config.sample_rate, channels, &chain_config);
        let sample_rate = buffer_config.sample_rate as u32;
        self.speakers = (0..channels)
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
            let controls = AmpControls {
                volume: self.params.gain.smoothed.next(),
                bass: self.params.bass.smoothed.next(),
                cut: self.params.cut.smoothed.next(),
                treble: self.params.tone.smoothed.next(),
                output: self.params.master.smoothed.next(),
                drive: 0.0,
                presence: 0.0,
                sag: 0.0,
            };
            let fuzz_controls = MuffinControls {
                sustain: self.params.fuzz_sustain.smoothed.next(),
                tone: self.params.fuzz_tone.smoothed.next(),
                level: self.params.fuzz_level.smoothed.next(),
            };
            let overdrive_controls = MinotaurControls {
                gain: self.params.overdrive_gain.smoothed.next(),
                treble: self.params.overdrive_treble.smoothed.next(),
                output: self.params.overdrive_output.smoothed.next(),
            };
            let device_controls = [
                DeviceSlotControls {
                    bypassed: !self.params.overdrive.value(),
                    controls: DeviceControls::Minotaur(overdrive_controls),
                },
                DeviceSlotControls {
                    bypassed: !self.params.fuzz.value(),
                    controls: DeviceControls::Muffin(fuzz_controls),
                },
            ];
            let chain_controls = SignalChainControls {
                amp: controls,
                devices: &device_controls,
            };

            for (channel, sample) in channel_samples.iter_mut().enumerate() {
                let amp_output = self.channels[channel].process(*sample, chain_controls);
                *sample =
                    self.speakers[channel].process(amp_output, self.params.speaker_ir.value());
            }
        }

        ProcessStatus::Normal
    }
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

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

struct GreyboundPluginApp {
    ui: GreyboundUi,
    params: Arc<GreyboundParams>,
    context: Arc<dyn GuiContext>,
}

impl GreyboundPluginApp {
    fn new(params: Arc<GreyboundParams>, context: Arc<dyn GuiContext>) -> Self {
        preload_render_assets();
        let mut ui = GreyboundUi::new(AppProfile::greybound_glass());
        ui.update(Message::WindowResized {
            width: DESIGN_WIDTH as u32,
            height: DESIGN_HEIGHT as u32,
        });
        let app = Self {
            ui,
            params,
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
                .raw_set_parameter_normalized(self.params.master.as_ptr(), amp.master);
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
    }
}

impl PluginIcedApp for GreyboundPluginApp {
    type Message = Message;

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
