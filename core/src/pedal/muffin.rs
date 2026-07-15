use crate::amp::oversampling::{half_band_coefficients, FirFilter, OVERSAMPLING_FACTOR};
use crate::circuit::muffin::{
    MuffinFeedbackClippingStage, MuffinShuntFeedbackStage, MuffinToneStack, MuffinVoicing,
};

use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct MuffinControls {
    pub sustain: f32,
    pub tone: f32,
    pub level: f32,
    /// Tone Wicker macro: lifts C2/C6/C9 and bypasses the passive tone stack.
    pub wicker: f32,
    /// 0 = V3, 1 = 1974 Violet Ram's Head, 2 = Green Russian, 3 = Triangle.
    pub voicing: f32,
}

/// Audio-domain voltages at the named signal boundaries of the Muffin model.
///
/// These are AC signals, not the transistor's absolute 9 V DC operating
/// points.  They map to the corresponding SPICE fixture nodes after their
/// coupling capacitors: `input_rs`, `q1_c`, `sustain_wiper`, `q2_c`, `q3_c`,
/// `tone_wiper`, `q4_c`, and `output`.
#[derive(Clone, Copy, Debug, Default)]
pub struct MuffinNodeVoltages {
    pub loaded_input: f32,
    pub q1_collector: f32,
    pub sustain_wiper: f32,
    pub q2_collector: f32,
    pub q3_collector: f32,
    pub tone_wiper: f32,
    pub q4_collector: f32,
    pub output: f32,
}

#[derive(Clone, Copy, Debug)]
struct MuffinProcessResult {
    output: f32,
    stages: MuffinNodeVoltages,
}

impl Default for MuffinControls {
    fn default() -> Self {
        Self {
            sustain: 0.55,
            tone: 0.50,
            level: 0.70,
            wicker: 0.0,
            voicing: MuffinVoicing::V3.control_value(),
        }
    }
}

/// 1976/77 red-and-black Big Muff Pi V3 topology hypothesis.
///
/// The model is intentionally a component circuit, rather than four scalar
/// gains followed by `tanh`: every transistor has an emitter/capacitor state,
/// Q2/Q3 place the clipping pairs in bounded collector-to-base feedback
/// branches, and the tone control is a passive MNA network. Exact production
/// values vary between units; the constants below are the documented V3 set.
pub struct Muffin {
    circuit_sample_rate: f32,
    input_impedance_ohms: f32,
    input_connection: ConnectionState,
    upsampler: FirFilter,
    downsampler: FirFilter,
    input_coupling: OnePoleHighpass,
    q1: MuffinShuntFeedbackStage,
    q2: MuffinFeedbackClippingStage,
    q3: MuffinFeedbackClippingStage,
    q4: MuffinShuntFeedbackStage,
    q1_to_sustain: OnePoleHighpass,
    sustain_to_q2: OnePoleHighpass,
    q2_to_q3: OnePoleHighpass,
    tone_to_q4: OnePoleHighpass,
    output_coupling: OnePoleHighpass,
    tone_stack: MuffinToneStack,
    active_wicker: bool,
    active_voicing: MuffinVoicing,
}

impl Muffin {
    /// The V3 source resistance is dominated by its 39 kOhm input resistor.
    pub const INPUT_IMPEDANCE_OHMS: f32 = 39_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 25_000.0;
    const RECOVERY_STAGE_INPUT_LOAD_OHMS: f32 = 100_000.0;
    const SUSTAIN_POTENTIOMETER_OHMS: f32 = 100_000.0;
    const SUSTAIN_STOP_RESISTANCE_OHMS: f32 = 1_000.0;
    // Q2's base network plus its small-signal transistor input load the pot
    // well below the nominal 100 kOhm shunt resistor.  This value is the
    // V3/SPICE operating-region approximation used to retain that loading.
    const SUSTAIN_WIPER_LOAD_OHMS: f32 = 16_000.0;

    pub fn new(sample_rate: f32) -> Self {
        let circuit_sample_rate = sample_rate * OVERSAMPLING_FACTOR;
        // Coupling corners are derived from the V3 capacitor/resistor paths.
        // They keep DC bias private to each BJT cell while retaining the
        // physical low-frequency recovery.
        Self {
            circuit_sample_rate,
            input_impedance_ohms: Self::INPUT_IMPEDANCE_OHMS,
            input_connection: ConnectionState::new(sample_rate, 470e-12),
            upsampler: FirFilter::new(half_band_coefficients()),
            downsampler: FirFilter::new(half_band_coefficients()),
            input_coupling: OnePoleHighpass::new(circuit_sample_rate, 1.8),
            q1: MuffinShuntFeedbackStage::v3_input_booster(circuit_sample_rate),
            q2: MuffinFeedbackClippingStage::v3(circuit_sample_rate),
            q3: MuffinFeedbackClippingStage::v3(circuit_sample_rate),
            q4: MuffinShuntFeedbackStage::v3_recovery(circuit_sample_rate),
            q1_to_sustain: OnePoleHighpass::new(circuit_sample_rate, 1.6),
            sustain_to_q2: OnePoleHighpass::new(circuit_sample_rate, 60.0),
            q2_to_q3: OnePoleHighpass::new(circuit_sample_rate, 14.5),
            tone_to_q4: OnePoleHighpass::new(circuit_sample_rate, 15.9),
            output_coupling: OnePoleHighpass::new(circuit_sample_rate, 15.9),
            tone_stack: MuffinToneStack::new(circuit_sample_rate),
            active_wicker: false,
            active_voicing: MuffinVoicing::V3,
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
        self.q1_to_sustain.reset();
        self.sustain_to_q2.reset();
        self.q2_to_q3.reset();
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
            .drive_load(input, Load::new(self.input_impedance_ohms));
        self.process_loaded_voltage(loaded_input, controls)
    }

    /// Processes one host-rate sample and returns the AC signal at each
    /// component-model boundary. This is intended for model validation and
    /// diagnostics; normal callers should use [`Self::process`].
    pub fn process_with_node_voltages(
        &mut self,
        input: ElectricalSignal,
        controls: MuffinControls,
    ) -> (ElectricalSignal, MuffinNodeVoltages) {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(self.input_impedance_ohms));
        self.process_loaded_voltage_with_node_voltages(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: MuffinControls,
    ) -> ElectricalSignal {
        self.process_loaded_voltage_with_node_voltages(loaded_input, controls)
            .0
    }

    fn process_loaded_voltage_with_node_voltages(
        &mut self,
        loaded_input: f32,
        controls: MuffinControls,
    ) -> (ElectricalSignal, MuffinNodeVoltages) {
        let first_input = self.upsampler.process(loaded_input * OVERSAMPLING_FACTOR);
        let first = self.process_circuit(first_input, controls);
        // The first 2x output clocks the FIR's intermediate polyphase state.
        // The following (zero-stuffed) half-step is the host-rate sample. Using
        // the first result as the returned audio leaves an alternating image in
        // the rendered waveform even though both steps must advance the BJT
        // state.
        self.downsampler.process(first.output);
        let second_input = self.upsampler.process(0.0);
        let second = self.process_circuit(second_input, controls);
        let output = self.downsampler.process(second.output);

        // The half-band reconstruction can overshoot an otherwise rail-bounded
        // internal waveform by a fraction of a volt. Keep the published pedal
        // boundary inside the 9 V single-supply audio headroom.
        let output = output.clamp(-4.5, 4.5);
        // Most diagnostic boundaries describe the first 2x circuit step. The
        // pedal output, however, is defined at the host-rate decimator
        // boundary, so expose the exact value returned to the caller.
        let mut stages = first.stages;
        stages.output = output;
        (
            ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS),
            stages,
        )
    }

    fn process_circuit(
        &mut self,
        loaded_input: f32,
        controls: MuffinControls,
    ) -> MuffinProcessResult {
        let sustain = controls.sustain.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let level = controls.level.clamp(0.0, 1.0);
        let wicker = controls.wicker >= 0.5;
        let voicing = MuffinVoicing::from_control(controls.voicing);

        let wicker_topology_changed = wicker != self.active_wicker;
        if wicker != self.active_wicker {
            self.q1.set_wicker_enabled(wicker);
            self.q2.set_wicker_enabled(wicker);
            self.q3.set_wicker_enabled(wicker);
            self.active_wicker = wicker;
        }
        if voicing != self.active_voicing {
            self.tone_stack
                .set_voicing(self.circuit_sample_rate, voicing);
            self.apply_transistor_voicing(voicing);
            self.active_voicing = voicing;
        }
        if wicker_topology_changed {
            // Wicker changes the feedback topology. Voice only retunes live
            // component values and must not reset the whole circuit, which
            // was the source of its audible switch click.
            self.reset();
        }

        // R1/C1/R2/R3/C2 are the V3 input path. Q1 owns the 39 kOhm source
        // divider and its 470 kOhm / 470 pF collector-to-base feedback loop.
        let q1 = self.q1.process(self.input_coupling.process(loaded_input));

        // C3 feeds the physical 100 kOhm Sustain divider.  R6 retains the
        // finite low-stop feed; there is no artificial gain floor or taper.
        let sustain_wiper = Self::sustain_wiper(self.q1_to_sustain.process(q1), sustain);
        let q2 = self.q2.process(self.sustain_to_q2.process(sustain_wiper));
        let q3 = self.q3.process(self.q2_to_q3.process(q2));

        // The single Tone Wicker control is deliberately a macro: it opens
        // the three high-frequency filters and routes around the lossy tone
        // stack. This is the useful one-control form of the EHX Tone Wicker
        // behavior; Tone is intentionally inactive while it is engaged.
        let tone_output = if wicker {
            q3
        } else {
            self.tone_stack
                .process(q3, tone, 10_000.0, Self::RECOVERY_STAGE_INPUT_LOAD_OHMS)
        };
        let recovery = self.q4.process(self.tone_to_q4.process(tone_output));

        // V3 uses a linear 100 kOhm output pot after Q4.
        let volume_taper = level;
        let output = self
            .output_coupling
            .process(recovery * volume_taper)
            .clamp(-4.5, 4.5);

        MuffinProcessResult {
            output,
            stages: MuffinNodeVoltages {
                loaded_input,
                q1_collector: q1,
                sustain_wiper,
                q2_collector: q2,
                q3_collector: q3,
                tone_wiper: tone_output,
                q4_collector: recovery,
                output,
            },
        }
    }

    fn sustain_wiper(input_v: f32, sustain: f32) -> f32 {
        let top = (1.0 - sustain) * Self::SUSTAIN_POTENTIOMETER_OHMS + 1.0;
        let bottom =
            sustain * Self::SUSTAIN_POTENTIOMETER_OHMS + Self::SUSTAIN_STOP_RESISTANCE_OHMS + 1.0;
        let loaded_bottom = parallel(bottom, Self::SUSTAIN_WIPER_LOAD_OHMS);
        input_v * loaded_bottom / (top + loaded_bottom)
    }

    fn apply_transistor_voicing(&mut self, voicing: MuffinVoicing) {
        // Each voice changes the actual stage networks, not only hFE. Values
        // are a named schematic-family target and remain hypotheses until a
        // particular vintage unit is traced or measured.
        let sr = self.circuit_sample_rate;
        match voicing {
            MuffinVoicing::V3 => {
                self.input_impedance_ohms = 39_000.0;
                self.q1.set_component_profile(
                    sr, 39_000.0, 47_000.0, 470_000.0, 10_000.0, 100.0, 0.187e-3, 300.0, 470e-12,
                );
                self.q2.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 10_000.0, 150.0, 0.438e-3, 300.0, 1e-6,
                    470e-12,
                );
                self.q3.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 10_000.0, 150.0, 0.438e-3, 300.0, 1e-6,
                    470e-12,
                );
                self.q4.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 15_000.0, 3_300.0, 0.164e-3, 300.0, 0.0,
                );
                self.set_coupling_corners(1.8, 1.6, 60.0, 14.5, 15.9, 15.9);
            }
            MuffinVoicing::RamsHead => {
                // 1974 V2 Violet Ram's Head, 470 pF filter version.
                self.input_impedance_ohms = 39_000.0;
                self.q1.set_component_profile(
                    sr, 39_000.0, 100_000.0, 470_000.0, 15_000.0, 100.0, 0.190e-3, 220.0, 470e-12,
                );
                self.q2.set_component_profile(
                    sr, 8_200.0, 100_000.0, 470_000.0, 15_000.0, 100.0, 0.430e-3, 220.0, 100e-9,
                    470e-12,
                );
                self.q3.set_component_profile(
                    sr, 8_200.0, 100_000.0, 470_000.0, 15_000.0, 100.0, 0.430e-3, 220.0, 100e-9,
                    470e-12,
                );
                self.q4.set_component_profile(
                    sr, 10_000.0, 100_000.0, 390_000.0, 10_000.0, 2_200.0, 0.180e-3, 220.0, 0.0,
                );
                self.set_coupling_corners(18.0, 16.0, 60.0, 14.5, 15.9, 15.9);
            }
            MuffinVoicing::GreenRussian => {
                self.input_impedance_ohms = 39_000.0;
                self.q1.set_component_profile(
                    sr, 39_000.0, 100_000.0, 470_000.0, 12_000.0, 390.0, 0.160e-3, 230.0, 470e-12,
                );
                self.q2.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 12_000.0, 390.0, 0.330e-3, 230.0, 47e-9,
                    560e-12,
                );
                self.q3.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 12_000.0, 390.0, 0.330e-3, 230.0, 47e-9,
                    560e-12,
                );
                self.q4.set_component_profile(
                    sr, 10_000.0, 100_000.0, 470_000.0, 10_000.0, 2_000.0, 0.130e-3, 230.0, 0.0,
                );
                self.set_coupling_corners(18.0, 16.0, 60.0, 14.5, 15.9, 15.9);
            }
            MuffinVoicing::Triangle => {
                self.input_impedance_ohms = 33_000.0;
                self.q1.set_component_profile(
                    sr, 33_000.0, 82_000.0, 390_000.0, 12_000.0, 100.0, 0.175e-3, 260.0, 560e-12,
                );
                self.q2.set_component_profile(
                    sr, 8_200.0, 82_000.0, 390_000.0, 12_000.0, 100.0, 0.400e-3, 260.0, 50e-9,
                    560e-12,
                );
                self.q3.set_component_profile(
                    sr, 8_200.0, 82_000.0, 390_000.0, 12_000.0, 100.0, 0.400e-3, 260.0, 50e-9,
                    560e-12,
                );
                self.q4.set_component_profile(
                    sr, 10_000.0, 100_000.0, 390_000.0, 12_000.0, 2_700.0, 0.155e-3, 260.0, 0.0,
                );
                self.set_coupling_corners(18.0, 16.0, 60.0, 14.5, 15.9, 15.9);
            }
        }
    }

    fn set_coupling_corners(
        &mut self,
        input_hz: f32,
        sustain_hz: f32,
        q2_hz: f32,
        q3_hz: f32,
        recovery_hz: f32,
        output_hz: f32,
    ) {
        let sr = self.circuit_sample_rate;
        self.input_coupling.set_cutoff(sr, input_hz);
        self.q1_to_sustain.set_cutoff(sr, sustain_hz);
        self.sustain_to_q2.set_cutoff(sr, q2_hz);
        self.q2_to_q3.set_cutoff(sr, q3_hz);
        self.tone_to_q4.set_cutoff(sr, recovery_hz);
        self.output_coupling.set_cutoff(sr, output_hz);
    }
}

fn parallel(a: f32, b: f32) -> f32 {
    1.0 / (1.0 / a.max(1.0) + 1.0 / b.max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_output_uses_the_second_decimator_clock() {
        let controls = MuffinControls {
            sustain: 1.0,
            tone: 0.5,
            level: 1.0,
            wicker: 0.0,
            voicing: 0.0,
        };
        let input = ElectricalSignal::new(0.04, 10_000.0);
        let mut pedal = Muffin::new(48_000.0);
        let mut manual = Muffin::new(48_000.0);

        let output = pedal.process(input, controls).voltage;
        let loaded = manual
            .input_connection
            .drive_load(input, Load::new(manual.input_impedance_ohms));
        let first_input = manual.upsampler.process(loaded * OVERSAMPLING_FACTOR);
        let first = manual.process_circuit(first_input, controls);
        manual.downsampler.process(first.output);
        let second_input = manual.upsampler.process(0.0);
        let second = manual.process_circuit(second_input, controls);
        let expected = manual.downsampler.process(second.output).clamp(-4.5, 4.5);

        assert!((output - expected).abs() < 1.0e-6);
    }
}
