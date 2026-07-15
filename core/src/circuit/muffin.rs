//! Component-level cells for the BJT Muffin topology.
//!
//! These cells intentionally use AC quantities around their DC operating
//! points.  The fixed bias network establishes `quiescent_collector_current_a`;
//! the audio path solves the incremental BJT/emitter equation, diode load, and
//! passive tone network at every sample.  This keeps the runtime bounded while
//! retaining the circuit quantities that matter at the pedal boundary.

const THERMAL_VOLTAGE_V: f32 = 25.85e-3;
const BJT_CURRENT_GAIN: f32 = 300.0;
const TONE_NODES: usize = 5;

/// Three passive tone-stack component sets found in the transistor Big Muff
/// family. They intentionally preserve the common four-transistor topology;
/// this is a circuit voicing selector, not a claim that every production
/// revision (transistors, bias and diode batches included) is identical.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MuffinVoicing {
    V3,
    RamsHead,
    GreenRussian,
    Triangle,
}

impl MuffinVoicing {
    pub fn from_control(value: f32) -> Self {
        match value.round().clamp(0.0, 3.0) as u8 {
            1 => Self::RamsHead,
            2 => Self::GreenRussian,
            3 => Self::Triangle,
            _ => Self::V3,
        }
    }

    pub const fn control_value(self) -> f32 {
        match self {
            Self::V3 => 0.0,
            Self::RamsHead => 1.0,
            Self::GreenRussian => 2.0,
            Self::Triangle => 3.0,
        }
    }
}

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
        let (emitter_ac_v, collector_ac_v) = self.solve(base_ac_v);
        self.commit(emitter_ac_v, collector_ac_v);
        collector_ac_v
    }

    fn solve(&self, base_ac_v: f32) -> (f32, f32) {
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
        let negative_headroom = (quiescent_collector_v - 0.2).max(0.1);
        let positive_headroom = (self.params.supply_voltage_v - quiescent_collector_v).max(0.1);
        // A transistor approaches collector saturation continuously.  A hard
        // rail clamp created discontinuous slopes once the BJT exponential
        // reached the fixed operating-point headroom, which was audible as
        // glitchy edge energy after Q2/Q3 feedback.  Keep the asymmetric rail
        // limits but make that physical transition smooth.
        let collector_ac_v = if unclamped_collector_ac_v >= 0.0 {
            positive_headroom * (unclamped_collector_ac_v / positive_headroom).tanh()
        } else {
            negative_headroom * (unclamped_collector_ac_v / negative_headroom).tanh()
        };
        (emitter_ac_v, collector_ac_v)
    }

    fn commit(&mut self, emitter_ac_v: f32, collector_ac_v: f32) {
        self.emitter_capacitor.update(emitter_ac_v);
        self.collector_capacitor.update(collector_ac_v);
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

    fn current(&self, voltage_v: f32) -> f32 {
        let normalized = (voltage_v / self.ideality_times_thermal_voltage_v).clamp(-16.0, 16.0);
        2.0 * self.saturation_current_a * normalized.sinh()
    }
}

/// V3 clipping amplifier: an NPN common-emitter stage with the diode pair in
/// its *collector-to-base feedback loop*.  The 1 uF branch capacitor removes
/// DC from the diode loop, while the 470 kOhm / 470 pF network supplies the
/// always-active linear feedback.  This is deliberately not a post-stage
/// shunt clipper.
pub struct MuffinFeedbackClippingStage {
    diodes: SiliconDiodePair,
    input_series_resistance_ohms: f32,
    base_resistance_ohms: f32,
    feedback_resistance_ohms: f32,
    supply_voltage_v: f32,
    collector_resistance_ohms: f32,
    emitter_resistance_ohms: f32,
    quiescent_collector_current_a: f32,
    current_gain: f32,
    diode_capacitor: TrapezoidalCapacitor,
    miller_capacitor: TrapezoidalCapacitor,
    wicker_enabled: bool,
    base_v: f32,
    collector_v: f32,
    emitter_v: f32,
    diode_v: f32,
}

impl MuffinFeedbackClippingStage {
    pub fn v3(sample_rate: f32) -> Self {
        Self {
            diodes: SiliconDiodePair::one_n4148(),
            input_series_resistance_ohms: 10_000.0,
            base_resistance_ohms: 100_000.0,
            feedback_resistance_ohms: 470_000.0,
            supply_voltage_v: 9.0,
            collector_resistance_ohms: 10_000.0,
            emitter_resistance_ohms: 150.0,
            quiescent_collector_current_a: 0.438e-3,
            current_gain: BJT_CURRENT_GAIN,
            // C5/C8 connect the base to a separate diode node, not directly
            // to the collector. Retaining that node is essential to the
            // feedback loop's smooth release.
            diode_capacitor: TrapezoidalCapacitor::new(sample_rate, 1e-6),
            // C6/C9 are literal collector-to-base Miller capacitors.
            miller_capacitor: TrapezoidalCapacitor::new(sample_rate, 470e-12),
            wicker_enabled: false,
            base_v: 0.0,
            collector_v: 0.0,
            emitter_v: 0.0,
            diode_v: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.diode_capacitor.reset();
        self.miller_capacitor.reset();
        self.base_v = 0.0;
        self.collector_v = 0.0;
        self.emitter_v = 0.0;
        self.diode_v = 0.0;
    }

    /// The Tone Wicker switch lifts the collector-to-base high-frequency
    /// feedback capacitors on all three fuzz stages. The diode feedback path
    /// remains active.
    pub fn set_wicker_enabled(&mut self, enabled: bool) {
        self.wicker_enabled = enabled;
        self.miller_capacitor.set_wicker_lifted(enabled);
        // This is a hard component switch, not a continuously varying
        // capacitance. Discard the prior Newton seed rather than starting the
        // new circuit topology from a state that belongs to the old one.
        self.base_v = 0.0;
        self.collector_v = 0.0;
        self.emitter_v = 0.0;
        self.diode_v = 0.0;
    }

    pub fn set_transistor_profile(
        &mut self,
        current_gain: f32,
        quiescent_collector_current_a: f32,
        emitter_resistance_ohms: f32,
    ) {
        self.current_gain = current_gain.max(20.0);
        self.quiescent_collector_current_a = quiescent_collector_current_a.max(1.0e-6);
        self.emitter_resistance_ohms = emitter_resistance_ohms.max(10.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_component_profile(
        &mut self,
        sample_rate: f32,
        input_series_resistance_ohms: f32,
        base_resistance_ohms: f32,
        feedback_resistance_ohms: f32,
        collector_resistance_ohms: f32,
        emitter_resistance_ohms: f32,
        quiescent_collector_current_a: f32,
        current_gain: f32,
        diode_capacitance_f: f32,
        miller_capacitance_f: f32,
    ) {
        self.input_series_resistance_ohms = input_series_resistance_ohms.max(100.0);
        self.base_resistance_ohms = base_resistance_ohms.max(1_000.0);
        self.feedback_resistance_ohms = feedback_resistance_ohms.max(10_000.0);
        self.collector_resistance_ohms = collector_resistance_ohms.max(1_000.0);
        self.set_transistor_profile(
            current_gain,
            quiescent_collector_current_a,
            emitter_resistance_ohms,
        );
        self.diode_capacitor
            .set_capacitance(sample_rate, diode_capacitance_f);
        self.miller_capacitor
            .set_capacitance(sample_rate, miller_capacitance_f);
        self.miller_capacitor.set_wicker_lifted(self.wicker_enabled);
    }

    /// Drive the physical 10 kOhm base input and return the collector AC
    /// voltage. Base, collector and emitter are solved together: this keeps
    /// the 470 kOhm branch, the 470 pF capacitor and the diode path in the
    /// same KCL system instead of approximating them as a base-voltage offset.
    pub fn process(&mut self, source_v: f32) -> f32 {
        let mut nodes = [self.base_v, self.collector_v, self.emitter_v, self.diode_v];
        for _ in 0..10 {
            let residual = self.residual(source_v, nodes);
            let mut jacobian = [[0.0; 4]; 4];
            for column in 0..4 {
                let mut perturbed = nodes;
                perturbed[column] += 1.0e-4;
                let shifted = self.residual(source_v, perturbed);
                for row in 0..4 {
                    jacobian[row][column] = (shifted[row] - residual[row]) / 1.0e-4;
                }
            }
            let Some(delta) = solve_4x4(jacobian, residual) else {
                break;
            };
            for index in 0..4 {
                nodes[index] -= delta[index].clamp(-0.25, 0.25);
            }
            nodes[0] = nodes[0].clamp(-1.5, 1.5);
            nodes[1] = nodes[1].clamp(
                -self.negative_collector_headroom_v() - 0.1,
                self.positive_collector_headroom_v() + 0.1,
            );
            nodes[2] = nodes[2].clamp(-1.5, 1.5);
            nodes[3] = nodes[3].clamp(-4.5, 4.5);
            if delta.iter().all(|value| value.abs() < 1.0e-5) {
                break;
            }
        }
        if !nodes.iter().all(|value| value.is_finite()) {
            self.reset();
            return 0.0;
        }
        self.base_v = nodes[0];
        self.collector_v = nodes[1];
        self.emitter_v = nodes[2];
        self.diode_v = nodes[3];
        self.diode_capacitor.update(self.base_v - self.diode_v);
        self.miller_capacitor.update(self.collector_v - self.base_v);
        self.collector_v
    }

    fn residual(&self, source_v: f32, nodes: [f32; 4]) -> [f32; 4] {
        let [base_v, collector_v, emitter_v, diode_v] = nodes;
        let exponent = ((base_v - emitter_v) / THERMAL_VOLTAGE_V).clamp(-16.0, 16.0);
        let unconstrained_collector_current = self.quiescent_collector_current_a * exponent.exp()
            - self.quiescent_collector_current_a;
        // The collector resistor can supply only the current between the 9 V
        // rail and VCE(sat).  Without this in the nonlinear equation, a hot
        // input has no valid KCL root and Newton repeatedly collides with an
        // external voltage clamp.  The asymmetric tanh keeps the cutoff and
        // saturation transitions continuous while retaining that rail budget.
        let positive_increment = ((self.supply_voltage_v - 0.2) / self.collector_resistance_ohms
            - self.quiescent_collector_current_a)
            .max(1.0e-6);
        let negative_increment = self.quiescent_collector_current_a.max(1.0e-6);
        let collector_current = if unconstrained_collector_current >= 0.0 {
            positive_increment * (unconstrained_collector_current / positive_increment).tanh()
        } else {
            negative_increment * (unconstrained_collector_current / negative_increment).tanh()
        };
        let collector_base_v = collector_v - base_v;
        let miller_current = self.miller_capacitor.current(collector_base_v);
        let diode_capacitor_current = self.diode_capacitor.current(base_v - diode_v);
        let diode_current = self.diodes.current(diode_v - collector_v);
        [
            (base_v - source_v) / self.input_series_resistance_ohms
                + base_v / self.base_resistance_ohms
                + (base_v - collector_v) / self.feedback_resistance_ohms
                - miller_current
                + diode_capacitor_current
                + collector_current / self.current_gain,
            collector_v / self.collector_resistance_ohms
                + (collector_v - base_v) / self.feedback_resistance_ohms
                + miller_current
                - diode_current
                + collector_current,
            emitter_v / self.emitter_resistance_ohms
                - collector_current * (1.0 + 1.0 / self.current_gain),
            diode_current - diode_capacitor_current,
        ]
    }

    fn negative_collector_headroom_v(&self) -> f32 {
        (self.supply_voltage_v
            - 0.2
            - self.quiescent_collector_current_a * self.collector_resistance_ohms)
            .max(0.1)
    }

    fn positive_collector_headroom_v(&self) -> f32 {
        (self.quiescent_collector_current_a * self.collector_resistance_ohms).max(0.1)
    }
}

/// Q1 input booster of the V3 circuit.  Unlike Q2/Q3 it has only the linear
/// collector-to-base shunt feedback branch; its 470 kOhm resistor is essential
/// because it keeps the first stage near the documented moderate gain instead
/// of feeding the Sustain divider from a saturated common-emitter stage.
pub struct MuffinShuntFeedbackStage {
    input_series_resistance_ohms: f32,
    base_resistance_ohms: f32,
    feedback_resistance_ohms: f32,
    supply_voltage_v: f32,
    collector_resistance_ohms: f32,
    emitter_resistance_ohms: f32,
    quiescent_collector_current_a: f32,
    current_gain: f32,
    miller_capacitor: TrapezoidalCapacitor,
    wicker_enabled: bool,
    base_v: f32,
    collector_v: f32,
    emitter_v: f32,
}

impl MuffinShuntFeedbackStage {
    pub fn v3_input_booster(sample_rate: f32) -> Self {
        Self::new(
            sample_rate,
            39_000.0,
            47_000.0,
            10_000.0,
            100.0,
            0.187e-3,
            470e-12,
        )
    }

    /// Q4 uses the same collector-to-base resistor feedback topology as Q1,
    /// but no Miller capacitor in the documented V3 drawing.
    pub fn v3_recovery(sample_rate: f32) -> Self {
        Self::new(
            sample_rate,
            10_000.0,
            100_000.0,
            15_000.0,
            3_300.0,
            0.164e-3,
            0.0,
        )
    }

    fn new(
        sample_rate: f32,
        input_series_resistance_ohms: f32,
        base_resistance_ohms: f32,
        collector_resistance_ohms: f32,
        emitter_resistance_ohms: f32,
        quiescent_collector_current_a: f32,
        miller_capacitance_f: f32,
    ) -> Self {
        Self {
            input_series_resistance_ohms,
            base_resistance_ohms,
            feedback_resistance_ohms: 470_000.0,
            supply_voltage_v: 9.0,
            collector_resistance_ohms,
            emitter_resistance_ohms,
            quiescent_collector_current_a,
            current_gain: BJT_CURRENT_GAIN,
            miller_capacitor: TrapezoidalCapacitor::new(sample_rate, miller_capacitance_f),
            wicker_enabled: false,
            base_v: 0.0,
            collector_v: 0.0,
            emitter_v: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.miller_capacitor.reset();
        self.base_v = 0.0;
        self.collector_v = 0.0;
        self.emitter_v = 0.0;
    }

    pub fn set_wicker_enabled(&mut self, enabled: bool) {
        self.wicker_enabled = enabled;
        self.miller_capacitor.set_wicker_lifted(enabled);
        self.base_v = 0.0;
        self.collector_v = 0.0;
        self.emitter_v = 0.0;
    }

    pub fn set_transistor_profile(
        &mut self,
        current_gain: f32,
        quiescent_collector_current_a: f32,
        emitter_resistance_ohms: f32,
    ) {
        self.current_gain = current_gain.max(20.0);
        self.quiescent_collector_current_a = quiescent_collector_current_a.max(1.0e-6);
        self.emitter_resistance_ohms = emitter_resistance_ohms.max(10.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_component_profile(
        &mut self,
        sample_rate: f32,
        input_series_resistance_ohms: f32,
        base_resistance_ohms: f32,
        feedback_resistance_ohms: f32,
        collector_resistance_ohms: f32,
        emitter_resistance_ohms: f32,
        quiescent_collector_current_a: f32,
        current_gain: f32,
        miller_capacitance_f: f32,
    ) {
        self.input_series_resistance_ohms = input_series_resistance_ohms.max(100.0);
        self.base_resistance_ohms = base_resistance_ohms.max(1_000.0);
        self.feedback_resistance_ohms = feedback_resistance_ohms.max(10_000.0);
        self.collector_resistance_ohms = collector_resistance_ohms.max(1_000.0);
        self.set_transistor_profile(
            current_gain,
            quiescent_collector_current_a,
            emitter_resistance_ohms,
        );
        self.miller_capacitor
            .set_capacitance(sample_rate, miller_capacitance_f);
        self.miller_capacitor.set_wicker_lifted(self.wicker_enabled);
    }

    pub fn process(&mut self, source_v: f32) -> f32 {
        let mut nodes = [self.base_v, self.collector_v, self.emitter_v];
        for _ in 0..20 {
            let residual = self.residual(source_v, nodes);
            let mut jacobian = [[0.0; 3]; 3];
            for column in 0..3 {
                let mut perturbed = nodes;
                perturbed[column] += 1.0e-4;
                let shifted = self.residual(source_v, perturbed);
                for row in 0..3 {
                    jacobian[row][column] = (shifted[row] - residual[row]) / 1.0e-4;
                }
            }
            let Some(delta) = solve_3x3(jacobian, residual) else {
                break;
            };
            // Q4 is driven directly by Q3 in Tone Wicker mode. Near the
            // upper Sustain end, an undamped Newton update can jump from the
            // continuous audio root to a low-output saturated root. Accept
            // only a residual-reducing step, shortening it as needed.
            let residual_energy = residual.iter().map(|value| value.powi(2)).sum::<f32>();
            let mut accepted = false;
            for damping in [1.0, 0.5, 0.25, 0.125] {
                let mut candidate = nodes;
                for index in 0..3 {
                    candidate[index] -= delta[index].clamp(-0.10, 0.10) * damping;
                }
                self.clamp_nodes(&mut candidate);
                let candidate_energy = self
                    .residual(source_v, candidate)
                    .iter()
                    .map(|value| value.powi(2))
                    .sum::<f32>();
                if candidate_energy <= residual_energy {
                    nodes = candidate;
                    accepted = true;
                    break;
                }
            }
            if !accepted {
                for index in 0..3 {
                    nodes[index] -= delta[index].clamp(-0.025, 0.025);
                }
                self.clamp_nodes(&mut nodes);
            }
            if delta.iter().all(|value| value.abs() < 1.0e-5) {
                break;
            }
        }
        if !nodes.iter().all(|value| value.is_finite()) {
            self.reset();
            return 0.0;
        }
        self.base_v = nodes[0];
        self.collector_v = nodes[1];
        self.emitter_v = nodes[2];
        self.miller_capacitor.update(self.collector_v - self.base_v);
        self.collector_v
    }

    fn residual(&self, source_v: f32, nodes: [f32; 3]) -> [f32; 3] {
        let [base_v, collector_v, emitter_v] = nodes;
        let exponent = ((base_v - emitter_v) / THERMAL_VOLTAGE_V).clamp(-16.0, 16.0);
        let unconstrained_collector_current = self.quiescent_collector_current_a * exponent.exp()
            - self.quiescent_collector_current_a;
        let positive_increment = ((self.supply_voltage_v - 0.2) / self.collector_resistance_ohms
            - self.quiescent_collector_current_a)
            .max(1.0e-6);
        let negative_increment = self.quiescent_collector_current_a.max(1.0e-6);
        let collector_current = if unconstrained_collector_current >= 0.0 {
            positive_increment * (unconstrained_collector_current / positive_increment).tanh()
        } else {
            negative_increment * (unconstrained_collector_current / negative_increment).tanh()
        };
        let collector_base_v = collector_v - base_v;
        let miller_current = self.miller_capacitor.current(collector_base_v);
        [
            (base_v - source_v) / self.input_series_resistance_ohms
                + base_v / self.base_resistance_ohms
                + (base_v - collector_v) / self.feedback_resistance_ohms
                - miller_current
                + collector_current / self.current_gain,
            collector_v / self.collector_resistance_ohms
                + (collector_v - base_v) / self.feedback_resistance_ohms
                + miller_current
                + collector_current,
            emitter_v / self.emitter_resistance_ohms
                - collector_current * (1.0 + 1.0 / self.current_gain),
        ]
    }

    fn clamp_nodes(&self, nodes: &mut [f32; 3]) {
        nodes[0] = nodes[0].clamp(-1.5, 1.5);
        nodes[1] = nodes[1].clamp(
            -self.negative_collector_headroom_v() - 0.1,
            self.positive_collector_headroom_v() + 0.1,
        );
        nodes[2] = nodes[2].clamp(-1.5, 1.5);
    }

    fn negative_collector_headroom_v(&self) -> f32 {
        (self.supply_voltage_v
            - 0.2
            - self.quiescent_collector_current_a * self.collector_resistance_ohms)
            .max(0.1)
    }

    fn positive_collector_headroom_v(&self) -> f32 {
        (self.quiescent_collector_current_a * self.collector_resistance_ohms).max(0.1)
    }
}

/// Solve a feedback residual in the BJT base-voltage range. The relaxed
/// fixed-point iteration used previously occasionally selected opposite
/// saturated roots, creating full-scale one-sample glitches in renders.
fn solve_3x3(mut matrix: [[f32; 3]; 3], mut rhs: [f32; 3]) -> Option<[f32; 3]> {
    for pivot in 0..3 {
        let mut pivot_row = pivot;
        for row in (pivot + 1)..3 {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if matrix[pivot_row][pivot].abs() < 1.0e-9 {
            return None;
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            rhs.swap(pivot, pivot_row);
        }
        let scale = matrix[pivot][pivot];
        for column in pivot..3 {
            matrix[pivot][column] /= scale;
        }
        rhs[pivot] /= scale;
        for row in 0..3 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..3 {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

fn solve_4x4(mut matrix: [[f32; 4]; 4], mut rhs: [f32; 4]) -> Option<[f32; 4]> {
    for pivot in 0..4 {
        let mut pivot_row = pivot;
        for row in (pivot + 1)..4 {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if matrix[pivot_row][pivot].abs() < 1.0e-9 {
            return None;
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            rhs.swap(pivot, pivot_row);
        }
        let scale = matrix[pivot][pivot];
        for column in pivot..4 {
            matrix[pivot][column] /= scale;
        }
        rhs[pivot] /= scale;
        for row in 0..4 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..4 {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

/// The classic Big Muff passive blend: a 39 kOhm / 10 nF low path, a 3.9 nF
/// / 22 kOhm high path, and a 100 kOhm pot. The MNA system retains both
/// capacitor histories and includes source/recovery loading. The high branch
/// remains a *series* capacitor/resistor path into the top of the pot; it is
/// not a resistor-to-ground shelf.
pub struct MuffinToneStack {
    low_capacitor: TrapezoidalCapacitor,
    high_capacitor: TrapezoidalCapacitor,
    low_resistance_ohms: f32,
    high_resistance_ohms: f32,
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
            low_resistance_ohms: Self::LOW_RESISTANCE_OHMS,
            high_resistance_ohms: Self::HIGH_RESISTANCE_OHMS,
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

    pub fn set_voicing(&mut self, sample_rate: f32, voicing: MuffinVoicing) {
        let (low_resistance_ohms, low_capacitance_f, high_resistance_ohms, high_capacitance_f) =
            match voicing {
                // 1976/77 V3 target.
                MuffinVoicing::V3 => (39_000.0, 10e-9, 22_000.0, 3.9e-9),
                // 1974 V2 Violet Ram's Head: 39 kOhm branches, 10 nF low
                // path, and 3.9 nF high path. This is a named V2 circuit
                // target, not an arbitrary midrange modification of the V3.
                MuffinVoicing::RamsHead => (39_000.0, 10e-9, 39_000.0, 3.9e-9),
                // Green Russian: the 20 kOhm low-branch resistor shifts the
                // scoop for the heavier, smoother Russian voice.
                MuffinVoicing::GreenRussian => (20_000.0, 10e-9, 22_000.0, 3.9e-9),
                // Early Triangle family: less scooped 33 kOhm / 10 nF low
                // path and 33 kOhm / 4 nF high path.
                MuffinVoicing::Triangle => (33_000.0, 10e-9, 33_000.0, 4e-9),
            };
        if self.low_resistance_ohms == low_resistance_ohms
            && self.high_resistance_ohms == high_resistance_ohms
            && self
                .low_capacitor
                .has_capacitance(low_capacitance_f, sample_rate)
            && self
                .high_capacitor
                .has_capacitance(high_capacitance_f, sample_rate)
        {
            return;
        }
        self.low_resistance_ohms = low_resistance_ohms;
        self.high_resistance_ohms = high_resistance_ohms;
        self.low_capacitor
            .set_capacitance(sample_rate, low_capacitance_f);
        self.high_capacitor
            .set_capacitance(sample_rate, high_capacitance_f);
        self.tone = f32::NAN;
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

        // input, low branch, high-capacitor output, high pot terminal, wiper
        let mut rhs = [0.0; TONE_NODES];
        rhs[0] = input_v / source_resistance_ohms;
        self.low_capacitor.stamp_rhs_to_ground(&mut rhs, 1);
        self.high_capacitor.stamp_rhs_between(&mut rhs, 0, 2);
        let voltages = multiply(self.inverse_matrix, rhs);
        self.low_capacitor.update(voltages[1]);
        self.high_capacitor.update(voltages[0] - voltages[2]);
        voltages[4]
    }

    fn update_matrix(&mut self, tone: f32, source_resistance_ohms: f32, load_resistance_ohms: f32) {
        let mut matrix = [[0.0; TONE_NODES]; TONE_NODES];
        stamp_to_ground(&mut matrix, 0, source_resistance_ohms);
        stamp_between(&mut matrix, 0, 1, self.low_resistance_ohms);
        stamp_between(&mut matrix, 2, 3, self.high_resistance_ohms);
        stamp_between(
            &mut matrix,
            1,
            4,
            pot_segment(Self::POTENTIOMETER_OHMS * tone),
        );
        stamp_between(
            &mut matrix,
            4,
            3,
            pot_segment(Self::POTENTIOMETER_OHMS * (1.0 - tone)),
        );
        stamp_to_ground(&mut matrix, 4, load_resistance_ohms);
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
    nominal_conductance: f32,
    history_current: f32,
    capacitance_f: f32,
    sample_rate: f32,
}

impl TrapezoidalCapacitor {
    fn new(sample_rate: f32, capacitance_f: f32) -> Self {
        Self {
            conductance: 2.0 * sample_rate * capacitance_f.max(0.0),
            nominal_conductance: 2.0 * sample_rate * capacitance_f.max(0.0),
            history_current: 0.0,
            capacitance_f: capacitance_f.max(0.0),
            sample_rate,
        }
    }

    fn reset(&mut self) {
        self.history_current = 0.0;
    }

    /// The hardware Wicker switch opens the three collector-to-base filters.
    /// In the bounded Newton companion model, a tiny residual conductance
    /// keeps the switched feedback topology numerically well-conditioned;
    /// at 20% of 470 pF it remains above the guitar-band filter corner while
    /// preventing Q3 from converging to a false DC-only root under hot drive.
    fn set_wicker_lifted(&mut self, lifted: bool) {
        let conductance = self.nominal_conductance * if lifted { 0.20 } else { 1.0 };
        if self.conductance != conductance {
            self.conductance = conductance;
            self.reset();
        }
    }

    fn has_capacitance(&self, capacitance_f: f32, sample_rate: f32) -> bool {
        self.capacitance_f == capacitance_f.max(0.0) && self.sample_rate == sample_rate
    }

    fn set_capacitance(&mut self, sample_rate: f32, capacitance_f: f32) {
        let previous_conductance = self.conductance;
        self.sample_rate = sample_rate;
        self.capacitance_f = capacitance_f.max(0.0);
        self.nominal_conductance = 2.0 * sample_rate * self.capacitance_f;
        self.conductance = self.nominal_conductance;
        // Preserve the equivalent capacitor voltage across a voice switch
        // rather than dumping the companion state and creating a click.
        if previous_conductance > 0.0 {
            self.history_current *= self.conductance / previous_conductance;
        } else {
            self.reset();
        }
    }

    fn current(&self, voltage_v: f32) -> f32 {
        self.conductance * voltage_v + self.history_current
    }

    fn update(&mut self, voltage_v: f32) {
        let current = self.current(voltage_v);
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
    fn v3_feedback_clipping_stage_has_level_dependent_transfer() {
        let mut low = MuffinFeedbackClippingStage::v3(96_000.0);
        let mut high = MuffinFeedbackClippingStage::v3(96_000.0);
        let mut low_energy = 0.0;
        let mut high_energy = 0.0;
        for index in 0..19_200 {
            let phase = std::f32::consts::TAU * 110.0 * index as f32 / 96_000.0;
            let low_output = low.process(phase.sin() * 5e-3);
            let high_output = high.process(phase.sin() * 0.12);
            if index >= 9_600 {
                low_energy += low_output.powi(2);
                high_energy += high_output.powi(2);
            }
        }
        assert!(low_energy.is_finite());
        assert!(high_energy.is_finite());
        assert!(
            high_energy > low_energy * 2.0,
            "low={low_energy}, high={high_energy}"
        );
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
    fn v3_tone_stack_noon_matches_the_series_high_branch_transfer() {
        let mut tone_stack = MuffinToneStack::new(48_000.0);
        let mut input_energy = 0.0;
        let mut output_energy = 0.0;
        for index in 0..9_600 {
            let input = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin();
            let output = tone_stack.process(input, 0.5, 10_000.0, 100_000.0);
            if index >= 4_800 {
                input_energy += input.powi(2);
                output_energy += output.powi(2);
            }
        }
        let transfer = (output_energy / input_energy).sqrt();
        // The V3 ngspice network at this point is 0.2929 V/V. The high
        // capacitor must feed R18 in series into the pot; grounding R18, as
        // an earlier four-node solve did, yields roughly 0.12 V/V instead.
        assert!(
            (0.28..0.31).contains(&transfer),
            "unexpected V3 noon tone transfer={transfer}"
        );
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
