use super::common::*;

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GodessOneMode {
    #[default]
    Standard,
    Custom,
}

#[derive(Clone, Copy, Debug)]
pub struct GodessOneControls {
    pub distortion: f32,
    pub tone: f32,
    pub level: f32,
    pub mode: GodessOneMode,
}

impl Default for GodessOneControls {
    fn default() -> Self {
        Self {
            distortion: 0.55,
            tone: 0.52,
            level: 0.58,
            mode: GodessOneMode::Standard,
        }
    }
}

pub struct GodessOne {
    input_connection: ConnectionState,
    input_highpass: OnePoleHighpass,
    pre_emphasis: OnePoleHighpass,
    preclip_lowpass: OnePoleLowpass,
    postclip_lowpass: OnePoleLowpass,
    body_lowpass: OnePoleLowpass,
    edge_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
}

impl GodessOne {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 220e-12),
            input_highpass: OnePoleHighpass::new(sample_rate, 35.0),
            pre_emphasis: OnePoleHighpass::new(sample_rate, 720.0),
            preclip_lowpass: OnePoleLowpass::new(sample_rate, 6_800.0),
            postclip_lowpass: OnePoleLowpass::new(sample_rate, 4_800.0),
            body_lowpass: OnePoleLowpass::new(sample_rate, 620.0),
            edge_highpass: OnePoleHighpass::new(sample_rate, 1_150.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 14_000.0),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_highpass.reset();
        self.pre_emphasis.reset();
        self.preclip_lowpass.reset();
        self.postclip_lowpass.reset();
        self.body_lowpass.reset();
        self.edge_highpass.reset();
        self.output_lowpass.reset();
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: GodessOneControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: GodessOneControls,
    ) -> ElectricalSignal {
        let distortion = controls.distortion.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let level = controls.level.clamp(0.0, 1.0);
        let custom = matches!(controls.mode, GodessOneMode::Custom);

        let input = self.input_highpass.process(loaded_input);
        let edge = self.pre_emphasis.process(input) * if custom { 0.46 } else { 0.64 };
        let body = input * if custom { 0.78 } else { 0.50 };
        let voiced_input = body + edge;
        let filtered = self.preclip_lowpass.process(voiced_input);

        let drive_gain = if custom {
            4.0 + distortion * 58.0
        } else {
            3.2 + distortion * 44.0
        };
        let knee = if custom { 0.58 } else { 0.48 };
        let clipped = diode_pair_clip(filtered * drive_gain, knee);
        let clipped = clipped + hard_clip(filtered * drive_gain * 0.42, knee * 1.08) * 0.32;
        let clipped = self.postclip_lowpass.process(clipped);

        let low = self.body_lowpass.process(clipped);
        let high = self.edge_highpass.process(clipped);
        let mid_fill = clipped * if custom { 0.34 } else { 0.12 };
        let low_weight = if custom {
            1.22 - tone * 0.46
        } else {
            0.92 - tone * 0.58
        };
        let high_weight = if custom {
            0.22 + tone * 0.92
        } else {
            0.42 + tone * 1.42
        };
        let voiced = low * low_weight + high * high_weight + mid_fill;
        let output_gain = if custom {
            0.16 + level * 2.05
        } else {
            0.13 + level * 1.82
        };
        let output = self
            .output_lowpass
            .process(voiced * output_gain)
            .clamp(-3.8, 3.8);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
