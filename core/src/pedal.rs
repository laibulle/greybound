use crate::amp::{NeuralCellMode, StageBoundaryState, StageCoupling};
use crate::audio_circuit::{
    AudioCircuitBlockDescriptor, AudioCircuitBlockKind, AudioCircuitBlockProcessor,
    AudioCircuitDescriptor, ExecutableAudioCircuit,
};
use crate::ir::SpeakerStage;
use crate::neural_cell::{ExperimentalNeuralCell, NeuralCellRuntime};
use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

mod auralith;
mod brigade;
mod celeste;
mod common;
mod dartford;
mod godess_one;
mod jetstream;
mod lumen;
mod minotaur;
mod monarch;
mod muffin;
mod muon;
mod springfield;
mod studio_delay;
mod studio_verb;
mod tron;

#[cfg(test)]
mod tests;

pub use auralith::{Auralith, AuralithControls};
pub use brigade::{Brigade, BrigadeControls};
pub use celeste::{Celeste, CelesteControls};
pub use common::{
    ConnectionState, ElectricalSignal, Load, AMP_INPUT_IMPEDANCE_OHMS, GUITAR_SOURCE_IMPEDANCE_OHMS,
};
pub use dartford::{Dartford, DartfordControls, DartfordWave};
pub use godess_one::{GodessOne, GodessOneControls, GodessOneMode};
pub use jetstream::{Jetstream, JetstreamControls};
pub use lumen::{Lumen, LumenControls, LUMEN_AUDIO_CIRCUIT};
pub use minotaur::{
    configure_minotaur_clip_neural, configure_minotaur_tone_neural, Minotaur, MinotaurControls,
    MinotaurNodeVoltages,
};
pub use monarch::{Monarch, MonarchControls};
pub use muffin::{Muffin, MuffinControls, MuffinNodeVoltages};
pub use muon::{Muon, MuonControls};
pub use springfield::{Springfield, SpringfieldControls};
pub use studio_delay::{StudioDelay, StudioDelayControls};
pub use studio_verb::{StudioVerb, StudioVerbAlgorithm, StudioVerbControls};
pub use tron::{Tron, TronControls};
