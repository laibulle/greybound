use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct TronControls {
    pub rate_hz: f32,
    pub depth: f32,
    pub feedback: f32,
    pub mix: f32,
}

impl Default for TronControls {
    fn default() -> Self {
        Self {
            rate_hz: 0.65,
            depth: 0.68,
            feedback: 0.34,
            mix: 0.50,
        }
    }
}

pub struct Tron {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    lamp_smoother: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    stages: [AllPassStage; 6],
    sample_rate: f32,
    phase: f32,
    feedback_state: f32,
}

impl Tron {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 470_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 180e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 18.0),
            lamp_smoother: OnePoleLowpass::new(sample_rate, 5.5),
            output_lowpass: OnePoleLowpass::new(sample_rate, 15_000.0),
            stages: [AllPassStage::default(); 6],
            sample_rate,
            phase: 0.0,
            feedback_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.lamp_smoother.reset();
        self.output_lowpass.reset();
        for stage in &mut self.stages {
            stage.reset();
        }
        self.phase = 0.0;
        self.feedback_state = 0.0;
    }

    pub fn process(&mut self, input: ElectricalSignal, controls: TronControls) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: TronControls,
    ) -> ElectricalSignal {
        let rate_hz = controls.rate_hz.clamp(0.03, 12.0);
        let depth = controls.depth.clamp(0.0, 1.0);
        let feedback = controls.feedback.clamp(0.0, 0.92);
        let mix = controls.mix.clamp(0.0, 1.0);

        let phase_radians = self.phase * std::f32::consts::TAU;
        let lamp_drive = (phase_radians.sin() * 0.5 + 0.5).powf(1.35);
        self.phase = (self.phase + rate_hz / self.sample_rate).fract();

        let lamp = self.lamp_smoother.process(lamp_drive);
        let sweep = (1.0 - depth) * 0.38 + depth * lamp;
        let center_hz = 70.0 * (6_200.0_f32 / 70.0).powf(sweep.clamp(0.0, 1.0));
        let stage_spreads = [0.56, 0.74, 0.98, 1.28, 1.70, 2.25];

        let input = self.input_coupling.process(loaded_input);
        let mut shifted = input + self.feedback_state * feedback * 0.42;
        for (stage, spread) in self.stages.iter_mut().zip(stage_spreads) {
            shifted = stage.process(
                shifted,
                allpass_coefficient(center_hz * spread, self.sample_rate),
            );
        }
        self.feedback_state = shifted.clamp(-8.0, 8.0);

        let phase_mix = mix * (0.72 + depth * 0.28);
        let notched =
            input * (1.0 - phase_mix * 0.48) + shifted * phase_mix * (0.72 + feedback * 0.18);
        let level = 1.14 + feedback * 0.08;
        let output = self
            .output_lowpass
            .process(notched * level)
            .clamp(-32.0, 32.0);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
