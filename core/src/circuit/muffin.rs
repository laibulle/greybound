//! Component-level cells for the BJT Muffin topology.
//!
//! These cells intentionally use AC quantities around their DC operating
//! points.  The fixed bias network establishes `quiescent_collector_current_a`;
//! the audio path solves the incremental BJT/emitter equation, diode load, and
//! passive tone network at every sample.  This keeps the runtime bounded while
//! retaining the circuit quantities that matter at the pedal boundary.

const THERMAL_VOLTAGE_V: f32 = 25.85e-3;
const TONE_NODES: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct BjtCommonEmitterParams {
    pub supply_voltage_v: f32,
    pub collector_resistance_ohms: f32,
    pub emitter_resistance_ohms: f32,
    pub emitter_bypass_capacitance_f: f32,
    pub collector_capacitance_f: f32,
    pub quiescent_collector_current_a: f32,
    pub collector_load_ohms: f32,
}

/// A common-emitter stage with an explicit emitter bypass capacitor and
/// collector capacitance. The Newton solve is bounded to four iterations and
/// is allocation-free in the audio path.
pub struct BjtCommonEmitterStage {
    params: BjtCommonEmitterParams,
    emitter_capacitor: TrapezoidalCapacitor,
    collector_capacitor: TrapezoidalCapacitor,
}

impl BjtCommonEmitterStage {
    pub fn new(sample_rate: f32, params: BjtCommonEmitterParams) -> Self {
        Self {
            params,
            emitter_capacitor: TrapezoidalCapacitor::new(
                sample_rate,
                params.emitter_bypass_capacitance_f,
            ),
            collector_capacitor: TrapezoidalCapacitor::new(
                sample_rate,
                params.collector_capacitance_f,
            ),
        }
    }

    pub fn reset(&mut self) {
        self.emitter_capacitor.reset();
        self.collector_capacitor.reset();
    }

    #[inline]
    pub fn collector_resistance_ohms(&self) -> f32 {
        self.params.collector_resistance_ohms
    }

    /// Process an AC-coupled base voltage and return the collector AC voltage.
    /// The DC current is represented by `quiescent_collector_current_a`, so a
    /// zero input produces a zero output without leaking the 9 V rail into the
    /// audio signal.
    pub fn process(&mut self, base_ac_v: f32) -> f32 {
        let emitter_conductance =
            1.0 / self.params.emitter_resistance_ohms.max(1.0) + self.emitter_capacitor.conductance;
        let emitter_history = self.emitter_capacitor.history_current;
        let quiescent = self.params.quiescent_collector_current_a.max(1.0e-7);
        let mut emitter_ac_v = 0.0;

        for _ in 0..4 {
            let exponent = ((base_ac_v - emitter_ac_v) / THERMAL_VOLTAGE_V).clamp(-16.0, 16.0);
            let collector_current = quiescent * exponent.exp();
            let incremental_current = collector_current - quiescent;
            let residual =
                emitter_conductance * emitter_ac_v + emitter_history - incremental_current;
            let slope = emitter_conductance + collector_current / THERMAL_VOLTAGE_V;
            emitter_ac_v -= residual / slope.max(1.0e-6);
            emitter_ac_v = emitter_ac_v.clamp(-1.5, 1.5);
        }

        let exponent = ((base_ac_v - emitter_ac_v) / THERMAL_VOLTAGE_V).clamp(-16.0, 16.0);
        let incremental_current = quiescent * exponent.exp() - quiescent;
        self.emitter_capacitor.update(emitter_ac_v);

        let collector_resistance = parallel(
            self.params.collector_resistance_ohms,
            self.params.collector_load_ohms,
        );
        let collector_conductance =
            1.0 / collector_resistance.max(1.0) + self.collector_capacitor.conductance;
        let unclamped_collector_ac_v = (-incremental_current
            - self.collector_capacitor.history_current)
            / collector_conductance.max(1.0e-6);
        // The incremental current solve is referenced to the DC operating
        // point, but the collector is still bounded by the 9 V rail and by
        // transistor saturation. Applying this limit before updating the
        // capacitor companion prevents an impossible current excursion from
        // becoming a multi-second DC transient at the next coupling capacitor.
        let quiescent_collector_v = (self.params.supply_voltage_v
            - self.params.quiescent_collector_current_a * self.params.collector_resistance_ohms)
            .clamp(0.2, self.params.supply_voltage_v - 0.1);
        let collector_ac_v = unclamped_collector_ac_v.clamp(
            -(quiescent_collector_v - 0.2).max(0.1),
            (self.params.supply_voltage_v - quiescent_collector_v).max(0.1),
        );
        self.collector_capacitor.update(collector_ac_v);
        collector_ac_v
    }
}

/// Antiparallel 1N4148-like silicon diodes driven from a finite source
/// resistance. This is a Shockley solve, not a post-gain waveshaper.
pub struct SiliconDiodePair {
    saturation_current_a: f32,
    ideality_times_thermal_voltage_v: f32,
}

impl SiliconDiodePair {
    pub fn one_n4148() -> Self {
        Self {
            saturation_current_a: 2.5e-9,
            ideality_times_thermal_voltage_v: 1.75 * THERMAL_VOLTAGE_V,
        }
    }

    pub fn process(&self, source_v: f32, source_resistance_ohms: f32) -> f32 {
        let source_resistance_ohms = source_resistance_ohms.max(1.0);
        // Start near the silicon knee. Starting at the unconstrained source
        // voltage makes Newton spend its bounded iteration budget walking down
        // the exponential tail for a high-impedance collector source.
        let mut node_v = source_v.clamp(-0.7, 0.7);
        for _ in 0..8 {
            let normalized = (node_v / self.ideality_times_thermal_voltage_v).clamp(-16.0, 16.0);
            let diode_current = 2.0 * self.saturation_current_a * normalized.sinh();
            let diode_conductance = 2.0 * self.saturation_current_a * normalized.cosh()
                / self.ideality_times_thermal_voltage_v;
            let residual = (source_v - node_v) / source_resistance_ohms - diode_current;
            let slope = -1.0 / source_resistance_ohms - diode_conductance;
            node_v -= residual / slope.min(-1.0e-9);
            node_v = node_v.clamp(-1.5, 1.5);
        }
        node_v
    }
}

/// The classic Big Muff passive blend: a 39 kOhm / 10 nF low path, a 3.9 nF
/// / 22 kOhm high path, and a 100 kOhm pot. The MNA system retains both
/// capacitor histories and includes source/recovery loading.
pub struct MuffinToneStack {
    low_capacitor: TrapezoidalCapacitor,
    high_capacitor: TrapezoidalCapacitor,
    inverse_matrix: [[f32; TONE_NODES]; TONE_NODES],
    tone: f32,
    source_resistance_ohms: f32,
    load_resistance_ohms: f32,
}

impl MuffinToneStack {
    pub const LOW_RESISTANCE_OHMS: f32 = 39_000.0;
    pub const LOW_CAPACITANCE_F: f32 = 10e-9;
    pub const HIGH_RESISTANCE_OHMS: f32 = 22_000.0;
    pub const HIGH_CAPACITANCE_F: f32 = 3.9e-9;
    pub const POTENTIOMETER_OHMS: f32 = 100_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            low_capacitor: TrapezoidalCapacitor::new(sample_rate, Self::LOW_CAPACITANCE_F),
            high_capacitor: TrapezoidalCapacitor::new(sample_rate, Self::HIGH_CAPACITANCE_F),
            inverse_matrix: [[0.0; TONE_NODES]; TONE_NODES],
            tone: f32::NAN,
            source_resistance_ohms: f32::NAN,
            load_resistance_ohms: f32::NAN,
        }
    }

    pub fn reset(&mut self) {
        self.low_capacitor.reset();
        self.high_capacitor.reset();
    }

    pub fn process(
        &mut self,
        input_v: f32,
        tone: f32,
        source_resistance_ohms: f32,
        load_resistance_ohms: f32,
    ) -> f32 {
        let tone = tone.clamp(0.0, 1.0);
        let source_resistance_ohms = source_resistance_ohms.max(1.0);
        let load_resistance_ohms = load_resistance_ohms.max(1.0);
        if tone != self.tone
            || source_resistance_ohms != self.source_resistance_ohms
            || load_resistance_ohms != self.load_resistance_ohms
        {
            self.update_matrix(tone, source_resistance_ohms, load_resistance_ohms);
        }

        // input, low branch, high branch, pot wiper/output
        let mut rhs = [0.0; TONE_NODES];
        rhs[0] = input_v / source_resistance_ohms;
        self.low_capacitor.stamp_rhs_to_ground(&mut rhs, 1);
        self.high_capacitor.stamp_rhs_between(&mut rhs, 0, 2);
        let voltages = multiply(self.inverse_matrix, rhs);
        self.low_capacitor.update(voltages[1]);
        self.high_capacitor.update(voltages[0] - voltages[2]);
        voltages[3]
    }

    fn update_matrix(&mut self, tone: f32, source_resistance_ohms: f32, load_resistance_ohms: f32) {
        let mut matrix = [[0.0; TONE_NODES]; TONE_NODES];
        stamp_to_ground(&mut matrix, 0, source_resistance_ohms);
        stamp_between(&mut matrix, 0, 1, Self::LOW_RESISTANCE_OHMS);
        stamp_to_ground(&mut matrix, 2, Self::HIGH_RESISTANCE_OHMS);
        stamp_between(
            &mut matrix,
            1,
            3,
            pot_segment(Self::POTENTIOMETER_OHMS * tone),
        );
        stamp_between(
            &mut matrix,
            3,
            2,
            pot_segment(Self::POTENTIOMETER_OHMS * (1.0 - tone)),
        );
        stamp_to_ground(&mut matrix, 3, load_resistance_ohms);
        self.low_capacitor.stamp_to_ground(&mut matrix, 1);
        self.high_capacitor.stamp_between(&mut matrix, 0, 2);
        self.inverse_matrix = invert(matrix);
        self.tone = tone;
        self.source_resistance_ohms = source_resistance_ohms;
        self.load_resistance_ohms = load_resistance_ohms;
    }
}

struct TrapezoidalCapacitor {
    conductance: f32,
    history_current: f32,
}

impl TrapezoidalCapacitor {
    fn new(sample_rate: f32, capacitance_f: f32) -> Self {
        Self {
            conductance: 2.0 * sample_rate * capacitance_f.max(0.0),
            history_current: 0.0,
        }
    }

    fn reset(&mut self) {
        self.history_current = 0.0;
    }

    fn update(&mut self, voltage_v: f32) {
        let current = self.conductance * voltage_v + self.history_current;
        self.history_current = -self.conductance * voltage_v - current;
    }

    fn stamp_to_ground(&self, matrix: &mut [[f32; TONE_NODES]; TONE_NODES], node: usize) {
        matrix[node][node] += self.conductance;
    }

    fn stamp_between(&self, matrix: &mut [[f32; TONE_NODES]; TONE_NODES], a: usize, b: usize) {
        stamp_conductance_between(matrix, a, b, self.conductance);
    }

    fn stamp_rhs_to_ground(&self, rhs: &mut [f32; TONE_NODES], node: usize) {
        rhs[node] -= self.history_current;
    }

    fn stamp_rhs_between(&self, rhs: &mut [f32; TONE_NODES], a: usize, b: usize) {
        rhs[a] -= self.history_current;
        rhs[b] += self.history_current;
    }
}

fn parallel(a: f32, b: f32) -> f32 {
    1.0 / (1.0 / a.max(1.0) + 1.0 / b.max(1.0))
}

fn pot_segment(resistance_ohms: f32) -> f32 {
    resistance_ohms.max(10.0)
}

fn stamp_to_ground(matrix: &mut [[f32; TONE_NODES]; TONE_NODES], node: usize, resistance: f32) {
    matrix[node][node] += 1.0 / resistance.max(1.0);
}

fn stamp_between(
    matrix: &mut [[f32; TONE_NODES]; TONE_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_conductance_between(matrix, a, b, 1.0 / resistance.max(1.0));
}

fn stamp_conductance_between(
    matrix: &mut [[f32; TONE_NODES]; TONE_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn multiply(matrix: [[f32; TONE_NODES]; TONE_NODES], rhs: [f32; TONE_NODES]) -> [f32; TONE_NODES] {
    let mut output = [0.0; TONE_NODES];
    for row in 0..TONE_NODES {
        for column in 0..TONE_NODES {
            output[row] += matrix[row][column] * rhs[column];
        }
    }
    output
}

fn invert(mut matrix: [[f32; TONE_NODES]; TONE_NODES]) -> [[f32; TONE_NODES]; TONE_NODES] {
    let mut inverse = [[0.0; TONE_NODES]; TONE_NODES];
    for row in 0..TONE_NODES {
        inverse[row][row] = 1.0;
    }
    for pivot in 0..TONE_NODES {
        let mut pivot_row = pivot;
        for row in (pivot + 1)..TONE_NODES {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            inverse.swap(pivot, pivot_row);
        }
        let scale = matrix[pivot][pivot].abs().max(1.0e-12);
        for column in 0..TONE_NODES {
            matrix[pivot][column] /= scale;
            inverse[pivot][column] /= scale;
        }
        for row in 0..TONE_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in 0..TONE_NODES {
                matrix[row][column] -= factor * matrix[pivot][column];
                inverse[row][column] -= factor * inverse[pivot][column];
            }
        }
    }
    inverse
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diode_pair_limits_a_finite_source() {
        let diodes = SiliconDiodePair::one_n4148();
        assert!(diodes.process(4.0, 39_000.0).abs() < 0.8);
    }

    #[test]
    fn tone_control_moves_high_frequency_energy() {
        let mut dark = MuffinToneStack::new(48_000.0);
        let mut bright = MuffinToneStack::new(48_000.0);
        let mut dark_energy = 0.0;
        let mut bright_energy = 0.0;
        for index in 0..9_600 {
            let input = (std::f32::consts::TAU * 3_000.0 * index as f32 / 48_000.0).sin();
            if index > 4_800 {
                dark_energy += dark.process(input, 0.05, 39_000.0, 100_000.0).abs();
                bright_energy += bright.process(input, 0.95, 39_000.0, 100_000.0).abs();
            } else {
                dark.process(input, 0.05, 39_000.0, 100_000.0);
                bright.process(input, 0.95, 39_000.0, 100_000.0);
            }
        }
        assert!(bright_energy > dark_energy * 1.5);
    }

    #[test]
    fn common_emitter_collector_respects_supply_headroom() {
        let mut stage = BjtCommonEmitterStage::new(
            48_000.0,
            BjtCommonEmitterParams {
                supply_voltage_v: 9.0,
                collector_resistance_ohms: 100_000.0,
                emitter_resistance_ohms: 390.0,
                emitter_bypass_capacitance_f: 1e-6,
                collector_capacitance_f: 220e-12,
                quiescent_collector_current_a: 45e-6,
                collector_load_ohms: 100_000.0,
            },
        );
        for _ in 0..1_024 {
            let output = stage.process(4.0);
            assert!(output.is_finite());
            assert!((-4.3..=4.5).contains(&output));
        }
    }
}
