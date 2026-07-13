use crate::amp::{StageBoundaryState, StageCoupling};

pub const GUITAR_SOURCE_IMPEDANCE_OHMS: f32 = 10_000.0;
pub const AMP_INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;

#[derive(Clone, Copy, Debug)]
pub struct ElectricalSignal {
    pub voltage: f32,
    pub source_impedance_ohms: f32,
}

impl ElectricalSignal {
    pub fn new(voltage: f32, source_impedance_ohms: f32) -> Self {
        Self {
            voltage,
            source_impedance_ohms: source_impedance_ohms.max(1.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Load {
    pub impedance_ohms: f32,
}

impl Load {
    pub fn new(impedance_ohms: f32) -> Self {
        Self {
            impedance_ohms: impedance_ohms.max(1.0),
        }
    }
}

/// Stateful electrical connection between two devices.
///
/// This models the part of a rig that belongs to neither endpoint: source/load
/// voltage division plus cable capacitance. Pedals and amps keep their own DSP
/// state, while this object carries the shared physical boundary state.
#[derive(Debug)]
pub struct ConnectionState {
    sample_rate: f32,
    cable_capacitance_farads: f32,
    voltage_state: f32,
}

impl ConnectionState {
    pub fn new(sample_rate: f32, cable_capacitance_farads: f32) -> Self {
        Self {
            sample_rate,
            cable_capacitance_farads: cable_capacitance_farads.max(0.0),
            voltage_state: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.voltage_state = 0.0;
    }

    pub fn drive_load(&mut self, source: ElectricalSignal, load: Load) -> f32 {
        let source_impedance = source.source_impedance_ohms.max(1.0);
        let load_impedance = load.impedance_ohms.max(1.0);
        let divided = source.voltage * load_impedance / (source_impedance + load_impedance);

        if self.cable_capacitance_farads <= 0.0 {
            self.voltage_state = divided;
            return divided;
        }

        let parallel_resistance = 1.0 / (1.0 / source_impedance + 1.0 / load_impedance);
        let time_constant = parallel_resistance * self.cable_capacitance_farads;
        if time_constant <= f32::EPSILON {
            self.voltage_state = divided;
            return divided;
        }

        let coefficient = 1.0 - (-1.0 / (self.sample_rate * time_constant)).exp();
        self.voltage_state += coefficient.clamp(0.0, 1.0) * (divided - self.voltage_state);
        self.voltage_state
    }
}

pub(super) fn pedal_boundary_state(
    id: &'static str,
    voltage_v: f32,
    source_impedance_ohms: f32,
    load_impedance_ohms: f32,
    coupling: StageCoupling,
    headroom_v: f32,
) -> StageBoundaryState {
    let nominal_level_v = voltage_v.abs();
    StageBoundaryState {
        id,
        voltage_v,
        source_impedance_ohms: source_impedance_ohms.max(0.0),
        load_impedance_ohms: load_impedance_ohms.max(0.0),
        coupling,
        dc_offset_v: 0.0,
        headroom_v: headroom_v.max(0.0),
        nominal_level_v,
        peak_level_v: nominal_level_v,
        latency_samples: 0,
    }
}

pub(super) fn diode_pair_clip(input: f32, knee_voltage: f32) -> f32 {
    knee_voltage * (input / knee_voltage).tanh()
}

pub(super) fn hard_clip(input: f32, limit: f32) -> f32 {
    input.clamp(-limit, limit)
}

pub(super) fn asymmetric_diode_clip(input: f32, negative_knee: f32, positive_knee: f32) -> f32 {
    if input >= 0.0 {
        positive_knee * (input / positive_knee).tanh()
    } else {
        negative_knee * (input / negative_knee).tanh()
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct AllPassStage {
    previous_input: f32,
    previous_output: f32,
}

impl AllPassStage {
    pub(super) fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }

    pub(super) fn process(&mut self, input: f32, coefficient: f32) -> f32 {
        let output = coefficient * (input - self.previous_output) + self.previous_input;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

pub(super) fn allpass_coefficient(frequency_hz: f32, sample_rate: f32) -> f32 {
    let tangent =
        (std::f32::consts::PI * frequency_hz.clamp(20.0, sample_rate * 0.42) / sample_rate).tan();
    ((tangent - 1.0) / (tangent + 1.0)).clamp(-0.98, 0.98)
}

pub(super) fn time_coefficient(sample_rate: f32, time_ms: f32) -> f32 {
    let samples = (time_ms.max(0.1) * 0.001 * sample_rate).max(1.0);
    (1.0 - (-1.0 / samples).exp()).clamp(0.0, 1.0)
}

pub(super) struct FractionalDelayLine {
    buffer: Vec<f32>,
    write_index: usize,
}

impl FractionalDelayLine {
    pub(super) fn new(sample_rate: f32, max_seconds: f32) -> Self {
        let len = (sample_rate * max_seconds).ceil().max(2.0) as usize + 2;
        Self {
            buffer: vec![0.0; len],
            write_index: 0,
        }
    }

    pub(super) fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
    }

    pub(super) fn process(&mut self, input: f32, delay_samples: f32) -> f32 {
        let len = self.buffer.len();
        let delay = delay_samples.clamp(1.0, (len - 2) as f32);
        self.buffer[self.write_index] = input;

        let read_position = (self.write_index as f32 - delay).rem_euclid(len as f32);
        let index_floor = read_position.floor();
        let index_a = index_floor as usize % len;
        let fraction = read_position - index_floor;
        let index_b = (index_a + 1) % len;
        let output = self.buffer[index_a] * (1.0 - fraction) + self.buffer[index_b] * fraction;

        self.write_index = (self.write_index + 1) % len;
        output
    }
}

pub(super) struct SpringDelay {
    buffer: Vec<f32>,
    index: usize,
}

impl SpringDelay {
    pub(super) fn new(sample_rate: f32, seconds: f32) -> Self {
        let len = (sample_rate * seconds).round().max(1.0) as usize;
        Self {
            buffer: vec![0.0; len],
            index: 0,
        }
    }

    pub(super) fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }

    pub(super) fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.buffer[self.index] = input;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

#[derive(Clone, Copy)]
pub(super) struct OnePoleLowpass {
    coefficient: f32,
    state: f32,
}

impl OnePoleLowpass {
    pub(super) fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let coefficient = 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp();
        Self {
            coefficient: coefficient.clamp(0.0, 1.0),
            state: 0.0,
        }
    }

    pub(super) fn reset(&mut self) {
        self.state = 0.0;
    }

    pub(super) fn process(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input - self.state);
        self.state
    }
}

pub(super) struct OnePoleHighpass {
    lowpass: OnePoleLowpass,
}

impl OnePoleHighpass {
    pub(super) fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            lowpass: OnePoleLowpass::new(sample_rate, cutoff_hz),
        }
    }

    pub(super) fn reset(&mut self) {
        self.lowpass.reset();
    }

    pub(super) fn process(&mut self, input: f32) -> f32 {
        input - self.lowpass.process(input)
    }
}

#[derive(Default)]
pub(super) struct StateVariableFilter {
    low: f32,
    band: f32,
}

impl StateVariableFilter {
    pub(super) fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }

    pub(super) fn process_bandpass(
        &mut self,
        input: f32,
        center_hz: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> f32 {
        let cutoff = center_hz.clamp(20.0, sample_rate * 0.42);
        let f = (2.0 * (std::f32::consts::PI * cutoff / sample_rate).sin()).clamp(0.0, 1.92);
        let damping = (1.0 / resonance.max(0.35)).clamp(0.08, 1.8);
        let high = input - self.low - damping * self.band;
        self.band = (self.band + f * high).clamp(-32.0, 32.0);
        self.low = (self.low + f * self.band).clamp(-32.0, 32.0);
        self.band
    }
}

pub(super) fn hadamard8(input: [f32; 8]) -> [f32; 8] {
    const SCALE: f32 = 0.353_553_38;
    [
        (input[0] + input[1] + input[2] + input[3] + input[4] + input[5] + input[6] + input[7])
            * SCALE,
        (input[0] - input[1] + input[2] - input[3] + input[4] - input[5] + input[6] - input[7])
            * SCALE,
        (input[0] + input[1] - input[2] - input[3] + input[4] + input[5] - input[6] - input[7])
            * SCALE,
        (input[0] - input[1] - input[2] + input[3] + input[4] - input[5] - input[6] + input[7])
            * SCALE,
        (input[0] + input[1] + input[2] + input[3] - input[4] - input[5] - input[6] - input[7])
            * SCALE,
        (input[0] - input[1] + input[2] - input[3] - input[4] + input[5] - input[6] + input[7])
            * SCALE,
        (input[0] + input[1] - input[2] - input[3] - input[4] - input[5] + input[6] + input[7])
            * SCALE,
        (input[0] - input[1] - input[2] + input[3] - input[4] + input[5] + input[6] - input[7])
            * SCALE,
    ]
}
