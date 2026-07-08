use super::AmpModel;
use crate::amp::components::{
    cathode_follower, six_l6_bank, triode_stage, EnvelopeFollower, OnePoleLowpass, WdfHighpass,
};
use crate::amp::AmpControls;

pub(in crate::amp) struct BoxerSevenLead {
    sample_rate: f32,
    input_coupling: WdfHighpass,
    bright_filter: OnePoleLowpass,
    v1_bypass: WdfHighpass,
    interstage_coupling: WdfHighpass,
    cold_clip_bypass: WdfHighpass,
    tone_low_filter: OnePoleLowpass,
    tone_high_filter: OnePoleLowpass,
    loop_coupling: WdfHighpass,
    presence_filter: OnePoleLowpass,
    excursion_filter: OnePoleLowpass,
    transformer_highpass: WdfHighpass,
    transformer_lowpass: OnePoleLowpass,
    bias_envelope: EnvelopeFollower,
    supply_sag: EnvelopeFollower,
}

impl BoxerSevenLead {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            input_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 22e-9),
            bright_filter: OnePoleLowpass::new(sample_rate, 2_900.0),
            v1_bypass: WdfHighpass::from_rc(sample_rate, 2_700.0, 1.0e-6),
            interstage_coupling: WdfHighpass::from_rc(sample_rate, 470_000.0, 4.7e-9),
            cold_clip_bypass: WdfHighpass::from_rc(sample_rate, 10_000.0, 680e-9),
            tone_low_filter: OnePoleLowpass::new(sample_rate, 185.0),
            tone_high_filter: OnePoleLowpass::new(sample_rate, 2_450.0),
            loop_coupling: WdfHighpass::from_rc(sample_rate, 470_000.0, 22e-9),
            presence_filter: OnePoleLowpass::new(sample_rate, 4_800.0),
            excursion_filter: OnePoleLowpass::new(sample_rate, 96.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 150_000.0, 22e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 13_500.0),
            bias_envelope: EnvelopeFollower::new(sample_rate, 0.012, 0.210),
            supply_sag: EnvelopeFollower::new(sample_rate, 0.030, 0.430),
        }
    }

    #[inline]
    fn tone_stack(&mut self, input: f32, bass: f32, middle: f32, treble: f32) -> f32 {
        let low = self.tone_low_filter.process(input);
        let high = input - self.tone_high_filter.process(input);
        let mid = input - low - high;

        let bass = bass.clamp(0.0, 1.0);
        let middle = middle.clamp(0.0, 1.0);
        let treble = treble.clamp(0.0, 1.0);

        let bass_gain = 0.18 + bass * 0.82;
        let mid_gain = 0.34 + middle * 1.12;
        let treble_gain = 0.28 + treble * 1.18;
        let low_tightening = 0.78 + (1.0 - treble) * 0.16;

        (low * bass_gain * low_tightening + mid * mid_gain + high * treble_gain) * 0.46
    }
}

impl AmpModel for BoxerSevenLead {
    fn reset(&mut self) {
        *self = Self::new(self.sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let gain = controls.volume.clamp(0.0, 1.0);
        let mode = controls.drive.clamp(0.0, 1.0);
        let master = controls.output.clamp(0.0, 2.0);
        let excursion = controls.sag.clamp(0.0, 1.0);

        let input = self.input_coupling.process(input);
        let bright = input - self.bright_filter.process(input);
        let bright_amount = (1.0 - gain).powf(1.7) * 0.46 + mode * 0.08;
        let v1_input = input * (1.05 + gain * 1.45) + bright * bright_amount;
        let v1 = triode_stage(
            v1_input * 2.25 + self.v1_bypass.process(v1_input) * 0.38,
            0.072,
        );

        let stage2_input = self
            .interstage_coupling
            .process(v1 * (1.58 + gain * 2.65 + mode * 0.42));
        let stage2 = triode_stage(stage2_input, -0.115 + mode * 0.026);

        let clip_drive = stage2 * (2.10 + gain * 2.85 + mode * 1.35)
            + self.cold_clip_bypass.process(stage2) * (0.24 + mode * 0.28);
        let cold_clipper = triode_stage(clip_drive, -0.205 + mode * 0.035);
        let compressed = cold_clipper / (1.0 + cold_clipper.abs() * (0.16 + mode * 0.14));

        let recovery = triode_stage(compressed * (1.58 + mode * 0.56), 0.030);
        let follower = cathode_follower(recovery * 0.92);
        let toned = self.tone_stack(follower, controls.bass, controls.cut, controls.treble);

        let channel_volume = self.loop_coupling.process(toned * (1.58 + gain * 1.10));
        let loop_driver = triode_stage(channel_volume * (1.18 + mode * 0.24), 0.020);

        let presence = controls.presence.clamp(0.0, 1.0);
        let presence_hz = 1_850.0 + presence * 6_800.0;
        self.presence_filter
            .set_cutoff(self.sample_rate, presence_hz);
        let presence_low = self.presence_filter.process(loop_driver);
        let presence_shaped =
            presence_low + (loop_driver - presence_low) * (0.30 + presence * 0.92);

        let excursion_low = self.excursion_filter.process(presence_shaped);
        let excursion_shaped = presence_shaped + excursion_low * (excursion * 0.48)
            - presence_shaped * excursion * 0.06;

        let pi_drive = excursion_shaped * (1.66 + mode * 0.32);
        let phase_a = triode_stage(pi_drive * 1.16, 0.020);
        let phase_b = triode_stage(-pi_drive * 1.12, -0.018);
        let differential = (phase_a - phase_b) * 0.5;

        let current_demand = (differential.abs() * (1.35 + mode * 0.28) - 0.44).max(0.0);
        let bias_shift = self.bias_envelope.process(current_demand);
        let sag = self
            .supply_sag
            .process(current_demand * current_demand * (0.72 + excursion * 0.50));
        let kt_drive = differential * (2.55 + mode * 0.42)
            / (1.0 + bias_shift * 0.18 + sag * (0.06 + excursion * 0.16));
        let positive_bank = six_l6_bank(kt_drive - bias_shift * 0.018);
        let negative_bank = six_l6_bank(-kt_drive - bias_shift * 0.016);
        let power_output = (positive_bank - negative_bank) * (0.92 + mode * 0.08);

        let mut transformer = self.transformer_highpass.process(power_output);
        transformer = self.transformer_lowpass.process(transformer);
        transformer * master * 0.062 * (1.0 - excursion * 0.05)
    }
}
