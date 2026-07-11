use super::common::*;

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudioVerbAlgorithm {
    #[default]
    Room,
    Plate,
}

#[derive(Clone, Copy, Debug)]
pub struct StudioVerbControls {
    pub algorithm: StudioVerbAlgorithm,
    pub decay: f32,
    pub size: f32,
    pub pre_delay_ms: f32,
    pub diffusion: f32,
    pub tone: f32,
    pub low_cut: f32,
    pub mod_depth: f32,
    pub mix: f32,
}

impl Default for StudioVerbControls {
    fn default() -> Self {
        Self {
            algorithm: StudioVerbAlgorithm::Room,
            decay: 0.42,
            size: 0.46,
            pre_delay_ms: 12.0,
            diffusion: 0.64,
            tone: 0.54,
            low_cut: 0.36,
            mod_depth: 0.18,
            mix: 0.24,
        }
    }
}

pub struct StudioVerb {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    low_cut_highpass: OnePoleHighpass,
    pre_delay: FractionalDelayLine,
    early_delays: [SpringDelay; 4],
    fdn_delays: [FractionalDelayLine; 8],
    fdn_damping: [OnePoleLowpass; 8],
    tone_lowpass: OnePoleLowpass,
    tone_highpass: OnePoleHighpass,
    output_lowpass: OnePoleLowpass,
    feedback_inputs: [f32; 8],
    modulation_phases: [f32; 8],
    sample_rate: f32,
}

impl StudioVerb {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 120e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 22.0),
            low_cut_highpass: OnePoleHighpass::new(sample_rate, 135.0),
            pre_delay: FractionalDelayLine::new(sample_rate, 0.14),
            early_delays: [
                SpringDelay::new(sample_rate, 0.006),
                SpringDelay::new(sample_rate, 0.011),
                SpringDelay::new(sample_rate, 0.017),
                SpringDelay::new(sample_rate, 0.023),
            ],
            fdn_delays: std::array::from_fn(|_| FractionalDelayLine::new(sample_rate, 0.25)),
            fdn_damping: std::array::from_fn(|_| OnePoleLowpass::new(sample_rate, 6_200.0)),
            tone_lowpass: OnePoleLowpass::new(sample_rate, 5_800.0),
            tone_highpass: OnePoleHighpass::new(sample_rate, 1_900.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 14_000.0),
            feedback_inputs: [0.0; 8],
            modulation_phases: [0.0, 0.19, 0.37, 0.53, 0.71, 0.83, 0.91, 0.97],
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
        for delay in &mut self.fdn_delays {
            delay.reset();
        }
        for damping in &mut self.fdn_damping {
            damping.reset();
        }
        self.tone_lowpass.reset();
        self.tone_highpass.reset();
        self.output_lowpass.reset();
        self.feedback_inputs = [0.0; 8];
        self.modulation_phases = [0.0, 0.19, 0.37, 0.53, 0.71, 0.83, 0.91, 0.97];
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: StudioVerbControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: StudioVerbControls,
    ) -> ElectricalSignal {
        let decay = controls.decay.clamp(0.0, 1.0);
        let size = controls.size.clamp(0.0, 1.0);
        let diffusion = controls.diffusion.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let low_cut = controls.low_cut.clamp(0.0, 1.0);
        let mod_depth = controls.mod_depth.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);
        let pre_delay_ms = controls.pre_delay_ms.clamp(0.0, 120.0);

        let dry = loaded_input;
        let coupled = self.input_coupling.process(loaded_input);
        let low_cut_input =
            coupled * (1.0 - low_cut * 0.55) + self.low_cut_highpass.process(coupled) * low_cut;
        let predelayed = self
            .pre_delay
            .process(low_cut_input, pre_delay_ms * 0.001 * self.sample_rate);

        let algo = match controls.algorithm {
            StudioVerbAlgorithm::Room => StudioVerbTuning {
                delay_scale: 0.64 + size * 0.55,
                feedback_gain: 0.42 + decay * 0.32,
                input_gain: 0.20 + diffusion * 0.14,
                early_gain: 0.24 + diffusion * 0.24,
                output_gain: 0.74,
                modulation_hz: 0.19,
            },
            StudioVerbAlgorithm::Plate => StudioVerbTuning {
                delay_scale: 0.88 + size * 0.82,
                feedback_gain: 0.56 + decay * 0.35,
                input_gain: 0.16 + diffusion * 0.12,
                early_gain: 0.10 + diffusion * 0.14,
                output_gain: 0.82,
                modulation_hz: 0.12,
            },
        };

        let e0 = self.early_delays[0].process(predelayed * 0.58);
        let e1 = self.early_delays[1].process(-predelayed * 0.44 + e0 * 0.18);
        let e2 = self.early_delays[2].process(predelayed * 0.36 - e1 * 0.14);
        let e3 = self.early_delays[3].process(-predelayed * 0.30 + e2 * 0.12);
        let early = e0 * 0.36 - e1 * 0.28 + e2 * 0.22 - e3 * 0.18;

        let base_delays = [
            0.0297, 0.0371, 0.0419, 0.0533, 0.0617, 0.0719, 0.0839, 0.0973,
        ];
        let mut outs = [0.0; 8];
        for idx in 0..8 {
            self.modulation_phases[idx] = (self.modulation_phases[idx]
                + algo.modulation_hz * (0.71 + idx as f32 * 0.07) / self.sample_rate)
                .fract();
            let lfo = (std::f32::consts::TAU * self.modulation_phases[idx]).sin();
            let mod_samples = lfo * mod_depth * (1.0 + size * 3.5);
            let delay_samples =
                base_delays[idx] * algo.delay_scale * self.sample_rate + mod_samples;
            outs[idx] = self.fdn_delays[idx].process(
                predelayed * algo.input_gain + self.feedback_inputs[idx],
                delay_samples,
            );
        }

        let damped: [f32; 8] = std::array::from_fn(|idx| {
            let damp = self.fdn_damping[idx].process(outs[idx]);
            damp * (0.62 + tone * 0.34) + outs[idx] * (0.14 + tone * 0.24)
        });
        let mixed = hadamard8(damped);
        let feedback_gain = algo.feedback_gain.clamp(0.0, 0.93);
        for idx in 0..8 {
            self.feedback_inputs[idx] = (mixed[idx] * feedback_gain).clamp(-4.0, 4.0);
        }

        let tank_sum = outs[0] * 0.18 - outs[1] * 0.16 + outs[2] * 0.15 - outs[3] * 0.13
            + outs[4] * 0.12
            - outs[5] * 0.11
            + outs[6] * 0.10
            - outs[7] * 0.09;
        let raw_wet = early * algo.early_gain + tank_sum * algo.output_gain;
        let dark = self.tone_lowpass.process(raw_wet);
        let bright = self.tone_highpass.process(raw_wet);
        let voiced = dark * (1.12 - tone * 0.54) + bright * (0.12 + tone * 0.58);
        let wet = self.output_lowpass.process(voiced).clamp(-4.0, 4.0);
        let wet_gain = match controls.algorithm {
            StudioVerbAlgorithm::Room => 1.65,
            StudioVerbAlgorithm::Plate => 1.45,
        };
        let output = dry * (1.0 - mix * 0.12) + wet * mix * wet_gain;

        ElectricalSignal::new(output.clamp(-32.0, 32.0), Self::OUTPUT_IMPEDANCE_OHMS)
    }
}

struct StudioVerbTuning {
    delay_scale: f32,
    feedback_gain: f32,
    input_gain: f32,
    early_gain: f32,
    output_gain: f32,
    modulation_hz: f32,
}
