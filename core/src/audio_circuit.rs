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
