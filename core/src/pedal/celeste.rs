use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct CelesteControls {
    pub rate_hz: f32,
    pub depth: f32,
    pub tone: f32,
    pub mix: f32,
}

impl Default for CelesteControls {
    fn default() -> Self {
        Self {
            rate_hz: 0.62,
            depth: 0.58,
            tone: 0.55,
            mix: 0.42,
        }
    }
}

pub struct Celeste {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    pre_delay_lowpass: OnePoleLowpass,
    wet_lowpass: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    delay_a: FractionalDelayLine,
    delay_b: FractionalDelayLine,
    sample_rate: f32,
    phase: f32,
}

impl Celeste {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 150e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 28.0),
            pre_delay_lowpass: OnePoleLowpass::new(sample_rate, 7_800.0),
            wet_lowpass: OnePoleLowpass::new(sample_rate, 3_600.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 12_000.0),
            delay_a: FractionalDelayLine::new(sample_rate, 0.035),
            delay_b: FractionalDelayLine::new(sample_rate, 0.035),
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.pre_delay_lowpass.reset();
        self.wet_lowpass.reset();
        self.output_lowpass.reset();
        self.delay_a.reset();
        self.delay_b.reset();
        self.phase = 0.0;
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: CelesteControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: CelesteControls,
    ) -> ElectricalSignal {
        let rate_hz = controls.rate_hz.clamp(0.05, 6.0);
        let depth = controls.depth.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        let phase_radians = self.phase * std::f32::consts::TAU;
        let lfo_a = phase_radians.sin();
        let lfo_b = (phase_radians + std::f32::consts::TAU * 0.37).sin();
        self.phase = (self.phase + rate_hz / self.sample_rate).fract();

        let base_ms = 14.0;
        let sweep_ms = 1.4 + depth * 7.8;
        let delay_a_ms = (base_ms + lfo_a * sweep_ms).clamp(5.0, 27.0);
        let delay_b_ms = (base_ms * 1.17 + lfo_b * sweep_ms * 0.82).clamp(6.0, 31.0);

        let input = self.input_coupling.process(loaded_input);
        let bbd_input = self.pre_delay_lowpass.process(input).clamp(-18.0, 18.0);
        let voice_a = self
            .delay_a
            .process(bbd_input, delay_a_ms * 0.001 * self.sample_rate);
        let voice_b = self
            .delay_b
            .process(bbd_input, delay_b_ms * 0.001 * self.sample_rate);
        let wet_raw = (voice_a + voice_b) * 0.5;
        let wet_dark = self.wet_lowpass.process(wet_raw);
        let wet = wet_dark * (1.0 - tone * 0.55) + wet_raw * tone * 0.55;

        let dry_gain = 1.0 - mix * 0.28;
        let wet_gain = mix * (0.78 + depth * 0.18);
        let output = self
            .output_lowpass
            .process(input * dry_gain + wet * wet_gain)
            .clamp(-32.0, 32.0);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
