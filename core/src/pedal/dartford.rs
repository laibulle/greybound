use super::common::*;

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DartfordWave {
    #[default]
    Sine,
    Triangle,
    Square,
}

#[derive(Clone, Copy, Debug)]
pub struct DartfordControls {
    pub rate_hz: f32,
    pub depth: f32,
    pub level: f32,
    pub wave: DartfordWave,
}

impl Default for DartfordControls {
    fn default() -> Self {
        Self {
            rate_hz: 4.5,
            depth: 0.55,
            level: 1.0,
            wave: DartfordWave::Sine,
        }
    }
}

pub struct Dartford {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    low_band: OnePoleLowpass,
    high_band: OnePoleHighpass,
    lfo_smoother: OnePoleLowpass,
    bias_memory: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    sample_rate: f32,
    phase: f32,
}

impl Dartford {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 120e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 16.0),
            low_band: OnePoleLowpass::new(sample_rate, 690.0),
            high_band: OnePoleHighpass::new(sample_rate, 690.0),
            lfo_smoother: OnePoleLowpass::new(sample_rate, 18.0),
            bias_memory: OnePoleLowpass::new(sample_rate, 8.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 16_500.0),
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.low_band.reset();
        self.high_band.reset();
        self.lfo_smoother.reset();
        self.bias_memory.reset();
        self.output_lowpass.reset();
        self.phase = 0.0;
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: DartfordControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: DartfordControls,
    ) -> ElectricalSignal {
        let rate_hz = controls.rate_hz.clamp(0.05, 20.0);
        let depth = controls.depth.clamp(0.0, 1.0);
        let intensity = depth.powf(0.75);
        let level = controls.level.clamp(0.0, 2.0);

        let phase_radians = self.phase * std::f32::consts::TAU;
        let raw_lfo = match controls.wave {
            DartfordWave::Sine => phase_radians.sin(),
            DartfordWave::Triangle => {
                if self.phase < 0.5 {
                    self.phase * 4.0 - 1.0
                } else {
                    3.0 - self.phase * 4.0
                }
            }
            DartfordWave::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
        };
        let quadrature_lfo = (phase_radians + std::f32::consts::FRAC_PI_2).sin();
        self.phase = (self.phase + rate_hz / self.sample_rate).fract();

        let asymmetric_lfo = (raw_lfo + 0.20 * raw_lfo * raw_lfo - 0.10).clamp(-1.0, 1.0);
        let smoothed_lfo = self.lfo_smoother.process(asymmetric_lfo);
        let pulse = ((smoothed_lfo + 1.0) * 0.5).clamp(0.0, 1.0);
        let tremolo_gain = 1.0 - intensity * 0.72 * (1.0 - pulse).powf(1.25);
        let makeup_gain = 1.0 + intensity * 0.03;

        let coupled_input = self.input_coupling.process(loaded_input);
        let low = self.low_band.process(coupled_input);
        let high = self.high_band.process(coupled_input);
        let low_motion = 1.0 - intensity * 0.035 * quadrature_lfo;
        let high_motion = 1.0 + intensity * 0.05 * quadrature_lfo;
        let voiced = low * low_motion + high * high_motion;

        let bias_memory = self
            .bias_memory
            .process((1.0 - tremolo_gain) * voiced.abs());
        let bias_compression = 1.0 / (1.0 + bias_memory * 0.35);
        let output = self
            .output_lowpass
            .process(voiced * tremolo_gain * makeup_gain * bias_compression * level)
            .clamp(-4.0, 4.0);
        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
