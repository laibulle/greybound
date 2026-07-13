use crate::amp::oversampling::{half_band_coefficients, FirFilter, OVERSAMPLING_FACTOR};
use crate::circuit::muffin::{
    BjtCommonEmitterParams, BjtCommonEmitterStage, MuffinToneStack, SiliconDiodePair,
};

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

/// Ram's Head-era Big Muff topology hypothesis.
///
/// The model is intentionally a component circuit, rather than four scalar
/// gains followed by `tanh`: every transistor has an emitter/capacitor state,
/// the clipping pairs use a bounded Shockley solve, and the tone control is a
/// passive MNA network. Exact production values vary between Muffin revisions;
/// the constants below are the documented Violet/Ram's-Head working set.
pub struct Muffin {
    input_connection: ConnectionState,
    upsampler: FirFilter,
    downsampler: FirFilter,
    input_coupling: OnePoleHighpass,
    q1: BjtCommonEmitterStage,
    q2: BjtCommonEmitterStage,
    q3: BjtCommonEmitterStage,
    q4: BjtCommonEmitterStage,
    q1_to_q2: OnePoleHighpass,
    q2_to_q3: OnePoleHighpass,
    q3_to_tone: OnePoleHighpass,
    tone_to_q4: OnePoleHighpass,
    output_coupling: OnePoleHighpass,
    diodes_a: SiliconDiodePair,
    diodes_b: SiliconDiodePair,
    tone_stack: MuffinToneStack,
}

impl Muffin {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 117_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 25_000.0;
    const CLIPPING_STAGE_INPUT_LOAD_OHMS: f32 = 100_000.0;
    const RECOVERY_STAGE_INPUT_LOAD_OHMS: f32 = 100_000.0;

    pub fn new(sample_rate: f32) -> Self {
        let circuit_sample_rate = sample_rate * OVERSAMPLING_FACTOR;
        // Coupling corners are derived from the named capacitor and the
        // adjacent source/load resistances. They keep DC bias private to each
        // BJT cell while retaining the physical low-frequency recovery.
        Self {
            input_connection: ConnectionState::new(sample_rate, 470e-12),
            upsampler: FirFilter::new(half_band_coefficients()),
            downsampler: FirFilter::new(half_band_coefficients()),
            input_coupling: OnePoleHighpass::new(circuit_sample_rate, 7.2),
            q1: BjtCommonEmitterStage::new(
                circuit_sample_rate,
                BjtCommonEmitterParams {
                    supply_voltage_v: 9.0,
                    collector_resistance_ohms: 39_000.0,
                    emitter_resistance_ohms: 390.0,
                    emitter_bypass_capacitance_f: 10e-6,
                    collector_capacitance_f: 470e-12,
                    quiescent_collector_current_a: 0.12e-3,
                    collector_load_ohms: Self::CLIPPING_STAGE_INPUT_LOAD_OHMS,
                },
            ),
            q2: BjtCommonEmitterStage::new(
                circuit_sample_rate,
                BjtCommonEmitterParams {
                    supply_voltage_v: 9.0,
                    collector_resistance_ohms: 100_000.0,
                    emitter_resistance_ohms: 390.0,
                    emitter_bypass_capacitance_f: 1e-6,
                    // Collector smoothing capacitors are a high-confidence
                    // voicing family in Ram's Head/V3-style Muff circuits.
                    // This 2.2 nF working value is evaluated against the V3 NAM
                    // anchor at the documented noon-tone setting.
                    collector_capacitance_f: 2.2e-9,
                    quiescent_collector_current_a: 45e-6,
                    collector_load_ohms: Self::CLIPPING_STAGE_INPUT_LOAD_OHMS,
                },
            ),
            q3: BjtCommonEmitterStage::new(
                circuit_sample_rate,
                BjtCommonEmitterParams {
                    supply_voltage_v: 9.0,
                    collector_resistance_ohms: 100_000.0,
                    emitter_resistance_ohms: 390.0,
                    emitter_bypass_capacitance_f: 1e-6,
                    collector_capacitance_f: 2.2e-9,
                    quiescent_collector_current_a: 45e-6,
                    collector_load_ohms: 39_000.0,
                },
            ),
            q4: BjtCommonEmitterStage::new(
                circuit_sample_rate,
                BjtCommonEmitterParams {
                    supply_voltage_v: 9.0,
                    collector_resistance_ohms: 39_000.0,
                    emitter_resistance_ohms: 390.0,
                    emitter_bypass_capacitance_f: 1e-6,
                    collector_capacitance_f: 2.2e-9,
                    quiescent_collector_current_a: 0.12e-3,
                    collector_load_ohms: 250_000.0,
                },
            ),
            q1_to_q2: OnePoleHighpass::new(circuit_sample_rate, 3.4),
            q2_to_q3: OnePoleHighpass::new(circuit_sample_rate, 3.4),
            q3_to_tone: OnePoleHighpass::new(circuit_sample_rate, 3.4),
            tone_to_q4: OnePoleHighpass::new(circuit_sample_rate, 4.6),
            output_coupling: OnePoleHighpass::new(circuit_sample_rate, 7.2),
            diodes_a: SiliconDiodePair::one_n4148(),
            diodes_b: SiliconDiodePair::one_n4148(),
            tone_stack: MuffinToneStack::new(circuit_sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.upsampler.reset();
        self.downsampler.reset();
        self.input_coupling.reset();
        self.q1.reset();
        self.q2.reset();
        self.q3.reset();
        self.q4.reset();
        self.q1_to_q2.reset();
        self.q2_to_q3.reset();
        self.q3_to_tone.reset();
        self.tone_to_q4.reset();
        self.output_coupling.reset();
        self.tone_stack.reset();
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
        let first_input = self.upsampler.process(loaded_input * OVERSAMPLING_FACTOR);
        let first_output = self.process_circuit(first_input, controls);
        let output = self.downsampler.process(first_output);
        let second_input = self.upsampler.process(0.0);
        let second_output = self.process_circuit(second_input, controls);
        self.downsampler.process(second_output);

        // The half-band reconstruction can overshoot an otherwise rail-bounded
        // internal waveform by a fraction of a volt. Keep the published pedal
        // boundary inside the 9 V single-supply audio headroom.
        ElectricalSignal::new(output.clamp(-4.5, 4.5), Self::OUTPUT_IMPEDANCE_OHMS)
    }

    fn process_circuit(&mut self, loaded_input: f32, controls: MuffinControls) -> f32 {
        let sustain = controls.sustain.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let level = controls.level.clamp(0.0, 1.0);

        let q1 = self.q1.process(self.input_coupling.process(loaded_input));

        // The Sustain pot moves the shunt-feedback operating range of the two
        // clipping stages. It is represented as the physical base drive made
        // available through their 100 kOhm feedback networks, not an output
        // waveshaper gain.
        let clipping_drive = 0.12 + sustain.powf(1.25) * 1.10;
        let q2_open = self.q2.process(self.q1_to_q2.process(q1) * clipping_drive);
        let q2 = self
            .diodes_a
            .process(q2_open, self.q2.collector_resistance_ohms());
        let q3_open = self.q3.process(self.q2_to_q3.process(q2) * clipping_drive);
        let q3 = self
            .diodes_b
            .process(q3_open, self.q3.collector_resistance_ohms());

        let tone_input = self.q3_to_tone.process(q3);
        let tone_output = self.tone_stack.process(
            tone_input,
            tone,
            self.q3.collector_resistance_ohms(),
            Self::RECOVERY_STAGE_INPUT_LOAD_OHMS,
        );
        let recovery = self.q4.process(self.tone_to_q4.process(tone_output));

        // The level control is the 100 kOhm output pot after Q4. It attenuates
        // the recovered voltage but does not alter clipping-stage bias.
        let volume_taper = level.powf(1.35);
        let output = self
            .output_coupling
            .process(recovery * volume_taper)
            .clamp(-4.5, 4.5);

        output
    }
}
