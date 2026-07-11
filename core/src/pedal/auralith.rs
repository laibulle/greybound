use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct AuralithControls {
    pub decay: f32,
    pub size: f32,
    pub texture: f32,
    pub tone: f32,
    pub low_cut: f32,
    pub mix: f32,
}

impl Default for AuralithControls {
    fn default() -> Self {
        Self {
            decay: 0.52,
            size: 0.55,
            texture: 0.68,
            tone: 0.55,
            low_cut: 0.32,
            mix: 0.24,
        }
    }
}

pub struct Auralith {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    low_cut_highpass: OnePoleHighpass,
    pre_delay: FractionalDelayLine,
    early_delays: [SpringDelay; 6],
    tank_delays: [FractionalDelayLine; 8],
    damping_filters: [OnePoleLowpass; 8],
    tone_lowpass: OnePoleLowpass,
    tone_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
    feedback_inputs: [f32; 8],
    modulation_phases: [f32; 8],
    sample_rate: f32,
}

impl Auralith {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 120e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 18.0),
            low_cut_highpass: OnePoleHighpass::new(sample_rate, 145.0),
            pre_delay: FractionalDelayLine::new(sample_rate, 0.08),
            early_delays: [
                SpringDelay::new(sample_rate, 0.0047),
                SpringDelay::new(sample_rate, 0.0079),
                SpringDelay::new(sample_rate, 0.0113),
                SpringDelay::new(sample_rate, 0.0169),
                SpringDelay::new(sample_rate, 0.0231),
                SpringDelay::new(sample_rate, 0.0317),
            ],
            tank_delays: std::array::from_fn(|_| FractionalDelayLine::new(sample_rate, 0.34)),
            damping_filters: std::array::from_fn(|_| OnePoleLowpass::new(sample_rate, 5_800.0)),
            tone_lowpass: OnePoleLowpass::new(sample_rate, 6_400.0),
            tone_highpass: OnePoleHighpass::new(sample_rate, 2_100.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 15_000.0),
            feedback_inputs: [0.0; 8],
            modulation_phases: [0.03, 0.17, 0.29, 0.41, 0.58, 0.67, 0.79, 0.91],
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.low_cut_highpass.reset();
        self.pre_delay.reset();
        for delay in &mut self.early_delays {
            delay.reset();
        }
        for delay in &mut self.tank_delays {
            delay.reset();
        }
        for filter in &mut self.damping_filters {
            filter.reset();
        }
        self.tone_lowpass.reset();
        self.tone_highpass.reset();
        self.output_lowpass.reset();
        self.feedback_inputs = [0.0; 8];
        self.modulation_phases = [0.03, 0.17, 0.29, 0.41, 0.58, 0.67, 0.79, 0.91];
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: AuralithControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: AuralithControls,
    ) -> ElectricalSignal {
        let decay = controls.decay.clamp(0.0, 1.0);
        let size = controls.size.clamp(0.0, 1.0);
        let texture = controls.texture.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let low_cut = controls.low_cut.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        let dry = loaded_input;
        let coupled = self.input_coupling.process(loaded_input);
        let filtered_input =
            coupled * (1.0 - low_cut * 0.50) + self.low_cut_highpass.process(coupled) * low_cut;
        let pre_delay_ms = 4.0 + size * 18.0 + texture * 6.0;
        let predelayed = self
            .pre_delay
            .process(filtered_input, pre_delay_ms * 0.001 * self.sample_rate);

        let excited = (predelayed * (0.82 + texture * 0.72)).tanh() * (0.78 + texture * 0.28);
        let e0 = self.early_delays[0].process(excited * 0.54);
        let e1 = self.early_delays[1].process(-excited * 0.43 + e0 * 0.18);
        let e2 = self.early_delays[2].process(excited * 0.36 - e1 * 0.15);
        let e3 = self.early_delays[3].process(-excited * 0.30 + e2 * 0.13);
        let e4 = self.early_delays[4].process(excited * 0.25 - e3 * 0.11);
        let e5 = self.early_delays[5].process(-excited * 0.21 + e4 * 0.10);
        let early = e0 * 0.25 - e1 * 0.21 + e2 * 0.18 - e3 * 0.15 + e4 * 0.12 - e5 * 0.10;

        let base_delays = [
            0.0311, 0.0379, 0.0437, 0.0523, 0.0611, 0.0733, 0.0839, 0.0971,
        ];
        let delay_scale = 0.78 + size * 1.12;
        let input_gain = 0.14 + texture * 0.15;
        let modulation_hz = 0.08 + texture * 0.16;
        let modulation_depth = 0.45 + size * 3.2 + texture * 1.1;
        let mut outs = [0.0; 8];
        for idx in 0..8 {
            self.modulation_phases[idx] = (self.modulation_phases[idx]
                + modulation_hz * (0.73 + idx as f32 * 0.061) / self.sample_rate)
                .fract();
            let lfo = (std::f32::consts::TAU * self.modulation_phases[idx]).sin();
            let delay_samples =
                base_delays[idx] * delay_scale * self.sample_rate + lfo * modulation_depth;
            let injection = excited * input_gain * if idx % 2 == 0 { 1.0 } else { -0.86 };
            outs[idx] =
                self.tank_delays[idx].process(injection + self.feedback_inputs[idx], delay_samples);
        }

        let damped: [f32; 8] = std::array::from_fn(|idx| {
            let cutoff_sample = self.damping_filters[idx].process(outs[idx]);
            let grain = (outs[idx] * (1.0 + texture * 0.55)).tanh();
            cutoff_sample * (0.70 + tone * 0.20)
                + outs[idx] * (0.07 + tone * 0.22)
                + grain * texture * 0.05
        });
        let mixed = hadamard8(damped);
        let feedback_gain = (0.50 + decay * 0.39 + size * 0.03).clamp(0.0, 0.92);
        for idx in 0..8 {
            self.feedback_inputs[idx] = (mixed[idx] * feedback_gain).clamp(-3.2, 3.2);
        }

        let tank_sum = outs[0] * 0.16 - outs[1] * 0.15 + outs[2] * 0.14 - outs[3] * 0.13
            + outs[4] * 0.12
            - outs[5] * 0.11
            + outs[6] * 0.10
            - outs[7] * 0.09;
        let raw_wet = early * (0.32 + texture * 0.18) + tank_sum * (0.90 + size * 0.16);
        let dark = self.tone_lowpass.process(raw_wet);
        let bright = self.tone_highpass.process(raw_wet);
        let voiced = dark * (1.10 - tone * 0.46) + bright * (0.08 + tone * 0.50);
        let wet = self.output_lowpass.process(voiced).clamp(-3.0, 3.0);
        let wet_gain = 1.52 + decay * 0.24;
        let output = dry * (1.0 - mix * 0.10) + wet * mix * wet_gain;

        ElectricalSignal::new(output.clamp(-32.0, 32.0), Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
