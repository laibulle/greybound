use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct MuffinControls {
    pub sustain: f32,
    pub tone: f32,
    pub level: f32,
}

impl Default for MuffinControls {
    fn default() -> Self {
        Self {
            sustain: 0.55,
            tone: 0.50,
            level: 0.70,
        }
    }
}

pub struct Muffin {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    stage_filters: [OnePoleLowpass; 4],
    tone_lowpass: OnePoleLowpass,
    tone_highpass: OnePoleHighpass,
    output_coupling: OnePoleHighpass,
}

impl Muffin {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 130_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 10_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 470e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 7.0),
            stage_filters: [
                OnePoleLowpass::new(sample_rate, 7_500.0),
                OnePoleLowpass::new(sample_rate, 6_500.0),
                OnePoleLowpass::new(sample_rate, 6_500.0),
                OnePoleLowpass::new(sample_rate, 8_500.0),
            ],
            tone_lowpass: OnePoleLowpass::new(sample_rate, 720.0),
            tone_highpass: OnePoleHighpass::new(sample_rate, 1_250.0),
            output_coupling: OnePoleHighpass::new(sample_rate, 16.0),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        for filter in &mut self.stage_filters {
            filter.reset();
        }
        self.tone_lowpass.reset();
        self.tone_highpass.reset();
        self.output_coupling.reset();
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: MuffinControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: MuffinControls,
    ) -> ElectricalSignal {
        let sustain = controls.sustain.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let level = controls.level.clamp(0.0, 1.0);

        let mut x = self.input_coupling.process(loaded_input);

        x = self.common_emitter_stage(x, 0, 7.5 + sustain * 8.0, false);
        x = self.common_emitter_stage(x, 1, 9.0 + sustain * 30.0, true);
        x = self.common_emitter_stage(x, 2, 9.0 + sustain * 34.0, true);
        x = self.common_emitter_stage(x, 3, 5.0 + level * 11.0, false);

        let low = self.tone_lowpass.process(x);
        let high = self.tone_highpass.process(x);
        let scooped = low * (1.0 - tone) + high * tone;
        let volume = 0.08 + level * 1.65;
        let output = self
            .output_coupling
            .process(scooped * volume)
            .clamp(-4.5, 4.5);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }

    fn common_emitter_stage(
        &mut self,
        input: f32,
        stage_index: usize,
        gain: f32,
        diode_clip: bool,
    ) -> f32 {
        let filtered = self.stage_filters[stage_index].process(input);
        let amplified = -filtered * gain;
        if diode_clip {
            diode_pair_clip(amplified, 0.42)
        } else {
            transistor_limit(amplified)
        }
    }
}
