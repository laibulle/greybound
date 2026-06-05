use super::AmpModel;
use crate::amp::components::{
    cathode_follower, el84_bank, triode_stage, OnePoleLowpass, SupplyNode, TopBoostToneStack,
    WdfHighpass,
};
use crate::amp::AmpControls;

pub(in crate::amp) struct Nox {
    sample_rate: f32,
    input_coupling: WdfHighpass,
    first_cathode_bypass: WdfHighpass,
    bright_filter: OnePoleLowpass,
    tone_stack: TopBoostToneStack,
    phase_inverter_coupling: WdfHighpass,
    cut_filter: OnePoleLowpass,
    transformer_highpass: WdfHighpass,
    transformer_lowpass: OnePoleLowpass,
    preamp_supply: SupplyNode,
    power_supply: SupplyNode,
}

impl Nox {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            input_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            first_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_500.0, 25e-6),
            bright_filter: OnePoleLowpass::new(sample_rate, 2_900.0),
            tone_stack: TopBoostToneStack::new(sample_rate),
            phase_inverter_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            cut_filter: OnePoleLowpass::new(sample_rate, 12_000.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 100_000.0, 47e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 13_000.0),
            preamp_supply: SupplyNode::new(sample_rate, 280.0, 12_000.0, 47e-6),
            power_supply: SupplyNode::new(sample_rate, 320.0, 360.0, 32e-6),
        }
    }
}

impl AmpModel for Nox {
    fn reset(&mut self) {
        self.input_coupling = WdfHighpass::from_rc(self.sample_rate, 1_000_000.0, 47e-9);
        self.first_cathode_bypass = WdfHighpass::from_rc(self.sample_rate, 1_500.0, 25e-6);
        self.bright_filter = OnePoleLowpass::new(self.sample_rate, 2_900.0);
        self.tone_stack = TopBoostToneStack::new(self.sample_rate);
        self.phase_inverter_coupling = WdfHighpass::from_rc(self.sample_rate, 1_000_000.0, 47e-9);
        self.cut_filter = OnePoleLowpass::new(self.sample_rate, 12_000.0);
        self.transformer_highpass = WdfHighpass::from_rc(self.sample_rate, 100_000.0, 47e-9);
        self.transformer_lowpass = OnePoleLowpass::new(self.sample_rate, 13_000.0);
        self.preamp_supply.reset();
        self.power_supply.reset();
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let input = self.input_coupling.process(input);

        let preamp_voltage = self.preamp_supply.normalized();
        let volume = controls.volume * controls.volume;
        let high = input - self.bright_filter.process(input);
        let volume_output = input * volume + high * (1.0 - volume) * 0.18;

        let first_bypass = self.first_cathode_bypass.process(volume_output);
        let first_drive = (volume_output * 4.8 + first_bypass * 0.8) * preamp_voltage;
        let first_stage = triode_stage(first_drive, 0.16 * preamp_voltage);
        let preamp_current = first_stage.abs() * (0.00045 + controls.volume * 0.00035);
        self.preamp_supply.process(preamp_current);

        let follower_drive = cathode_follower(first_stage * preamp_voltage);
        let toned = self
            .tone_stack
            .process(follower_drive, controls.bass, controls.treble);

        let pi_input = self
            .phase_inverter_coupling
            .process(toned * 4.6 * preamp_voltage);
        let phase_a = triode_stage(pi_input * 1.34, 0.040 * preamp_voltage);
        let phase_b = triode_stage(-pi_input * 1.30, -0.032 * preamp_voltage);
        let differential = (phase_a - phase_b) * 0.5;

        let cut_hz = 13_500.0 * (1.0 - controls.cut).powi(2) + 1_150.0;
        self.cut_filter.set_cutoff(self.sample_rate, cut_hz);
        let cut_output = self.cut_filter.process(differential);

        let power_voltage = self.power_supply.normalized();
        let power_drive = cut_output * 1.58 * power_voltage;
        let positive_bank = el84_bank(power_drive);
        let negative_bank = el84_bank(-power_drive);
        let push_pull_current =
            (positive_bank.abs() + negative_bank.abs()) * (0.020 + controls.sag * 0.035);
        let updated_power_voltage = self.power_supply.process(push_pull_current) / 320.0;
        let power_output = (positive_bank - negative_bank) * 0.72 * updated_power_voltage;

        let mut transformer = self.transformer_highpass.process(power_output);
        transformer = self.transformer_lowpass.process(transformer);
        transformer * controls.output
    }
}
