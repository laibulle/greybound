use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct BrigadeControls {
    pub time_ms: f32,
    pub repeats: f32,
    pub tone: f32,
    pub mix: f32,
}

impl Default for BrigadeControls {
    fn default() -> Self {
        Self {
            time_ms: 320.0,
            repeats: 0.38,
            tone: 0.42,
            mix: 0.30,
        }
    }
}

pub struct Brigade {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    pre_delay_lowpass: OnePoleLowpass,
    repeat_lowpass: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    delay: FractionalDelayLine,
    sample_rate: f32,
    feedback_state: f32,
}

impl Brigade {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 170e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 35.0),
            pre_delay_lowpass: OnePoleLowpass::new(sample_rate, 5_800.0),
            repeat_lowpass: OnePoleLowpass::new(sample_rate, 2_900.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 10_500.0),
            delay: FractionalDelayLine::new(sample_rate, 0.85),
            sample_rate,
            feedback_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.pre_delay_lowpass.reset();
        self.repeat_lowpass.reset();
        self.output_lowpass.reset();
        self.delay.reset();
        self.feedback_state = 0.0;
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: BrigadeControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: BrigadeControls,
    ) -> ElectricalSignal {
        let time_ms = controls.time_ms.clamp(60.0, 700.0);
        let repeats = controls.repeats.clamp(0.0, 0.92);
        let tone = controls.tone.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        let input = self.input_coupling.process(loaded_input);
        let delay_samples = time_ms * 0.001 * self.sample_rate;
        let feedback_gain = repeats * 0.86;
        let drive = (input + self.feedback_state * feedback_gain).clamp(-18.0, 18.0);
        let bbd_input = self.pre_delay_lowpass.process(drive);
        let delayed = self.delay.process(bbd_input, delay_samples);

        let compressed = (delayed * (1.0 + repeats * 0.45)).tanh();
        let dark_repeat = self.repeat_lowpass.process(compressed);
        let wet = dark_repeat * (1.0 - tone * 0.50) + compressed * tone * 0.50;
        self.feedback_state = wet.clamp(-10.0, 10.0);

        let dry_gain = 1.0 - mix * 0.18;
        let wet_gain = mix * (0.82 + repeats * 0.16);
        let output = self
            .output_lowpass
            .process(input * dry_gain + wet * wet_gain)
            .clamp(-32.0, 32.0);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
