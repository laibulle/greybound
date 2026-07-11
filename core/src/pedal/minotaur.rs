use super::common::*;
use super::*;

#[derive(Clone)]
struct MinotaurClipNeuralConfig {
    descriptor_path: PathBuf,
    mode: NeuralCellMode,
}

#[derive(Clone)]
enum MinotaurClipNeuralSelection {
    Default,
    Disabled,
    Configured(MinotaurClipNeuralConfig),
}

struct MinotaurClipNeural {
    runtime: NeuralCellRuntime,
    mode: NeuralCellMode,
    buffer_history: [f32; 4],
    last_clip_ac_v: f32,
}

struct MinotaurToneNeural {
    runtime: NeuralCellRuntime,
    mode: NeuralCellMode,
    buffer_history: [f32; 4],
    last_tone_ac_v: f32,
}

static MINOTAUR_CLIP_NEURAL_CONFIG: OnceLock<Mutex<MinotaurClipNeuralSelection>> = OnceLock::new();
static MINOTAUR_TONE_NEURAL_CONFIG: OnceLock<Mutex<MinotaurClipNeuralSelection>> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub struct MinotaurControls {
    pub gain: f32,
    pub treble: f32,
    pub output: f32,
}

impl Default for MinotaurControls {
    fn default() -> Self {
        Self {
            gain: 0.42,
            treble: 0.70,
            output: 0.03,
        }
    }
}

pub struct Minotaur {
    pub(super) input_connection: ConnectionState,
    input_highpass: OnePoleHighpass,
    drive_input_highpass: OnePoleHighpass,
    drive_feedback_lowpass: OnePoleLowpass,
    clip_coupling_highpass: OnePoleHighpass,
    clean_feed_highpass: OnePoleHighpass,
    summing_lowpass: OnePoleLowpass,
    treble_lowpass: OnePoleLowpass,
    treble_highpass: OnePoleHighpass,
    treble_mid_highpass: OnePoleHighpass,
    treble_mid_lowpass: OnePoleLowpass,
    transient_fast_envelope: OnePoleLowpass,
    transient_slow_envelope: OnePoleLowpass,
    level_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
    clip_neural: Option<MinotaurClipNeural>,
    tone_neural: Option<MinotaurToneNeural>,
    last_boundary_states: [StageBoundaryState; 8],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MinotaurNodeVoltages {
    pub loaded_input: f32,
    pub buffered: f32,
    pub clean_feed: f32,
    pub drive_stage: f32,
    pub clipped: f32,
    pub sum_node: f32,
    pub voiced: f32,
    pub output: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct MinotaurProcessResult {
    pub(super) signal: ElectricalSignal,
    pub(super) stages: MinotaurNodeVoltages,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct MinotaurCircuitParams {
    pub(super) clip_knee_v: f32,
    pub(super) output_makeup_gain: f32,
}

impl Minotaur {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 560.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 220e-12),
            input_highpass: OnePoleHighpass::new(sample_rate, 1.6),
            drive_input_highpass: OnePoleHighpass::new(sample_rate, 26.0),
            drive_feedback_lowpass: OnePoleLowpass::new(sample_rate, 900.0),
            clip_coupling_highpass: OnePoleHighpass::new(sample_rate, 3.4),
            clean_feed_highpass: OnePoleHighpass::new(sample_rate, 106.0),
            summing_lowpass: OnePoleLowpass::new(sample_rate, 4_900.0),
            treble_lowpass: OnePoleLowpass::new(sample_rate, 2_100.0),
            treble_highpass: OnePoleHighpass::new(sample_rate, 1_650.0),
            treble_mid_highpass: OnePoleHighpass::new(sample_rate, 850.0),
            treble_mid_lowpass: OnePoleLowpass::new(sample_rate, 3_600.0),
            transient_fast_envelope: OnePoleLowpass::new(sample_rate, 120.0),
            transient_slow_envelope: OnePoleLowpass::new(sample_rate, 12.0),
            level_highpass: OnePoleHighpass::new(sample_rate, 0.34),
            output_lowpass: OnePoleLowpass::new(sample_rate, 18_000.0),
            clip_neural: minotaur_clip_neural(),
            tone_neural: minotaur_tone_neural(),
            last_boundary_states: minotaur_boundaries(MinotaurNodeVoltages::default()),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_highpass.reset();
        self.drive_input_highpass.reset();
        self.drive_feedback_lowpass.reset();
        self.clip_coupling_highpass.reset();
        self.clean_feed_highpass.reset();
        self.summing_lowpass.reset();
        self.treble_lowpass.reset();
        self.treble_highpass.reset();
        self.treble_mid_highpass.reset();
        self.treble_mid_lowpass.reset();
        self.transient_fast_envelope.reset();
        self.transient_slow_envelope.reset();
        self.level_highpass.reset();
        self.output_lowpass.reset();
        if let Some(neural) = &mut self.clip_neural {
            neural.reset();
        }
        if let Some(neural) = &mut self.tone_neural {
            neural.reset();
        }
        self.last_boundary_states = minotaur_boundaries(MinotaurNodeVoltages::default());
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: MinotaurControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_with_node_voltages(
        &mut self,
        input: ElectricalSignal,
        controls: MinotaurControls,
    ) -> (ElectricalSignal, MinotaurNodeVoltages) {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        let result = self.process_loaded_voltage_with_stages(
            loaded_input,
            controls,
            MinotaurCircuitParams::current(),
        );
        self.last_boundary_states = minotaur_boundaries(result.stages);
        (result.signal, result.stages)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: MinotaurControls,
    ) -> ElectricalSignal {
        let result = self.process_loaded_voltage_with_stages(
            loaded_input,
            controls,
            MinotaurCircuitParams::current(),
        );
        self.last_boundary_states = minotaur_boundaries(result.stages);
        result.signal
    }

    pub fn boundary_states(&self) -> [StageBoundaryState; 8] {
        self.last_boundary_states
    }

    pub(super) fn process_loaded_voltage_with_stages(
        &mut self,
        loaded_input: f32,
        controls: MinotaurControls,
        params: MinotaurCircuitParams,
    ) -> MinotaurProcessResult {
        let gain = controls.gain.clamp(0.0, 1.0);
        let treble = controls.treble.clamp(0.0, 1.0);
        let output = controls.output.clamp(0.0, 1.0);

        let buffered = self.input_highpass.process(loaded_input);

        // Klon-style dual-gang gain: the clean feed stays present while the
        // non-inverting gain stage and summing contribution rise together.
        let gain_pot = gain.powf(1.15);
        let drive_input = self.drive_input_highpass.process(buffered);
        let feedback_gain = 1.45 + gain_pot * 13.5;
        let drive_stage = self
            .drive_feedback_lowpass
            .process(drive_input * feedback_gain);
        let clip_input = self.clip_coupling_highpass.process(drive_stage);
        let analytic_clipped = diode_pair_clip(clip_input, params.clip_knee_v);
        let clipped = if let Some(neural) = &mut self.clip_neural {
            neural.process(buffered, gain, treble, output, analytic_clipped)
        } else {
            analytic_clipped
        };

        let clean_feed =
            self.clean_feed_highpass.process(buffered) * (0.13 - gain * 0.06).max(0.035);
        let drive_feed = clipped * (2.35 + gain_pot * 2.2);
        let sum_node = self
            .summing_lowpass
            .process(drive_feed + clean_feed - drive_input * 0.08);

        let low = self.treble_lowpass.process(sum_node);
        let high = self.treble_highpass.process(sum_node);
        let mid = self
            .treble_mid_lowpass
            .process(self.treble_mid_highpass.process(sum_node));
        let tone_gain = 0.78 + treble * 0.44;
        let analytic_voiced =
            low * (0.88 - treble * 0.36) + high * (0.17 + treble * 1.00) + mid * 0.28;
        let voiced = if let Some(neural) = &mut self.tone_neural {
            neural.process(buffered, gain, treble, output, analytic_voiced)
        } else {
            analytic_voiced
        };
        let envelope_input = voiced.abs();
        let fast_envelope = self.transient_fast_envelope.process(envelope_input);
        let slow_envelope = self.transient_slow_envelope.process(envelope_input);
        let transient_lift =
            ((fast_envelope - slow_envelope).max(0.0) / (slow_envelope + 1.0e-5)).clamp(0.0, 1.0);
        let dynamic_gain = 1.0 + transient_lift * 0.055;
        // The Klon level pot sits after an active treble recovery stage, so it
        // can produce a real boost instead of acting as a small post-DSP trim.
        let level = output * 1.50;
        let final_output = self
            .output_lowpass
            .process(
                self.level_highpass
                    .process(voiced * tone_gain * dynamic_gain * level * params.output_makeup_gain),
            )
            .clamp(-4.5, 4.5);

        MinotaurProcessResult {
            signal: ElectricalSignal::new(final_output, Self::OUTPUT_IMPEDANCE_OHMS),
            stages: MinotaurNodeVoltages {
                loaded_input,
                buffered,
                clean_feed,
                drive_stage,
                clipped,
                sum_node,
                voiced,
                output: final_output,
            },
        }
    }
}

impl MinotaurCircuitParams {
    pub(super) fn current() -> Self {
        Self {
            clip_knee_v: 0.36,
            output_makeup_gain: 1.023,
        }
    }
}

pub(super) fn minotaur_boundaries(stages: MinotaurNodeVoltages) -> [StageBoundaryState; 8] {
    [
        pedal_boundary_state(
            "input_load",
            stages.loaded_input,
            GUITAR_SOURCE_IMPEDANCE_OHMS,
            Minotaur::INPUT_IMPEDANCE_OHMS,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "input_coupling",
            stages.buffered,
            1_000.0,
            470_000.0,
            StageCoupling::Buffered,
            9.0,
        ),
        pedal_boundary_state(
            "clean_path",
            stages.clean_feed,
            10_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "drive_gain",
            stages.drive_stage,
            10_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "soft_clip",
            stages.clipped,
            1_000.0,
            100_000.0,
            StageCoupling::DcCoupled,
            0.42,
        ),
        pedal_boundary_state(
            "summing_node",
            stages.sum_node,
            12_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "treble_presence",
            stages.voiced,
            10_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "output_driver",
            stages.output,
            Minotaur::OUTPUT_IMPEDANCE_OHMS,
            AMP_INPUT_IMPEDANCE_OHMS,
            StageCoupling::Buffered,
            9.0,
        ),
    ]
}

pub fn configure_minotaur_clip_neural(descriptor_path: Option<PathBuf>, mode: NeuralCellMode) {
    let slot = MINOTAUR_CLIP_NEURAL_CONFIG
        .get_or_init(|| Mutex::new(MinotaurClipNeuralSelection::Default));
    *slot.lock().expect("minotaur neural config mutex poisoned") =
        if let Some(descriptor_path) = descriptor_path {
            MinotaurClipNeuralSelection::Configured(MinotaurClipNeuralConfig {
                descriptor_path,
                mode,
            })
        } else {
            MinotaurClipNeuralSelection::Disabled
        };
}

pub fn configure_minotaur_tone_neural(descriptor_path: Option<PathBuf>, mode: NeuralCellMode) {
    let slot = MINOTAUR_TONE_NEURAL_CONFIG
        .get_or_init(|| Mutex::new(MinotaurClipNeuralSelection::Default));
    *slot.lock().expect("minotaur neural config mutex poisoned") =
        if let Some(descriptor_path) = descriptor_path {
            MinotaurClipNeuralSelection::Configured(MinotaurClipNeuralConfig {
                descriptor_path,
                mode,
            })
        } else {
            MinotaurClipNeuralSelection::Disabled
        };
}

fn minotaur_clip_neural() -> Option<MinotaurClipNeural> {
    let config = match minotaur_clip_neural_selection() {
        MinotaurClipNeuralSelection::Disabled => None,
        MinotaurClipNeuralSelection::Configured(config) => Some(config),
        MinotaurClipNeuralSelection::Default => env_minotaur_clip_neural(
            "GREYBOUND_MINOTAUR_CLIP_REPLACE_DESCRIPTOR",
            NeuralCellMode::Replace,
        )
        .or_else(|| {
            env_minotaur_clip_neural(
                "GREYBOUND_MINOTAUR_CLIP_SHADOW_DESCRIPTOR",
                NeuralCellMode::Shadow,
            )
        }),
    }?;
    let cell = ExperimentalNeuralCell::from_descriptor_path(&config.descriptor_path).ok()?;
    Some(MinotaurClipNeural {
        runtime: cell.into_runtime(),
        mode: config.mode,
        buffer_history: [0.0; 4],
        last_clip_ac_v: 0.0,
    })
}

fn minotaur_tone_neural() -> Option<MinotaurToneNeural> {
    let config = match minotaur_tone_neural_selection() {
        MinotaurClipNeuralSelection::Disabled => None,
        MinotaurClipNeuralSelection::Configured(config) => Some(config),
        MinotaurClipNeuralSelection::Default => env_minotaur_clip_neural(
            "GREYBOUND_MINOTAUR_TONE_REPLACE_DESCRIPTOR",
            NeuralCellMode::Replace,
        )
        .or_else(|| {
            env_minotaur_clip_neural(
                "GREYBOUND_MINOTAUR_TONE_SHADOW_DESCRIPTOR",
                NeuralCellMode::Shadow,
            )
        }),
    }?;
    let cell = ExperimentalNeuralCell::from_descriptor_path(&config.descriptor_path).ok()?;
    Some(MinotaurToneNeural {
        runtime: cell.into_runtime(),
        mode: config.mode,
        buffer_history: [0.0; 4],
        last_tone_ac_v: 0.0,
    })
}

fn minotaur_clip_neural_selection() -> MinotaurClipNeuralSelection {
    MINOTAUR_CLIP_NEURAL_CONFIG
        .get_or_init(|| Mutex::new(MinotaurClipNeuralSelection::Default))
        .lock()
        .expect("minotaur neural config mutex poisoned")
        .clone()
}

fn minotaur_tone_neural_selection() -> MinotaurClipNeuralSelection {
    MINOTAUR_TONE_NEURAL_CONFIG
        .get_or_init(|| Mutex::new(MinotaurClipNeuralSelection::Default))
        .lock()
        .expect("minotaur neural config mutex poisoned")
        .clone()
}

fn env_minotaur_clip_neural(name: &str, mode: NeuralCellMode) -> Option<MinotaurClipNeuralConfig> {
    env::var_os(name).map(|descriptor_path| MinotaurClipNeuralConfig {
        descriptor_path: PathBuf::from(descriptor_path),
        mode,
    })
}

impl MinotaurClipNeural {
    fn reset(&mut self) {
        self.buffer_history = [0.0; 4];
        self.last_clip_ac_v = 0.0;
    }

    fn process(
        &mut self,
        buffered: f32,
        gain: f32,
        treble: f32,
        _output: f32,
        analytic_clipped: f32,
    ) -> f32 {
        self.buffer_history.copy_within(0..3, 1);
        self.buffer_history[0] = buffered;
        let mut features = vec![0.0; self.runtime.input_features()];
        let history_len = features.len().min(4);
        features[..history_len].copy_from_slice(&self.buffer_history[..history_len]);
        if features.len() > 4 {
            features[4] = gain;
        }
        if features.len() > 5 {
            features[5] = treble;
        }
        if features.len() > 6 {
            features[6] = 0.70;
        }
        let neural = self
            .runtime
            .process_features(&features)
            .unwrap_or(analytic_clipped)
            .clamp(-1.5, 1.5);
        self.last_clip_ac_v = neural;
        if self.mode == NeuralCellMode::Replace {
            neural
        } else {
            analytic_clipped
        }
    }
}

impl MinotaurToneNeural {
    fn reset(&mut self) {
        self.buffer_history = [0.0; 4];
        self.last_tone_ac_v = 0.0;
    }

    fn process(
        &mut self,
        buffered: f32,
        gain: f32,
        treble: f32,
        _output: f32,
        analytic_tone: f32,
    ) -> f32 {
        self.buffer_history.copy_within(0..3, 1);
        self.buffer_history[0] = buffered;
        let mut features = vec![0.0; self.runtime.input_features()];
        let history_len = features.len().min(4);
        features[..history_len].copy_from_slice(&self.buffer_history[..history_len]);
        if features.len() > 4 {
            features[4] = gain;
        }
        if features.len() > 5 {
            features[5] = treble;
        }
        let neural = self
            .runtime
            .process_features(&features)
            .unwrap_or(analytic_tone)
            .clamp(-4.5, 4.5);
        self.last_tone_ac_v = neural;
        if self.mode == NeuralCellMode::Replace {
            neural
        } else {
            analytic_tone
        }
    }
}
