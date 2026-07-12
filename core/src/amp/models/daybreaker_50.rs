use super::AmpModel;
use crate::amp::components::{triode_stage, EnvelopeFollower, OnePoleLowpass, WdfHighpass};
use crate::amp::AmpControls;
use crate::circuit::power::{PushPull6L6Params, PushPull6L6Stage};

/// Modern high-headroom clean/edge 6L6 target.
///
/// This is deliberately a stage-level graybox. The Daybreaker 50 references an
/// amp-head NAM for integration calibration; it does not claim a component-exact
/// model of that capture's source amp.
pub(in crate::amp) struct Daybreaker50 {
    sample_rate: f32,
    input_coupling: WdfHighpass,
    first_cathode_bypass: WdfHighpass,
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
}

impl Daybreaker50 {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            input_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            first_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_800.0, 2.2e-6),
            stack_low_filter: OnePoleLowpass::new(sample_rate, 190.0),
            stack_high_filter: OnePoleLowpass::new(sample_rate, 2_450.0),
            gain_bright_filter: OnePoleLowpass::new(sample_rate, 3_100.0),
            recovery_coupling: WdfHighpass::from_rc(sample_rate, 330_000.0, 22e-9),
            phase_inverter_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            presence_filter: OnePoleLowpass::new(sample_rate, 3_800.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 100_000.0, 68e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 32_000.0),
            bias_envelope: EnvelopeFollower::new(sample_rate, 0.016, 0.320),
            supply_sag: EnvelopeFollower::new(sample_rate, 0.042, 0.680),
            power_stage: PushPull6L6Stage::new(power_stage_params(sample_rate)),
        }
    }

    #[inline]
    fn tone_stack(&mut self, input: f32, bass: f32, mid: f32, treble: f32) -> f32 {
        let low = self.stack_low_filter.process(input);
        let high = input - self.stack_high_filter.process(input);
        let mid_band = input - low - high;

        let bass_gain = 0.20 + bass.clamp(0.0, 1.0) * 0.70;
        let mid_gain = 0.34 + mid.clamp(0.0, 1.0) * 0.94;
        let treble_gain = 0.34 + treble.clamp(0.0, 1.0) * 0.98;

        (low * bass_gain + mid_band * mid_gain + high * treble_gain) * 0.48
    }
}

impl AmpModel for Daybreaker50 {
    fn reset(&mut self) {
        *self = Self::new(self.sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let input = self.input_coupling.process(input);
        let volume = controls.volume.clamp(0.0, 1.0);
        let edge = (controls.drive.clamp(0.0, 1.0) * 0.68
            + ((volume - 0.78) / 0.22).clamp(0.0, 1.0) * 0.32)
            .clamp(0.0, 1.0);
        let master = controls.output.clamp(0.0, 2.0);

        let first_bypass = self.first_cathode_bypass.process(input);
        let first_stage = triode_stage(input * 1.18 + first_bypass * 0.12, 0.105);
        let toned = self.tone_stack(first_stage, controls.bass, controls.cut, controls.treble);

        let channel_gain = 1.30 + volume * 3.55;
        let bright = toned - self.gain_bright_filter.process(toned);
        let bright_bypass = bright * ((1.0 - volume) * 0.72 + edge * 0.08);
        let recovery = triode_stage(
            self.recovery_coupling
                .process(toned * channel_gain + bright_bypass),
            0.090 - edge * 0.030,
        );
        let master_signal =
            triode_stage(recovery * (1.04 + edge * 0.16), 0.050) * (0.34 + master * 0.72);

        let presence_amount = controls.presence.clamp(0.0, 1.0);
        self.presence_filter
            .set_cutoff(self.sample_rate, 1_300.0 + presence_amount * 4_600.0);
        let presence_low = self.presence_filter.process(master_signal);
        let presence_shaped =
            presence_low + (master_signal - presence_low) * (0.26 + presence_amount * 0.58);

        let pi_input = self
            .phase_inverter_coupling
            .process(presence_shaped * (1.24 + edge * 0.30));
        let phase_a = triode_stage(pi_input, 0.050);
        let phase_b = triode_stage(-pi_input * 0.98, -0.046);
        let differential = (phase_a - phase_b) * 0.5;

        let current_demand = (differential.abs() * (0.96 + edge * 0.42) - 0.60).max(0.0);
        let bias_shift = self.bias_envelope.process(current_demand);
        let sag = self.supply_sag.process(current_demand * current_demand);
        let sag_control = controls.sag.clamp(0.0, 1.0);
        let power_drive = differential * (0.92 + edge * 1.55)
            / (1.0 + bias_shift * 0.16 + sag * (0.06 + sag_control * 0.16));
        let power_output = self.power_stage.process(power_drive, sag_control);

        let transformer = self
            .transformer_lowpass
            .process(self.transformer_highpass.process(power_output));
        transformer * 0.015 * (1.0 - sag_control * 0.04)
    }
}

fn power_stage_params(sample_rate: f32) -> PushPull6L6Params {
    PushPull6L6Params {
        sample_rate,
        nominal_supply_voltage: 455.0,
        screen_voltage: 440.0,
        screen_resistance: 470.0,
        screen_capacitance: 33e-6,
        primary_half_resistance: 1_900.0,
        supply_resistance: 210.0,
        supply_capacitance: 100e-6,
        fixed_bias_voltage: 38.0,
        bias_shift_resistance: 22_000.0,
        bias_shift_capacitance: 1.0e-6,
        idle_current: 0.062,
        drive_gain: 22.0,
        current_gain: 0.017,
        load_current_coupling: 0.92,
        attack_current_coupling: 0.14,
        compression: 0.060,
        output_scale: 0.010,
    }
}
