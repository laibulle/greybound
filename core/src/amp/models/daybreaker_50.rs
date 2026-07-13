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
    presence_highpass: WdfHighpass,
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
            presence_highpass: WdfHighpass::from_rc(sample_rate, 22_000.0, 4.7e-9),
            presence_filter: OnePoleLowpass::new(sample_rate, 4_823.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 100_000.0, 68e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 4_823.0),
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

/// Applies a triode transfer at a controlled drive level while retaining its
/// small-signal gain.  The clean NAM reference stays linear much further into
/// the nominal guitar-input range than the unscaled stage approximation.  The
/// drive factor therefore models available voltage headroom rather than a
/// post-hoc output trim; at full edge drive it is exactly the original stage.
#[inline]
fn headroom_triode_stage(input: f32, bias: f32, drive: f32) -> f32 {
    let drive = drive.clamp(0.25, 1.0);
    triode_stage(input * drive, bias) / drive
}

impl AmpModel for Daybreaker50 {
    fn reset(&mut self) {
        *self = Self::new(self.sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let volume = controls.volume.clamp(0.0, 1.0);
        let edge = (controls.drive.clamp(0.0, 1.0) * 0.68
            + ((volume - 0.78) / 0.22).clamp(0.0, 1.0) * 0.32)
            .clamp(0.0, 1.0);
        // Calibrated from the Dumble Steel String Singer Clean NAM sweep:
        // retain the original full-edge curve, but give the clean operating
        // point 8.6 dB of stage headroom before cascading preamp saturation.
        let clean_stage_drive = 0.37 + edge * 0.63;
        let master = controls.output.clamp(0.0, 2.0);

        let input = self.input_coupling.process(input);
        let first_bypass = self.first_cathode_bypass.process(input);
        let first_stage =
            headroom_triode_stage(input * 1.18 + first_bypass * 0.12, 0.105, clean_stage_drive);
        let toned = self.tone_stack(first_stage, controls.bass, controls.cut, controls.treble);

        let channel_gain = 1.30 + volume * 3.55;
        let bright = toned - self.gain_bright_filter.process(toned);
        let bright_bypass = bright * ((1.0 - volume) * 0.72 + edge * 0.08);
        let recovery = headroom_triode_stage(
            self.recovery_coupling
                .process(toned * channel_gain + bright_bypass),
            0.090 - edge * 0.030,
            clean_stage_drive,
        );
        let master_signal =
            headroom_triode_stage(recovery * (1.04 + edge * 0.16), 0.050, clean_stage_drive)
                * (0.34 + master * 0.72);

        let presence_amount = controls.presence.clamp(0.0, 1.0);
        let presence_band = self
            .presence_filter
            .process(self.presence_highpass.process(master_signal));
        // Active recovery after the post-tone High filter. The 22k / 4.7n
        // high-pass and 22k / 1.5n low-pass boundaries are mirrored in the
        // Daybreaker SPICE fixture; this gain is the controlled recovery ratio,
        // not an EQ makeup trim.
        let presence_recovery_gain = 0.18 + presence_amount * 2.4;
        let presence_shaped = master_signal + presence_band * presence_recovery_gain;

        let pi_input = self
            .phase_inverter_coupling
            .process(presence_shaped * (1.24 + edge * 0.30));
        let phase_a = headroom_triode_stage(pi_input, 0.050, clean_stage_drive);
        let phase_b = headroom_triode_stage(-pi_input * 0.98, -0.046, clean_stage_drive);
        let differential = (phase_a - phase_b) * 0.5;

        let current_demand = (differential.abs() * (0.96 + edge * 0.42) - 0.60).max(0.0);
        let bias_shift = self.bias_envelope.process(current_demand);
        let sag = self.supply_sag.process(current_demand * current_demand);
        let sag_control = controls.sag.clamp(0.0, 1.0);
        let power_drive = differential * (0.92 + edge * 1.55)
            / (1.0 + bias_shift * 0.16 + sag * (0.06 + sag_control * 0.16));
        // The clean capture has a substantially wider 6L6 operating region
        // than the original graybox calibration.  Scale the grid excursion
        // before the physical power-stage solve and compensate after it, so
        // the linear transfer stays intact while the clipping knee moves.
        // 0.25 is 12 dB of clean power-stage headroom, selected from the NAM
        // sine-level sweep; the edge endpoint remains the unmodified curve.
        let clean_power_drive = 0.25 + edge * 0.75;
        let power_output = self
            .power_stage
            .process(power_drive * clean_power_drive, sag_control)
            / clean_power_drive;

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
