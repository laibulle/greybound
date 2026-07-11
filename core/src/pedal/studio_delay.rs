use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct StudioDelayControls {
    pub time_ms: f32,
    pub feedback: f32,
    pub tone: f32,
    pub mod_depth: f32,
    pub mix: f32,
}

impl Default for StudioDelayControls {
    fn default() -> Self {
        Self {
            time_ms: 360.0,
            feedback: 0.34,
            tone: 0.58,
            mod_depth: 0.08,
            mix: 0.18,
        }
    }
}

pub struct StudioDelay {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    delay: FractionalDelayLine,
    feedback_lowpass: OnePoleLowpass,
    feedback_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
    feedback_sample: f32,
    modulation_phase: f32,
    sample_rate: f32,
}

impl StudioDelay {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 120e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 18.0),
            delay: FractionalDelayLine::new(sample_rate, 2.2),
            feedback_lowpass: OnePoleLowpass::new(sample_rate, 5_800.0),
            feedback_highpass: OnePoleHighpass::new(sample_rate, 85.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 16_000.0),
            feedback_sample: 0.0,
            modulation_phase: 0.0,
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.delay.reset();
        self.feedback_lowpass.reset();
        self.feedback_highpass.reset();
        self.output_lowpass.reset();
        self.feedback_sample = 0.0;
        self.modulation_phase = 0.0;
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: StudioDelayControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: StudioDelayControls,
    ) -> ElectricalSignal {
        let time_ms = controls.time_ms.clamp(40.0, 1_200.0);
        let feedback = controls.feedback.clamp(0.0, 0.92);
        let tone = controls.tone.clamp(0.0, 1.0);
        let mod_depth = controls.mod_depth.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        self.modulation_phase = (self.modulation_phase + 0.17 / self.sample_rate).rem_euclid(1.0);
        let lfo = (std::f32::consts::TAU * self.modulation_phase).sin();
        let delay_samples = (time_ms * 0.001 * self.sample_rate) + lfo * mod_depth * 9.0;

        let coupled = self.input_coupling.process(loaded_input);
        let delay_input = coupled + self.feedback_sample * feedback;
        let delayed = self
            .delay
            .process(delay_input.clamp(-4.0, 4.0), delay_samples);

        let dark = self.feedback_lowpass.process(delayed);
        let bright = delayed;
        let voiced = dark * (1.18 - tone * 0.58) + bright * (0.16 + tone * 0.42);
        let voiced = self.feedback_highpass.process(voiced);
        self.feedback_sample = voiced.clamp(-4.0, 4.0);

        let wet = self.output_lowpass.process(voiced).clamp(-4.0, 4.0);
        let output = loaded_input * (1.0 - mix * 0.08) + wet * mix * 1.18;
        ElectricalSignal::new(output.clamp(-32.0, 32.0), Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
