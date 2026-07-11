use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct MonarchControls {
    pub gain: f32,
    pub tone: f32,
    pub output: f32,
}

impl Default for MonarchControls {
    fn default() -> Self {
        Self {
            gain: 0.45,
            tone: 0.52,
            output: 0.58,
        }
    }
}

pub struct Monarch {
    input_connection: ConnectionState,
    input_highpass: OnePoleHighpass,
    preclip_lowpass: OnePoleLowpass,
    first_stage_lowpass: OnePoleLowpass,
    second_stage_lowpass: OnePoleLowpass,
    tone_lowpass: OnePoleLowpass,
    tone_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
}

impl Monarch {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 4_700.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 330e-12),
            input_highpass: OnePoleHighpass::new(sample_rate, 28.0),
            preclip_lowpass: OnePoleLowpass::new(sample_rate, 8_200.0),
            first_stage_lowpass: OnePoleLowpass::new(sample_rate, 4_500.0),
            second_stage_lowpass: OnePoleLowpass::new(sample_rate, 5_800.0),
            tone_lowpass: OnePoleLowpass::new(sample_rate, 680.0),
            tone_highpass: OnePoleHighpass::new(sample_rate, 1_450.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 13_500.0),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_highpass.reset();
        self.preclip_lowpass.reset();
        self.first_stage_lowpass.reset();
        self.second_stage_lowpass.reset();
        self.tone_lowpass.reset();
        self.tone_highpass.reset();
        self.output_lowpass.reset();
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: MonarchControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: MonarchControls,
    ) -> ElectricalSignal {
        let gain = controls.gain.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let output = controls.output.clamp(0.0, 1.0);

        let input = self.input_highpass.process(loaded_input);
        let filtered = self.preclip_lowpass.process(input);
        let drive = 1.4 + gain * 22.0;
        let first_clip = asymmetric_diode_clip(filtered * drive, 0.50, 0.74);
        let first_stage = self.first_stage_lowpass.process(first_clip) * (0.32 + gain * 0.82);
        let second_drive = first_stage * (1.15 + gain * 7.5);
        let second_clip = asymmetric_diode_clip(second_drive, 0.56, 0.86);
        let second_stage = self.second_stage_lowpass.process(second_clip);

        let clean_blend = input * (0.38 - gain * 0.22).max(0.06);
        let mixed = clean_blend + second_stage * (0.58 + gain * 0.72);
        let low = self.tone_lowpass.process(mixed);
        let high = self.tone_highpass.process(mixed);
        let voiced = low * (1.15 - tone * 0.55) + high * (0.18 + tone * 1.05);
        let level = 0.20 + output * 2.25;
        let final_output = self.output_lowpass.process(voiced * level).clamp(-3.5, 3.5);

        ElectricalSignal::new(final_output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
