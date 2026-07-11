use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct JetstreamControls {
    pub manual: f32,
    pub rate_hz: f32,
    pub depth: f32,
    pub feedback: f32,
    pub mix: f32,
}

impl Default for JetstreamControls {
    fn default() -> Self {
        Self {
            manual: 0.42,
            rate_hz: 0.28,
            depth: 0.68,
            feedback: 0.46,
            mix: 0.56,
        }
    }
}

pub struct Jetstream {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    pre_delay_lowpass: OnePoleLowpass,
    post_delay_lowpass: OnePoleLowpass,
    delay: FractionalDelayLine,
    sample_rate: f32,
    phase: f32,
    feedback_state: f32,
}

impl Jetstream {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 160e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 24.0),
            pre_delay_lowpass: OnePoleLowpass::new(sample_rate, 7_200.0),
            post_delay_lowpass: OnePoleLowpass::new(sample_rate, 6_600.0),
            delay: FractionalDelayLine::new(sample_rate, 0.018),
            sample_rate,
            phase: 0.0,
            feedback_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.pre_delay_lowpass.reset();
        self.post_delay_lowpass.reset();
        self.delay.reset();
        self.phase = 0.0;
        self.feedback_state = 0.0;
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: JetstreamControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: JetstreamControls,
    ) -> ElectricalSignal {
        let manual = controls.manual.clamp(0.0, 1.0);
        let rate_hz = controls.rate_hz.clamp(0.02, 8.0);
        let depth = controls.depth.clamp(0.0, 1.0);
        let feedback = controls.feedback.clamp(0.0, 0.94);
        let mix = controls.mix.clamp(0.0, 1.0);

        let phase_radians = self.phase * std::f32::consts::TAU;
        let lfo = phase_radians.sin() * 0.5 + 0.5;
        self.phase = (self.phase + rate_hz / self.sample_rate).fract();

        let base_ms = 0.55 + manual * 5.2;
        let sweep_ms = depth * (0.35 + manual * 3.8);
        let delay_ms = (base_ms + (lfo - 0.5) * sweep_ms * 2.0).clamp(0.25, 9.5);
        let delay_samples = delay_ms * 0.001 * self.sample_rate;

        let input = self.input_coupling.process(loaded_input);
        let bbd_input = self
            .pre_delay_lowpass
            .process((input + self.feedback_state * feedback * 0.72).clamp(-18.0, 18.0));
        let delayed = self.delay.process(bbd_input, delay_samples);
        let wet = self.post_delay_lowpass.process(delayed);
        self.feedback_state = wet.clamp(-12.0, 12.0);

        let comb = input * (1.0 - mix * 0.20) + wet * mix * (0.82 + feedback * 0.18);
        let output = comb.clamp(-32.0, 32.0);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
