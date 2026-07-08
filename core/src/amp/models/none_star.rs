use super::AmpModel;
use crate::amp::components::{triode_stage, EnvelopeFollower, OnePoleLowpass, WdfHighpass};
use crate::amp::{AmpControls, NeuralCellMode};
use crate::circuit::power::{PushPull6L6Params, PushPull6L6Stage};
use crate::neural_cell::{ExperimentalNeuralCell, NeuralCellRuntime};
use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone)]
struct Power6L6NeuralConfig {
    descriptor_path: PathBuf,
    mode: NeuralCellMode,
}

#[derive(Clone)]
enum Power6L6NeuralSelection {
    Default,
    Disabled,
    Configured(Power6L6NeuralConfig),
}

struct Power6L6Neural {
    runtime: NeuralCellRuntime,
    mode: NeuralCellMode,
}

static POWER_6L6_NEURAL_CONFIG: OnceLock<Mutex<Power6L6NeuralSelection>> = OnceLock::new();

pub(in crate::amp) struct NoneStar {
    sample_rate: f32,
    input_coupling: WdfHighpass,
    first_cathode_bypass: WdfHighpass,
    second_cathode_bypass: WdfHighpass,
    stack_low_filter: OnePoleLowpass,
    stack_high_filter: OnePoleLowpass,
    gain_bright_filter: OnePoleLowpass,
    recovery_coupling: WdfHighpass,
    phase_inverter_coupling: WdfHighpass,
    presence_filter: OnePoleLowpass,
    transformer_highpass: WdfHighpass,
    transformer_lowpass: OnePoleLowpass,
    bias_envelope: EnvelopeFollower,
    supply_sag: EnvelopeFollower,
    power_stage: PushPull6L6Stage,
    power_6l6_neural: Option<Power6L6Neural>,
}

impl NoneStar {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            input_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            first_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_500.0, 4.7e-6),
            second_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_800.0, 2.2e-6),
            stack_low_filter: OnePoleLowpass::new(sample_rate, 235.0),
            stack_high_filter: OnePoleLowpass::new(sample_rate, 2_150.0),
            gain_bright_filter: OnePoleLowpass::new(sample_rate, 2_700.0),
            recovery_coupling: WdfHighpass::from_rc(sample_rate, 330_000.0, 22e-9),
            phase_inverter_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            presence_filter: OnePoleLowpass::new(sample_rate, 3_600.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 120_000.0, 47e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 36_000.0),
            bias_envelope: EnvelopeFollower::new(sample_rate, 0.018, 0.260),
            supply_sag: EnvelopeFollower::new(sample_rate, 0.034, 0.520),
            power_stage: PushPull6L6Stage::new(power_stage_params(sample_rate)),
            power_6l6_neural: power_6l6_neural(),
        }
    }

    #[inline]
    fn tone_stack(&mut self, input: f32, bass: f32, mid: f32, treble: f32) -> f32 {
        let low = self.stack_low_filter.process(input);
        let high = input - self.stack_high_filter.process(input);
        let mid_band = input - low - high;

        let bass_gain = 0.22 + bass.clamp(0.0, 1.0) * 0.72;
        let mid_gain = 0.30 + mid.clamp(0.0, 1.0) * 0.96;
        let treble_gain = 0.38 + treble.clamp(0.0, 1.0) * 1.22;

        (low * bass_gain + mid_band * mid_gain + high * treble_gain) * 0.42
    }
}

impl AmpModel for NoneStar {
    fn reset(&mut self) {
        *self = Self::new(self.sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let input = self.input_coupling.process(input);
        let volume = controls.volume.clamp(0.0, 1.0);
        let edge = controls.drive.max((volume - 0.62) / 0.38).clamp(0.0, 1.0);
        let master = controls.output.clamp(0.0, 2.0);

        // Clean channel shape: the first triode mostly establishes high-headroom voltage
        // before the passive tone/gain network. Pushing `volume` happens after the stack,
        // unlike the older two-hot-stages-before-tone approximation.
        let first_bypass = self.first_cathode_bypass.process(input);
        let first_drive = input * 1.42 + first_bypass * 0.18;
        let first_stage = triode_stage(first_drive, 0.080);

        let toned = self.tone_stack(first_stage, controls.bass, controls.cut, controls.treble);
        let channel_gain = 1.65 + volume * 4.85;
        let gain_bright = toned - self.gain_bright_filter.process(toned);
        let bright_bypass = gain_bright * ((1.0 - volume) * 1.05 + edge * 0.12);
        let second_bypass = self.second_cathode_bypass.process(toned);
        let recovery_drive =
            toned * channel_gain + bright_bypass + second_bypass * (0.10 + edge * 0.30);
        let recovery_input = self.recovery_coupling.process(recovery_drive);
        let recovery = triode_stage(recovery_input, 0.060 - edge * 0.040);

        let loop_driver = triode_stage(recovery * (1.12 + edge * 0.32), 0.028);
        let master_signal = loop_driver * (0.40 + master * 0.82);

        let presence_amount = controls.presence.clamp(0.0, 1.0);
        let presence_hz = 1_100.0 + presence_amount * 4_200.0;
        self.presence_filter
            .set_cutoff(self.sample_rate, presence_hz);
        let presence_low = self.presence_filter.process(master_signal);
        let presence_shaped =
            presence_low + (master_signal - presence_low) * (0.22 + presence_amount * 0.72);

        let pi_input = self
            .phase_inverter_coupling
            .process(presence_shaped * (1.55 + edge * 0.42));
        let phase_a = triode_stage(pi_input * 1.05, 0.032);
        let phase_b = triode_stage(-pi_input * 1.02, -0.026);
        let differential = (phase_a - phase_b) * 0.5;

        let current_demand = (differential.abs() * (1.12 + edge * 0.36) - 0.52).max(0.0);
        let bias_shift = self.bias_envelope.process(current_demand);
        let sag = self.supply_sag.process(current_demand * current_demand);
        let sag_control = controls.sag.clamp(0.0, 1.0);
        let power_drive = differential * (1.08 + edge * 2.20)
            / (1.0 + bias_shift * 0.22 + sag * (0.10 + sag_control * 0.20));
        let analytic_power_output = self.power_stage.process(power_drive, sag_control);
        let power_output = if let Some(neural) = &mut self.power_6l6_neural {
            let features = [power_drive, bias_shift, sag, edge, sag_control];
            match neural.runtime.process_features(&features) {
                Ok(neural_output) if neural.mode == NeuralCellMode::Replace => neural_output,
                Ok(_) | Err(_) => analytic_power_output,
            }
        } else {
            analytic_power_output
        };

        let mut transformer = self.transformer_highpass.process(power_output);
        transformer = self.transformer_lowpass.process(transformer);
        transformer * 0.016 * (1.0 - sag_control * 0.06)
    }
}

fn power_stage_params(sample_rate: f32) -> PushPull6L6Params {
    PushPull6L6Params {
        sample_rate,
        nominal_supply_voltage: 435.0,
        screen_voltage: 420.0,
        screen_resistance: 470.0,
        screen_capacitance: 22e-6,
        primary_half_resistance: 2_150.0,
        supply_resistance: 340.0,
        supply_capacitance: 47e-6,
        fixed_bias_voltage: 36.0,
        bias_shift_resistance: 18_000.0,
        bias_shift_capacitance: 2.2e-6,
        idle_current: 0.070,
        drive_gain: 24.0,
        current_gain: 0.018,
        load_current_coupling: 0.95,
        attack_current_coupling: 0.18,
        compression: 0.08,
        output_scale: 0.010,
    }
}

pub(super) fn configure_power_6l6_neural(descriptor_path: Option<PathBuf>, mode: NeuralCellMode) {
    let slot = POWER_6L6_NEURAL_CONFIG.get_or_init(|| Mutex::new(Power6L6NeuralSelection::Default));
    *slot
        .lock()
        .expect("none-star 6L6 neural config mutex poisoned") =
        if let Some(descriptor_path) = descriptor_path {
            Power6L6NeuralSelection::Configured(Power6L6NeuralConfig {
                descriptor_path,
                mode,
            })
        } else {
            Power6L6NeuralSelection::Disabled
        };
}

fn power_6l6_neural() -> Option<Power6L6Neural> {
    let config = match power_6l6_neural_selection() {
        Power6L6NeuralSelection::Disabled => None,
        Power6L6NeuralSelection::Configured(config) => Some(config),
        Power6L6NeuralSelection::Default => env_power_6l6_neural(
            "GREYBOUND_NONE_STAR_POWER_6L6_DESCRIPTOR",
            NeuralCellMode::Replace,
        ),
    }?;
    let cell = ExperimentalNeuralCell::from_descriptor_path(&config.descriptor_path).ok()?;
    let runtime = cell.into_runtime();
    (runtime.input_features() == 5).then_some(Power6L6Neural {
        runtime,
        mode: config.mode,
    })
}

fn power_6l6_neural_selection() -> Power6L6NeuralSelection {
    POWER_6L6_NEURAL_CONFIG
        .get_or_init(|| Mutex::new(Power6L6NeuralSelection::Default))
        .lock()
        .expect("none-star 6L6 neural config mutex poisoned")
        .clone()
}

fn env_power_6l6_neural(name: &str, mode: NeuralCellMode) -> Option<Power6L6NeuralConfig> {
    env::var_os(name).map(|descriptor_path| Power6L6NeuralConfig {
        descriptor_path: PathBuf::from(descriptor_path),
        mode,
    })
}
