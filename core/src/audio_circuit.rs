use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioCircuitBlockKind {
    InputCouplingHighpass,
    SidechainHighpass,
    OptoLevelDetector,
    OptoGainMemory,
    GainCell,
    TubeSoftClipper,
    ToneLowpass,
    ParallelMixer,
    OutputLowpass,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct AudioCircuitBlockDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: AudioCircuitBlockKind,
    pub role: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct AudioCircuitDescriptor {
    pub model_id: &'static str,
    pub label: &'static str,
    pub source_of_truth: &'static str,
    pub input_impedance_ohms: f32,
    pub output_impedance_ohms: f32,
    pub blocks: &'static [AudioCircuitBlockDescriptor],
}

pub trait AudioCircuitBlockProcessor<Frame> {
    fn process_audio_circuit_block(
        &mut self,
        block: &'static AudioCircuitBlockDescriptor,
        frame: &mut Frame,
    );
}

pub fn run_audio_circuit<Frame, Processor>(
    descriptor: &'static AudioCircuitDescriptor,
    processor: &mut Processor,
    frame: &mut Frame,
) where
    Processor: AudioCircuitBlockProcessor<Frame>,
{
    for block in descriptor.blocks {
        processor.process_audio_circuit_block(block, frame);
    }
}

pub trait ExecutableAudioCircuit {
    type Controls;
    type Frame;

    const DESCRIPTOR: &'static AudioCircuitDescriptor;

    fn prepare_audio_circuit_frame(
        &self,
        loaded_input: f32,
        controls: Self::Controls,
    ) -> Self::Frame;

    fn audio_circuit_output_voltage(&self, frame: &Self::Frame) -> f32;

    fn process_audio_circuit(&mut self, loaded_input: f32, controls: Self::Controls) -> f32
    where
        Self: AudioCircuitBlockProcessor<Self::Frame> + Sized,
    {
        let mut frame = self.prepare_audio_circuit_frame(loaded_input, controls);
        run_audio_circuit(Self::DESCRIPTOR, self, &mut frame);
        self.audio_circuit_output_voltage(&frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BLOCKS: &[AudioCircuitBlockDescriptor] = &[
        AudioCircuitBlockDescriptor {
            id: "first",
            label: "First",
            kind: AudioCircuitBlockKind::InputCouplingHighpass,
            role: "first test block",
        },
        AudioCircuitBlockDescriptor {
            id: "second",
            label: "Second",
            kind: AudioCircuitBlockKind::OutputLowpass,
            role: "second test block",
        },
    ];

    static TEST_DESCRIPTOR: AudioCircuitDescriptor = AudioCircuitDescriptor {
        model_id: "test",
        label: "Test",
        source_of_truth: "unit-test",
        input_impedance_ohms: 1.0,
        output_impedance_ohms: 1.0,
        blocks: TEST_BLOCKS,
    };

    #[derive(Default)]
    struct TestProcessor {
        visited: Vec<&'static str>,
    }

    impl AudioCircuitBlockProcessor<Vec<&'static str>> for TestProcessor {
        fn process_audio_circuit_block(
            &mut self,
            block: &'static AudioCircuitBlockDescriptor,
            frame: &mut Vec<&'static str>,
        ) {
            self.visited.push(block.id);
            frame.push(block.id);
        }
    }

    #[test]
    fn run_audio_circuit_visits_blocks_in_descriptor_order() {
        let mut processor = TestProcessor::default();
        let mut frame = Vec::new();

        run_audio_circuit(&TEST_DESCRIPTOR, &mut processor, &mut frame);

        assert_eq!(frame, ["first", "second"]);
        assert_eq!(processor.visited, ["first", "second"]);
    }
}
