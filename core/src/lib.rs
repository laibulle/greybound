pub mod amp;
pub mod audio_circuit;
pub mod chain;
pub mod circuit;
pub mod circuit_descriptor;
pub mod ir;
pub mod neural_cell;
pub mod pedal;
pub mod rig;

pub use amp::{
    configure_nox30_first_stage_graybox, configure_nox30_first_stage_neural, AmpControls,
    NeuralCellMode, Nox30OperatingPoint, StageBoundary, StageBoundaryState, StageCoupling,
    StageSignal, NOX30_STAGE_BOUNDARIES,
};
#[allow(deprecated)]
pub use amp::{
    ComponentBoundary, ComponentBoundaryState, ComponentCoupling, ComponentSignal,
    NOX30_COMPONENT_BOUNDARIES,
};
pub use audio_circuit::{
    AudioCircuitBlockDescriptor, AudioCircuitBlockKind, AudioCircuitDescriptor,
};
pub use chain::{
    amp_model_descriptor, AmpModelDescriptor, ControlDescriptor, ControlKind, DeviceConfig,
    DeviceControls, DeviceModelDescriptor, DeviceSlotConfig, DeviceSlotControls,
    DeviceVisualDescriptor, SignalChain, SignalChainConfig, SignalChainControls,
};
pub use circuit_descriptor::{
    amp_circuit_descriptor, device_circuit_descriptor, CircuitConfidence, CircuitControlBinding,
    CircuitDescriptor, CircuitDescriptorKind, CircuitEdgeDescriptor, CircuitGroupDescriptor,
    CircuitLayout, CircuitNodeDescriptor, CircuitNodeKind, CircuitSignalKind,
    CIRCUIT_DESCRIPTOR_SCHEMA, LUMEN_CIRCUIT, MINOTAUR_CIRCUIT, NOX30_CIRCUIT, SPRINGFIELD_CIRCUIT,
};
pub use pedal::{
    configure_minotaur_clip_neural, configure_minotaur_tone_neural, Auralith, AuralithControls,
    Brigade, BrigadeControls, Celeste, CelesteControls, ConnectionState, Dartford,
    DartfordControls, DartfordWave, ElectricalSignal, GodessOne, GodessOneControls, GodessOneMode,
    Jetstream, JetstreamControls, Load, Lumen, LumenControls, Minotaur, MinotaurControls,
    MinotaurNodeVoltages, Monarch, MonarchControls, Muffin, MuffinControls, Muon, MuonControls,
    Springfield, SpringfieldControls, StudioDelay, StudioDelayControls, StudioVerb,
    StudioVerbAlgorithm, StudioVerbControls, Tron, TronControls, LUMEN_AUDIO_CIRCUIT,
};
pub use rig::RigConfig;
