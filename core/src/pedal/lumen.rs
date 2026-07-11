use super::common::*;
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct LumenControls {
    pub peak_reduction: f32,
    pub gain: f32,
    pub emphasis: f32,
    pub mix: f32,
}

impl Default for LumenControls {
    fn default() -> Self {
        Self {
            peak_reduction: 0.42,
            gain: 0.50,
            emphasis: 0.44,
            mix: 0.82,
        }
    }
}

const LUMEN_INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
const LUMEN_OUTPUT_IMPEDANCE_OHMS: f32 = 1_200.0;

const LUMEN_AUDIO_CIRCUIT_BLOCKS: &[AudioCircuitBlockDescriptor] = &[
    AudioCircuitBlockDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: AudioCircuitBlockKind::InputCouplingHighpass,
        role: "DC blocking and low-end conditioning at the compressor input.",
    },
    AudioCircuitBlockDescriptor {
        id: "sidechain_filter",
        label: "Sidechain emphasis",
        kind: AudioCircuitBlockKind::SidechainHighpass,
        role: "Frequency-selective detector feed controlled by Emphasis.",
    },
    AudioCircuitBlockDescriptor {
        id: "level_detector",
        label: "Level detector",
        kind: AudioCircuitBlockKind::OptoLevelDetector,
        role: "Combines full-band and high-passed magnitude into the compressor detector.",
    },
    AudioCircuitBlockDescriptor {
        id: "opto_memory",
        label: "Opto memory",
        kind: AudioCircuitBlockKind::OptoGainMemory,
        role: "Program-dependent gain-reduction memory with peak-reduction controlled attack.",
    },
    AudioCircuitBlockDescriptor {
        id: "gain_cell",
        label: "Gain cell",
        kind: AudioCircuitBlockKind::GainCell,
        role: "Applies gain reduction and makeup gain to the coupled input voltage.",
    },
    AudioCircuitBlockDescriptor {
        id: "tube_softening",
        label: "Tube softening",
        kind: AudioCircuitBlockKind::TubeSoftClipper,
        role: "Soft nonlinear rounding after compression.",
    },
    AudioCircuitBlockDescriptor {
        id: "warm_filter",
        label: "Warm filter",
        kind: AudioCircuitBlockKind::ToneLowpass,
        role: "High-frequency smoothing on the compressed path.",
    },
    AudioCircuitBlockDescriptor {
        id: "parallel_mix",
        label: "Parallel mix",
        kind: AudioCircuitBlockKind::ParallelMixer,
        role: "Blends dry input voltage with the compressed/warmed path.",
    },
    AudioCircuitBlockDescriptor {
        id: "output_filter",
        label: "Output filter",
        kind: AudioCircuitBlockKind::OutputLowpass,
        role: "Final anti-harshness filter and output clamp before the low-Z boundary.",
    },
];

pub static LUMEN_AUDIO_CIRCUIT: AudioCircuitDescriptor = AudioCircuitDescriptor {
    model_id: "lumen",
    label: "Lumen compressor",
    source_of_truth: "executable-rust-audio-circuit",
    input_impedance_ohms: LUMEN_INPUT_IMPEDANCE_OHMS,
    output_impedance_ohms: LUMEN_OUTPUT_IMPEDANCE_OHMS,
    blocks: LUMEN_AUDIO_CIRCUIT_BLOCKS,
};

pub struct Lumen {
    input_connection: ConnectionState,
    circuit: LumenCircuitState,
}

struct LumenCircuitState {
    input_coupling: OnePoleHighpass,
    sidechain_highpass: OnePoleHighpass,
    tube_lowpass: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    sample_rate: f32,
    gain_reduction_db: f32,
}

impl Lumen {
    pub const INPUT_IMPEDANCE_OHMS: f32 = LUMEN_INPUT_IMPEDANCE_OHMS;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = LUMEN_OUTPUT_IMPEDANCE_OHMS;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 140e-12),
            circuit: LumenCircuitState::new(sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.circuit.reset();
    }

    pub fn audio_circuit_descriptor(&self) -> &'static AudioCircuitDescriptor {
        &LUMEN_AUDIO_CIRCUIT
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: LumenControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: LumenControls,
    ) -> ElectricalSignal {
        self.circuit.process_loaded_voltage(loaded_input, controls)
    }
}

#[derive(Clone, Copy, Debug)]
struct LumenCircuitFrame {
    loaded_input: f32,
    peak_reduction: f32,
    makeup: f32,
    emphasis: f32,
    mix: f32,
    input: f32,
    sidechain_hp: f32,
    detector: f32,
    target_reduction_db: f32,
    compressed: f32,
    softened: f32,
    warm: f32,
    output: f32,
}

impl LumenCircuitFrame {
    fn new(loaded_input: f32, controls: LumenControls) -> Self {
        Self {
            loaded_input,
            peak_reduction: controls.peak_reduction.clamp(0.0, 1.0),
            makeup: controls.gain.clamp(0.0, 1.0),
            emphasis: controls.emphasis.clamp(0.0, 1.0),
            mix: controls.mix.clamp(0.0, 1.0),
            input: 0.0,
            sidechain_hp: 0.0,
            detector: 1e-6,
            target_reduction_db: 0.0,
            compressed: 0.0,
            softened: 0.0,
            warm: 0.0,
            output: 0.0,
        }
    }
}

impl LumenCircuitState {
    fn new(sample_rate: f32) -> Self {
        Self {
            input_coupling: OnePoleHighpass::new(sample_rate, 18.0),
            sidechain_highpass: OnePoleHighpass::new(sample_rate, 115.0),
            tube_lowpass: OnePoleLowpass::new(sample_rate, 18_000.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 16_000.0),
            sample_rate,
            gain_reduction_db: 0.0,
        }
    }

    fn reset(&mut self) {
        self.input_coupling.reset();
        self.sidechain_highpass.reset();
        self.tube_lowpass.reset();
        self.output_lowpass.reset();
        self.gain_reduction_db = 0.0;
    }

    fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: LumenControls,
    ) -> ElectricalSignal {
        let output = self.process_audio_circuit(loaded_input, controls);
        ElectricalSignal::new(output, LUMEN_AUDIO_CIRCUIT.output_impedance_ohms)
    }
}

impl ExecutableAudioCircuit for LumenCircuitState {
    type Controls = LumenControls;
    type Frame = LumenCircuitFrame;

    const DESCRIPTOR: &'static AudioCircuitDescriptor = &LUMEN_AUDIO_CIRCUIT;

    fn prepare_audio_circuit_frame(
        &self,
        loaded_input: f32,
        controls: Self::Controls,
    ) -> Self::Frame {
        LumenCircuitFrame::new(loaded_input, controls)
    }

    fn audio_circuit_output_voltage(&self, frame: &Self::Frame) -> f32 {
        frame.output
    }
}

impl AudioCircuitBlockProcessor<LumenCircuitFrame> for LumenCircuitState {
    fn process_audio_circuit_block(
        &mut self,
        block: &'static AudioCircuitBlockDescriptor,
        frame: &mut LumenCircuitFrame,
    ) {
        match block.kind {
            AudioCircuitBlockKind::InputCouplingHighpass => {
                frame.input = self.input_coupling.process(frame.loaded_input);
            }
            AudioCircuitBlockKind::SidechainHighpass => {
                frame.sidechain_hp = self.sidechain_highpass.process(frame.input);
            }
            AudioCircuitBlockKind::OptoLevelDetector => {
                frame.detector = (frame.input.abs() * (1.0 - frame.emphasis * 0.36)
                    + frame.sidechain_hp.abs() * frame.emphasis * 0.92)
                    .max(1e-6);
            }
            AudioCircuitBlockKind::OptoGainMemory => {
                let threshold = 0.065 * 10.0_f32.powf(-frame.peak_reduction * 0.95);
                let over_db = (20.0 * (frame.detector / threshold).log10()).max(0.0);
                let knee = (over_db * over_db) / (over_db + 7.0);
                frame.target_reduction_db = (knee * (0.42 + frame.peak_reduction * 0.44)).min(24.0);

                let attack_ms = 3.5 + (1.0 - frame.peak_reduction) * 14.0;
                let release_ms = if frame.target_reduction_db > self.gain_reduction_db {
                    attack_ms
                } else {
                    95.0 + self.gain_reduction_db * 34.0
                };
                let coeff = time_coefficient(self.sample_rate, release_ms);
                self.gain_reduction_db +=
                    coeff * (frame.target_reduction_db - self.gain_reduction_db);
            }
            AudioCircuitBlockKind::GainCell => {
                let gain_reduction = 10.0_f32.powf(-self.gain_reduction_db / 20.0);
                let makeup_gain = 10.0_f32.powf((-1.0 + frame.makeup * 17.0) / 20.0);
                frame.compressed = frame.input * gain_reduction * makeup_gain;
            }
            AudioCircuitBlockKind::TubeSoftClipper => {
                let tube_drive = 1.04 + frame.peak_reduction * 0.22;
                frame.softened = (frame.compressed * tube_drive).tanh() / tube_drive;
            }
            AudioCircuitBlockKind::ToneLowpass => {
                frame.warm = self.tube_lowpass.process(frame.softened);
            }
            AudioCircuitBlockKind::ParallelMixer => {
                frame.output = frame.input * (1.0 - frame.mix) + frame.warm * frame.mix;
            }
            AudioCircuitBlockKind::OutputLowpass => {
                frame.output = self.output_lowpass.process(frame.output).clamp(-32.0, 32.0);
            }
        }
    }
}
