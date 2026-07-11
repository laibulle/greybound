use crate::chain::DeviceConfig;
use serde::Serialize;

pub const CIRCUIT_DESCRIPTOR_SCHEMA: &str = "greybound.circuit-descriptor.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitDescriptorKind {
    CircuitInformed,
    Greybox,
    GreyboxIrHybrid,
    Algorithmic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitConfidence {
    KnownBoundary,
    SchematicInspired,
    Inferred,
    TunedGreybox,
    ExternalReference,
    Algorithmic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitNodeKind {
    Port,
    InputLoad,
    CouplingFilter,
    Buffer,
    Split,
    CleanPath,
    GainStage,
    ClippingCell,
    ToneNetwork,
    LevelControl,
    OutputDriver,
    TransducerDriver,
    SpringTank,
    ImpulseResponse,
    DiffusionNetwork,
    Mixer,
    PhaseInverter,
    PowerStage,
    SupplyNetwork,
    Transformer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitSignalKind {
    AudioVoltage,
    LoadedAudioVoltage,
    BufferedAudio,
    DriveAudio,
    ClippedAudio,
    VoicedAudio,
    WetAudio,
    MixedAudio,
    PhaseSplitAudio,
    PowerAudio,
    RailVoltage,
    SpeakerVoltage,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitLayout {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitNodeDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: CircuitNodeKind,
    pub role: &'static str,
    pub control_id: Option<&'static str>,
    pub confidence: CircuitConfidence,
    pub implementation: &'static str,
    pub algorithm: &'static str,
    pub layout: CircuitLayout,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitEdgeDescriptor {
    pub from: &'static str,
    pub to: &'static str,
    pub signal: CircuitSignalKind,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitGroupDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub nodes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitControlBinding {
    pub control_id: &'static str,
    pub node_id: &'static str,
    pub role: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CircuitDescriptor {
    pub schema: &'static str,
    pub model_id: &'static str,
    pub label: &'static str,
    pub kind: CircuitDescriptorKind,
    pub source_of_truth: &'static str,
    pub implementation: &'static str,
    pub summary: &'static str,
    pub nodes: &'static [CircuitNodeDescriptor],
    pub edges: &'static [CircuitEdgeDescriptor],
    pub groups: &'static [CircuitGroupDescriptor],
    pub controls: &'static [CircuitControlBinding],
    pub notes: &'static [&'static str],
}

pub fn device_circuit_descriptor(device: DeviceConfig) -> Option<&'static CircuitDescriptor> {
    match device {
        DeviceConfig::Lumen => Some(&LUMEN_CIRCUIT),
        DeviceConfig::Minotaur => Some(&MINOTAUR_CIRCUIT),
        DeviceConfig::Springfield => Some(&SPRINGFIELD_CIRCUIT),
        _ => None,
    }
}

pub fn amp_circuit_descriptor(model: &str) -> Option<&'static CircuitDescriptor> {
    let model_base = model.split_once('?').map_or(model, |(base, _)| base);
    match model_base {
        "nox30" | "nox30-experimental" => Some(&NOX30_CIRCUIT),
        "none-star" | "lonestar-special" | "lone-star-special" | "lonestar" => {
            Some(&NONE_STAR_CIRCUIT)
        }
        "boxer-seven-lead" | "boxer-seven" | "shiva-20th-lead" | "shiva20-lead" => {
            Some(&BOXER_SEVEN_LEAD_CIRCUIT)
        }
        _ => None,
    }
}

const NOX30_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_volume", "first_stage"];
const NOX30_TONE_GROUP_NODES: &[&str] = &["cathode_follower", "tone_stack"];
const NOX30_PREAMP_GROUP_NODES: &[&str] = &["drive_stage", "recovery_stage"];
const NOX30_POWER_GROUP_NODES: &[&str] = &[
    "phase_inverter",
    "cut_presence",
    "power_stage",
    "supply_network",
    "output_transformer",
    "speaker_out",
];

const NOX30_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input and first gain",
        nodes: NOX30_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "top_boost",
        label: "Follower and Top Boost stack",
        nodes: NOX30_TONE_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "preamp",
        label: "Drive and recovery",
        nodes: NOX30_PREAMP_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "power",
        label: "Phase inverter, rails, and output",
        nodes: NOX30_POWER_GROUP_NODES,
    },
];

const NOX30_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Guitar, pedalboard, or FX return voltage enters the Nox30 boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp/models/nox30.rs::Nox30",
        algorithm: "SignalChain hands audio voltage and electrical boundary context to the amp.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_volume",
        label: "Input volume",
        kind: CircuitNodeKind::LevelControl,
        role: "Input coupling, volume attenuation, and bright bypass behavior before V1.",
        control_id: Some("volume"),
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::BrightVolumeInputStage",
        algorithm: "Frequency-dependent gain from the volume control and bright-cap approximation.",
        layout: CircuitLayout { x: 0.14, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "first_stage",
        label: "First ECC83",
        kind: CircuitNodeKind::GainStage,
        role: "First nonlinear common-cathode gain stage with cathode bypass behavior.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::CommonCathodeStage",
        algorithm: "Circuit-informed nonlinear triode approximation with observable plate current and cathode voltage.",
        layout: CircuitLayout { x: 0.25, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "cathode_follower",
        label: "Cathode follower",
        kind: CircuitNodeKind::Buffer,
        role: "Low source impedance follower that drives the Top Boost tone network.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::CathodeFollowerStage",
        algorithm: "Follower approximation exposes output voltage, source impedance assumption, plate current, and cathode voltage.",
        layout: CircuitLayout { x: 0.36, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "tone_stack",
        label: "Top Boost stack",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Passive Top Boost bass/treble network driven by the cathode follower.",
        control_id: Some("bass"),
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp/components.rs::TopBoostToneStack",
        algorithm: "Trapezoidal MNA solve with source/load assumptions.",
        layout: CircuitLayout { x: 0.47, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "drive_stage",
        label: "Drive stage",
        kind: CircuitNodeKind::GainStage,
        role: "Optional extra nonlinear preamp drive after the tone stack.",
        control_id: Some("drive"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/nox30.rs::CommonCathodeStage",
        algorithm: "Drive control pushes a second common-cathode approximation and blends it with the tone output.",
        layout: CircuitLayout { x: 0.58, y: 0.39 },
    },
    CircuitNodeDescriptor {
        id: "recovery_stage",
        label: "Recovery",
        kind: CircuitNodeKind::GainStage,
        role: "Post-drive nonlinear recovery stage feeding the phase inverter.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/nox30.rs::CommonCathodeStage",
        algorithm: "Recovery common-cathode approximation after the drive path.",
        layout: CircuitLayout { x: 0.58, y: 0.61 },
    },
    CircuitNodeDescriptor {
        id: "phase_inverter",
        label: "Phase inverter",
        kind: CircuitNodeKind::PhaseInverter,
        role: "Long-tail-pair phase inverter producing opposed power-stage drive.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::LongTailPairStage",
        algorithm: "Opposed nonlinear phases with exposed plate currents and cathode voltage.",
        layout: CircuitLayout { x: 0.70, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "cut_presence",
        label: "Cut / presence",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Cut and presence shaping around the phase-inverter/power-stage boundary.",
        control_id: Some("cut"),
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::CutPresenceStage",
        algorithm: "Cut and presence filters act on the phase-inverter output before power-stage drive.",
        layout: CircuitLayout { x: 0.78, y: 0.28 },
    },
    CircuitNodeDescriptor {
        id: "power_stage",
        label: "EL84 output",
        kind: CircuitNodeKind::PowerStage,
        role: "Cathode-biased push-pull EL84 power stage with screen and cathode-bias telemetry.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::PushPullEl84Stage",
        algorithm: "Push-pull saturation, dynamic cathode-bias shift, screen current, and attack current.",
        layout: CircuitLayout { x: 0.84, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "supply_network",
        label: "B+ supply",
        kind: CircuitNodeKind::SupplyNetwork,
        role: "Shared rail state that consumes current demand and publishes sagging preamp, phase-inverter, and power voltages.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/nox30.rs::SupplyNetwork",
        algorithm: "B+ sag and recovery from stage current demand; shared state, not a normal audio-series block.",
        layout: CircuitLayout { x: 0.70, y: 0.82 },
    },
    CircuitNodeDescriptor {
        id: "output_transformer",
        label: "Output transformer",
        kind: CircuitNodeKind::Transformer,
        role: "Transformer output filtering, core-flux state, and low-impedance speaker boundary.",
        control_id: Some("output"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/nox30.rs::OutputTransformerStage",
        algorithm: "Transformer high-pass/low-pass filtering, compression, and core-flux state.",
        layout: CircuitLayout { x: 0.94, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "speaker_out",
        label: "Speaker out",
        kind: CircuitNodeKind::Port,
        role: "Amp output voltage leaves the Nox30 boundary before optional cab IR convolution.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp.rs::VoxAmp",
        algorithm: "The output transformer sample is handed to the chain or speaker IR boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.50 },
    },
];

const NOX30_EDGES: &[CircuitEdgeDescriptor] = &[
    edge(
        "input_jack",
        "input_volume",
        CircuitSignalKind::AudioVoltage,
    ),
    edge(
        "input_volume",
        "first_stage",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge(
        "first_stage",
        "cathode_follower",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "cathode_follower",
        "tone_stack",
        CircuitSignalKind::BufferedAudio,
    ),
    edge("tone_stack", "drive_stage", CircuitSignalKind::VoicedAudio),
    edge(
        "drive_stage",
        "recovery_stage",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "recovery_stage",
        "phase_inverter",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "phase_inverter",
        "cut_presence",
        CircuitSignalKind::PhaseSplitAudio,
    ),
    edge(
        "cut_presence",
        "power_stage",
        CircuitSignalKind::PhaseSplitAudio,
    ),
    edge(
        "power_stage",
        "output_transformer",
        CircuitSignalKind::PowerAudio,
    ),
    edge(
        "output_transformer",
        "speaker_out",
        CircuitSignalKind::SpeakerVoltage,
    ),
    edge(
        "supply_network",
        "first_stage",
        CircuitSignalKind::RailVoltage,
    ),
    edge(
        "supply_network",
        "drive_stage",
        CircuitSignalKind::RailVoltage,
    ),
    edge(
        "supply_network",
        "phase_inverter",
        CircuitSignalKind::RailVoltage,
    ),
    edge(
        "supply_network",
        "power_stage",
        CircuitSignalKind::RailVoltage,
    ),
];

const NOX30_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "volume",
        node_id: "input_volume",
        role: "Sets input gain and bright-bypass behavior.",
    },
    CircuitControlBinding {
        control_id: "bass",
        node_id: "tone_stack",
        role: "Sets low-frequency branch of the Top Boost tone stack.",
    },
    CircuitControlBinding {
        control_id: "treble",
        node_id: "tone_stack",
        role: "Sets high-frequency branch of the Top Boost tone stack.",
    },
    CircuitControlBinding {
        control_id: "cut",
        node_id: "cut_presence",
        role: "Sets high-frequency cut around the phase-inverter/power boundary.",
    },
    CircuitControlBinding {
        control_id: "drive",
        node_id: "drive_stage",
        role: "Sets the optional extra preamp drive path.",
    },
    CircuitControlBinding {
        control_id: "presence",
        node_id: "cut_presence",
        role: "Sets presence shaping before the power stage.",
    },
    CircuitControlBinding {
        control_id: "sag",
        node_id: "supply_network",
        role: "Scales dynamic rail sag and recovery.",
    },
    CircuitControlBinding {
        control_id: "output",
        node_id: "output_transformer",
        role: "Sets final amp output scaling.",
    },
];

const NOX30_NOTES: &[&str] = &[
    "This is a runtime descriptor for the current Nox30 model; nox30-experimental is a legacy alias.",
    "The supply network is shared state: it consumes component current demand and publishes rail voltages.",
    "The descriptor is a stage-boundary graph, not a PCB layout or full SPICE netlist.",
];

pub static NOX30_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "nox30",
    label: "Nox30 circuit-informed amp",
    kind: CircuitDescriptorKind::CircuitInformed,
    source_of_truth: "rust-model-component-boundaries",
    implementation: "core/src/amp/models/nox30.rs::Nox30",
    summary: "Input volume and bright bypass into ECC83 preamp, cathode follower, Top Boost MNA tone stack, drive/recovery stages, long-tail-pair phase inverter, cut/presence shaping, EL84 push-pull power stage, shared B+ sag, and output transformer.",
    nodes: NOX30_NODES,
    edges: NOX30_EDGES,
    groups: NOX30_GROUPS,
    controls: NOX30_CONTROLS,
    notes: NOX30_NOTES,
};

const NONE_STAR_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_coupling", "clean_preamp"];
const NONE_STAR_TONE_GROUP_NODES: &[&str] = &["tone_stack", "recovery"];
const NONE_STAR_POWER_GROUP_NODES: &[&str] = &[
    "phase_inverter",
    "presence",
    "class_a_el84",
    "sag_bias",
    "output_transformer",
    "speaker_out",
];

const NONE_STAR_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input and clean preamp",
        nodes: NONE_STAR_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "tone",
        label: "Tone stack and recovery",
        nodes: NONE_STAR_TONE_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "power",
        label: "Class A EL84 power response",
        nodes: NONE_STAR_POWER_GROUP_NODES,
    },
];

const NONE_STAR_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Guitar or pedalboard voltage enters the None Star Clean/Edge boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp/models/none_star.rs::NoneStar",
        algorithm: "SignalChain hands audio voltage to the amp model.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: CircuitNodeKind::CouplingFilter,
        role: "High impedance input coupling before the first clean gain stage.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/none_star.rs::WdfHighpass",
        algorithm: "RC high-pass boundary with 1 MOhm input assumption.",
        layout: CircuitLayout { x: 0.16, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "clean_preamp",
        label: "Clean / edge preamp",
        kind: CircuitNodeKind::GainStage,
        role: "Two-stage clean preamp that moves into soft edge breakup as volume/drive rise.",
        control_id: Some("volume"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::NoneStar",
        algorithm:
            "Clean-channel flow: fixed high-headroom input triode, passive tone network, then gain/recovery drive.",
        layout: CircuitLayout { x: 0.30, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "tone_stack",
        label: "Bass / mid / treble",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Clean-amp tone shaping with bass, mid, and treble bands.",
        control_id: Some("bass"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::tone_stack",
        algorithm:
            "Three-band low/mid/high graybox filter; mid currently maps to AmpControls::cut.",
        layout: CircuitLayout { x: 0.46, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "recovery",
        label: "Recovery",
        kind: CircuitNodeKind::GainStage,
        role: "Post-tone recovery stage that feeds the phase inverter.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::triode_stage",
        algorithm: "Soft triode-stage approximation after coupling.",
        layout: CircuitLayout { x: 0.58, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "phase_inverter",
        label: "Phase inverter",
        kind: CircuitNodeKind::PhaseInverter,
        role: "Phase-split drive into the 6L6-style power response.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::NoneStar",
        algorithm: "Opposed soft triode-stage pair.",
        layout: CircuitLayout { x: 0.70, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "presence",
        label: "Presence",
        kind: CircuitNodeKind::ToneNetwork,
        role: "High-frequency feedback/presence shaping before the output section.",
        control_id: Some("presence"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::OnePoleLowpass",
        algorithm: "Variable high-frequency lift around a low-pass reference.",
        layout: CircuitLayout { x: 0.78, y: 0.32 },
    },
    CircuitNodeDescriptor {
        id: "power_6l6",
        label: "6L6 power",
        kind: CircuitNodeKind::PowerStage,
        role: "Lone Star Classic-inspired 6L6 output response for the Clean/Edge target.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/circuit/power.rs::PushPull6L6Stage",
        algorithm:
            "Structured push-pull 6L6 fallback with supply, screen, fixed-bias, bias-shift, load-current, and attack-current state.",
        layout: CircuitLayout { x: 0.84, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "sag_bias",
        label: "Sag / bias",
        kind: CircuitNodeKind::SupplyNetwork,
        role: "Slow dynamic feel from current demand, bias shift, and supply sag.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::EnvelopeFollower",
        algorithm: "Upstream envelope followers feed the structured 6L6 stage with bias and sag control signals.",
        layout: CircuitLayout { x: 0.72, y: 0.82 },
    },
    CircuitNodeDescriptor {
        id: "output_transformer",
        label: "Output transformer",
        kind: CircuitNodeKind::Transformer,
        role: "Output filtering and saturation before cab/IR.",
        control_id: Some("output"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/none_star.rs::WdfHighpass + OnePoleLowpass",
        algorithm: "Low-frequency coupling and high-frequency rolloff around the power response.",
        layout: CircuitLayout { x: 0.94, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "speaker_out",
        label: "Speaker out",
        kind: CircuitNodeKind::Port,
        role: "Amp output voltage leaves the boundary before optional cab IR convolution.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp.rs::VoxAmp",
        algorithm: "The output sample is handed to the chain or speaker IR boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.50 },
    },
];

const NONE_STAR_EDGES: &[CircuitEdgeDescriptor] = &[
    edge(
        "input_jack",
        "input_coupling",
        CircuitSignalKind::AudioVoltage,
    ),
    edge(
        "input_coupling",
        "clean_preamp",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge("clean_preamp", "tone_stack", CircuitSignalKind::DriveAudio),
    edge("tone_stack", "recovery", CircuitSignalKind::VoicedAudio),
    edge("recovery", "phase_inverter", CircuitSignalKind::DriveAudio),
    edge(
        "phase_inverter",
        "presence",
        CircuitSignalKind::PhaseSplitAudio,
    ),
    edge("presence", "power_6l6", CircuitSignalKind::PhaseSplitAudio),
    edge(
        "power_6l6",
        "output_transformer",
        CircuitSignalKind::PowerAudio,
    ),
    edge(
        "output_transformer",
        "speaker_out",
        CircuitSignalKind::SpeakerVoltage,
    ),
    edge("sag_bias", "power_6l6", CircuitSignalKind::RailVoltage),
    edge("sag_bias", "clean_preamp", CircuitSignalKind::RailVoltage),
];

const NONE_STAR_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "volume",
        node_id: "clean_preamp",
        role: "Sets clean input level and transition into edge breakup.",
    },
    CircuitControlBinding {
        control_id: "bass",
        node_id: "tone_stack",
        role: "Sets the low band of the graybox tone stack.",
    },
    CircuitControlBinding {
        control_id: "cut",
        node_id: "tone_stack",
        role: "Temporarily maps to midrange level for the Clean/Edge target.",
    },
    CircuitControlBinding {
        control_id: "treble",
        node_id: "tone_stack",
        role: "Sets the high band of the graybox tone stack.",
    },
    CircuitControlBinding {
        control_id: "presence",
        node_id: "presence",
        role: "Sets high-frequency presence before the output section.",
    },
    CircuitControlBinding {
        control_id: "sag",
        node_id: "sag_bias",
        role: "Sets dynamic supply/bias softness.",
    },
    CircuitControlBinding {
        control_id: "output",
        node_id: "output_transformer",
        role: "Sets final amp output scaling.",
    },
];

const NONE_STAR_NOTES: &[&str] = &[
    "This V1 targets None Star Clean/Edge behavior inspired by a Lone Star-style full-rig reference, not the full two-channel Drive mode.",
    "The model is a tuned graybox approximation; it is not a Mesa/Boogie schematic clone.",
    "AmpControls::cut is temporarily used as a mid control until the shared amp UI grows a dedicated Mid parameter.",
];

pub static NONE_STAR_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "none-star",
    label: "None Star Clean/Edge",
    kind: CircuitDescriptorKind::Greybox,
    source_of_truth: "rust-model-stage-boundaries",
    implementation: "core/src/amp/models/none_star.rs::NoneStar",
    summary: "Lone Star-style clean-channel-inspired input triode, passive three-band tone shaping before gain/recovery, loop/master/presence shaping, phase split, 6L6-style power response, dynamic sag/bias feel, and output filtering.",
    nodes: NONE_STAR_NODES,
    edges: NONE_STAR_EDGES,
    groups: NONE_STAR_GROUPS,
    controls: NONE_STAR_CONTROLS,
    notes: NONE_STAR_NOTES,
};

const BOXER_SEVEN_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_coupling", "bright_gain"];
const BOXER_SEVEN_PREAMP_GROUP_NODES: &[&str] =
    &["cascaded_gain", "cold_clipper", "recovery_follower"];
const BOXER_SEVEN_TONE_GROUP_NODES: &[&str] = &["tone_stack", "loop_master", "presence"];
const BOXER_SEVEN_POWER_GROUP_NODES: &[&str] = &[
    "phase_inverter",
    "kt88_power",
    "sag_excursion",
    "output_transformer",
    "speaker_out",
];

const BOXER_SEVEN_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input and bright gain",
        nodes: BOXER_SEVEN_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "lead_preamp",
        label: "Lead cascade",
        nodes: BOXER_SEVEN_PREAMP_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "tone",
        label: "Tone, loop, and presence",
        nodes: BOXER_SEVEN_TONE_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "power",
        label: "Phase inverter and KT-style output",
        nodes: BOXER_SEVEN_POWER_GROUP_NODES,
    },
];

const BOXER_SEVEN_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Guitar or pedalboard voltage enters the Boxer Seven Lead boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp/models/boxer_seven.rs::BoxerSevenLead",
        algorithm: "SignalChain hands audio voltage to the amp model.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: CircuitNodeKind::CouplingFilter,
        role: "High-impedance lead-channel input coupling before the gain cascade.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/amp/models/boxer_seven.rs::WdfHighpass",
        algorithm: "RC high-pass boundary with 1 MOhm input assumption.",
        layout: CircuitLayout { x: 0.14, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "bright_gain",
        label: "Bright gain",
        kind: CircuitNodeKind::GainStage,
        role: "Lead gain input stage with bright emphasis that recedes as gain rises.",
        control_id: Some("volume"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::BoxerSevenLead",
        algorithm: "Input triode approximation plus frequency-dependent bright lift.",
        layout: CircuitLayout { x: 0.25, y: 0.42 },
    },
    CircuitNodeDescriptor {
        id: "cascaded_gain",
        label: "Cascade",
        kind: CircuitNodeKind::GainStage,
        role: "Cascaded tube gain that establishes the compressed lead voice.",
        control_id: Some("volume"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::triode_stage",
        algorithm:
            "Two soft triode stages with interstage coupling and level-dependent compression.",
        layout: CircuitLayout { x: 0.38, y: 0.40 },
    },
    CircuitNodeDescriptor {
        id: "cold_clipper",
        label: "Lead clipper",
        kind: CircuitNodeKind::ClippingCell,
        role: "Asymmetric clipping/voicing cell for the lead-channel saturation.",
        control_id: Some("drive"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::triode_stage",
        algorithm:
            "Cold-biased triode approximation; drive raises gain, compression, and bypass emphasis.",
        layout: CircuitLayout { x: 0.50, y: 0.40 },
    },
    CircuitNodeDescriptor {
        id: "recovery_follower",
        label: "Recovery / follower",
        kind: CircuitNodeKind::Buffer,
        role: "Recovery gain and follower-like low source impedance into the tone network.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::cathode_follower",
        algorithm: "Soft recovery stage followed by cathode-follower approximation.",
        layout: CircuitLayout { x: 0.60, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "tone_stack",
        label: "Bass / mid / treble",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Lead-channel three-band voicing with tight low end and forward mids.",
        control_id: Some("bass"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::tone_stack",
        algorithm: "Three-band graybox filter; mid currently maps to AmpControls::cut.",
        layout: CircuitLayout { x: 0.70, y: 0.42 },
    },
    CircuitNodeDescriptor {
        id: "loop_master",
        label: "Loop / channel volume",
        kind: CircuitNodeKind::OutputDriver,
        role: "Post-tone recovery and loop/master driver before phase-inverter shaping.",
        control_id: Some("output"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::BoxerSevenLead",
        algorithm: "Coupled driver stage into the phase inverter.",
        layout: CircuitLayout { x: 0.78, y: 0.52 },
    },
    CircuitNodeDescriptor {
        id: "presence",
        label: "Presence",
        kind: CircuitNodeKind::ToneNetwork,
        role: "High-frequency feedback-style lift before the output section.",
        control_id: Some("presence"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::OnePoleLowpass",
        algorithm: "Variable high-frequency lift around a low-pass reference.",
        layout: CircuitLayout { x: 0.82, y: 0.26 },
    },
    CircuitNodeDescriptor {
        id: "phase_inverter",
        label: "Phase inverter",
        kind: CircuitNodeKind::PhaseInverter,
        role: "Opposed drive for the high-headroom output response.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::BoxerSevenLead",
        algorithm: "Opposed soft triode-stage pair.",
        layout: CircuitLayout { x: 0.88, y: 0.52 },
    },
    CircuitNodeDescriptor {
        id: "kt88_power",
        label: "KT-style power",
        kind: CircuitNodeKind::PowerStage,
        role: "High-headroom fixed-bias-style power response for the lead target.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::six_l6_bank",
        algorithm:
            "Current-bank approximation tuned for tight KT88-like headroom rather than a component-exact power stage.",
        layout: CircuitLayout { x: 0.93, y: 0.52 },
    },
    CircuitNodeDescriptor {
        id: "sag_excursion",
        label: "Sag / excursion",
        kind: CircuitNodeKind::SupplyNetwork,
        role: "Low-end excursion and slow supply/bias compression.",
        control_id: Some("sag"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::EnvelopeFollower",
        algorithm: "Envelope followers and low-frequency shaping soften the high-headroom output path.",
        layout: CircuitLayout { x: 0.86, y: 0.82 },
    },
    CircuitNodeDescriptor {
        id: "output_transformer",
        label: "Output transformer",
        kind: CircuitNodeKind::Transformer,
        role: "Output filtering and saturation before cab/IR.",
        control_id: Some("output"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/amp/models/boxer_seven.rs::WdfHighpass + OnePoleLowpass",
        algorithm: "Low-frequency coupling and high-frequency rolloff around the power response.",
        layout: CircuitLayout { x: 0.97, y: 0.52 },
    },
    CircuitNodeDescriptor {
        id: "speaker_out",
        label: "Speaker out",
        kind: CircuitNodeKind::Port,
        role: "Amp output voltage leaves the boundary before optional cab IR convolution.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/amp.rs::VoxAmp",
        algorithm: "The output sample is handed to the chain or speaker IR boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.52 },
    },
];

const BOXER_SEVEN_EDGES: &[CircuitEdgeDescriptor] = &[
    edge(
        "input_jack",
        "input_coupling",
        CircuitSignalKind::AudioVoltage,
    ),
    edge(
        "input_coupling",
        "bright_gain",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge(
        "bright_gain",
        "cascaded_gain",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "cascaded_gain",
        "cold_clipper",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "cold_clipper",
        "recovery_follower",
        CircuitSignalKind::ClippedAudio,
    ),
    edge(
        "recovery_follower",
        "tone_stack",
        CircuitSignalKind::BufferedAudio,
    ),
    edge("tone_stack", "loop_master", CircuitSignalKind::VoicedAudio),
    edge("loop_master", "presence", CircuitSignalKind::DriveAudio),
    edge("presence", "phase_inverter", CircuitSignalKind::DriveAudio),
    edge(
        "phase_inverter",
        "kt88_power",
        CircuitSignalKind::PhaseSplitAudio,
    ),
    edge(
        "kt88_power",
        "output_transformer",
        CircuitSignalKind::PowerAudio,
    ),
    edge(
        "output_transformer",
        "speaker_out",
        CircuitSignalKind::SpeakerVoltage,
    ),
    edge(
        "sag_excursion",
        "kt88_power",
        CircuitSignalKind::RailVoltage,
    ),
    edge("sag_excursion", "presence", CircuitSignalKind::RailVoltage),
];

const BOXER_SEVEN_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "volume",
        node_id: "bright_gain",
        role: "Sets lead-channel gain into the cascade.",
    },
    CircuitControlBinding {
        control_id: "drive",
        node_id: "cold_clipper",
        role: "Maps to the V1 mode/boost hypothesis: more clipping gain, compression, and upper-mid bite.",
    },
    CircuitControlBinding {
        control_id: "bass",
        node_id: "tone_stack",
        role: "Sets the low band of the graybox tone stack.",
    },
    CircuitControlBinding {
        control_id: "cut",
        node_id: "tone_stack",
        role: "Temporarily maps to midrange level for Boxer Seven Lead.",
    },
    CircuitControlBinding {
        control_id: "treble",
        node_id: "tone_stack",
        role: "Sets the high band of the graybox tone stack.",
    },
    CircuitControlBinding {
        control_id: "presence",
        node_id: "presence",
        role: "Sets high-frequency presence before the output section.",
    },
    CircuitControlBinding {
        control_id: "sag",
        node_id: "sag_excursion",
        role: "Sets low-end excursion and supply/bias softness.",
    },
    CircuitControlBinding {
        control_id: "output",
        node_id: "output_transformer",
        role: "Sets final amp output scaling.",
    },
];

const BOXER_SEVEN_NOTES: &[&str] = &[
    "This V1 targets the Boxer Seven Lead behavior inspired by the Bogner Shiva 20th Anniversary lead channel, not a component-exact schematic clone.",
    "The model is a tuned graybox approximation based on public control/topology information and Greybound amp-stage primitives.",
    "AmpControls::cut is temporarily used as a mid control until the shared amp UI grows a dedicated Mid parameter.",
    "AmpControls::drive represents the first-pass lead mode/boost aggression hypothesis rather than a separate physical front-panel pot.",
];

pub static BOXER_SEVEN_LEAD_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "boxer-seven-lead",
    label: "Boxer Seven Lead",
    kind: CircuitDescriptorKind::Greybox,
    source_of_truth: "rust-model-stage-boundaries",
    implementation: "core/src/amp/models/boxer_seven.rs::BoxerSevenLead",
    summary: "Shiva-20th-inspired lead-channel graybox with bright input gain, cascaded tube-style saturation, asymmetric lead clipper, recovery/follower into a three-band tone stack, loop/master driver, presence, high-headroom KT-style power response, sag/excursion feel, and output filtering.",
    nodes: BOXER_SEVEN_NODES,
    edges: BOXER_SEVEN_EDGES,
    groups: BOXER_SEVEN_GROUPS,
    controls: BOXER_SEVEN_CONTROLS,
    notes: BOXER_SEVEN_NOTES,
};

const LUMEN_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_load", "input_coupling"];
const LUMEN_SIDECHAIN_GROUP_NODES: &[&str] = &["sidechain_filter", "level_detector", "opto_memory"];
const LUMEN_GAIN_GROUP_NODES: &[&str] = &["gain_cell", "tube_softening", "warm_filter"];
const LUMEN_OUTPUT_GROUP_NODES: &[&str] = &["parallel_mix", "output_filter", "output_jack"];

const LUMEN_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input boundary",
        nodes: LUMEN_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "sidechain",
        label: "Sidechain and opto memory",
        nodes: LUMEN_SIDECHAIN_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "gain_path",
        label: "Gain cell and softening",
        nodes: LUMEN_GAIN_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "output",
        label: "Parallel mix and output",
        nodes: LUMEN_OUTPUT_GROUP_NODES,
    },
];

const LUMEN_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Guitar or previous pedal voltage enters the Lumen boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Voltage plus source impedance enters the pedal slot.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_load",
        label: "Buffered input load",
        kind: CircuitNodeKind::InputLoad,
        role: "High input impedance and cable/source loading before compression.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::Lumen",
        algorithm: "ConnectionState source/load division against Lumen::INPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.16, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: CircuitNodeKind::CouplingFilter,
        role: "DC blocking and low-frequency conditioning at the compressor input.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block InputCouplingHighpass.",
        layout: CircuitLayout { x: 0.28, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "sidechain_filter",
        label: "Sidechain emphasis",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Frequency-selective detector feed that avoids low-end over-triggering.",
        control_id: Some("emphasis"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block SidechainHighpass blended into the detector by Emphasis.",
        layout: CircuitLayout { x: 0.42, y: 0.28 },
    },
    CircuitNodeDescriptor {
        id: "level_detector",
        label: "Level detector",
        kind: CircuitNodeKind::LevelControl,
        role: "Rectifies full-band and emphasized sidechain energy into compression demand.",
        control_id: Some("peak_reduction"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block OptoLevelDetector.",
        layout: CircuitLayout { x: 0.55, y: 0.28 },
    },
    CircuitNodeDescriptor {
        id: "opto_memory",
        label: "Opto memory",
        kind: CircuitNodeKind::LevelControl,
        role: "Program-dependent attack/release memory that stores gain reduction in dB.",
        control_id: Some("peak_reduction"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LumenCircuitState",
        algorithm: "Executable block OptoGainMemory updates gain_reduction_db.",
        layout: CircuitLayout { x: 0.68, y: 0.28 },
    },
    CircuitNodeDescriptor {
        id: "gain_cell",
        label: "Gain cell",
        kind: CircuitNodeKind::GainStage,
        role: "Applies opto-style gain reduction and makeup gain to the audio path.",
        control_id: Some("gain"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block GainCell uses gain_reduction_db and the Gain control.",
        layout: CircuitLayout { x: 0.55, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "tube_softening",
        label: "Tube softening",
        kind: CircuitNodeKind::ClippingCell,
        role: "Rounds compressed peaks with a gentle tube-style nonlinearity.",
        control_id: Some("peak_reduction"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block TubeSoftClipper uses tanh softening.",
        layout: CircuitLayout { x: 0.68, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "warm_filter",
        label: "Warm filter",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Smooths the compressed path before the dry/wet mixer.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block ToneLowpass.",
        layout: CircuitLayout { x: 0.80, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "parallel_mix",
        label: "Parallel mix",
        kind: CircuitNodeKind::Mixer,
        role: "Blends the dry coupled input with the compressed and warmed path.",
        control_id: Some("mix"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block ParallelMixer.",
        layout: CircuitLayout { x: 0.90, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "output_filter",
        label: "Output filter",
        kind: CircuitNodeKind::OutputDriver,
        role: "Final smoothing and low source impedance output boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
        algorithm: "Executable block OutputLowpass emits Lumen::OUTPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.97, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "output_jack",
        label: "Output jack",
        kind: CircuitNodeKind::Port,
        role: "Compressed output voltage and source impedance leave the Lumen boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Signal is handed back to the chain electrical boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.50 },
    },
];

const LUMEN_EDGES: &[CircuitEdgeDescriptor] = &[
    edge("input_jack", "input_load", CircuitSignalKind::AudioVoltage),
    edge(
        "input_load",
        "input_coupling",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge(
        "input_coupling",
        "sidechain_filter",
        CircuitSignalKind::BufferedAudio,
    ),
    edge(
        "sidechain_filter",
        "level_detector",
        CircuitSignalKind::VoicedAudio,
    ),
    edge(
        "level_detector",
        "opto_memory",
        CircuitSignalKind::VoicedAudio,
    ),
    edge("opto_memory", "gain_cell", CircuitSignalKind::DriveAudio),
    edge(
        "input_coupling",
        "gain_cell",
        CircuitSignalKind::BufferedAudio,
    ),
    edge("gain_cell", "tube_softening", CircuitSignalKind::DriveAudio),
    edge(
        "tube_softening",
        "warm_filter",
        CircuitSignalKind::ClippedAudio,
    ),
    edge("warm_filter", "parallel_mix", CircuitSignalKind::WetAudio),
    edge(
        "input_coupling",
        "parallel_mix",
        CircuitSignalKind::BufferedAudio,
    ),
    edge(
        "parallel_mix",
        "output_filter",
        CircuitSignalKind::MixedAudio,
    ),
    edge(
        "output_filter",
        "output_jack",
        CircuitSignalKind::AudioVoltage,
    ),
];

const LUMEN_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "peak_reduction",
        node_id: "level_detector",
        role: "Lowers the detector threshold and increases compression depth.",
    },
    CircuitControlBinding {
        control_id: "peak_reduction",
        node_id: "opto_memory",
        role: "Shortens attack and raises maximum target reduction.",
    },
    CircuitControlBinding {
        control_id: "gain",
        node_id: "gain_cell",
        role: "Sets makeup gain after gain reduction.",
    },
    CircuitControlBinding {
        control_id: "emphasis",
        node_id: "sidechain_filter",
        role: "Weights the high-passed sidechain against the full-band detector.",
    },
    CircuitControlBinding {
        control_id: "mix",
        node_id: "parallel_mix",
        role: "Sets dry/compressed blend.",
    },
];

const LUMEN_NOTES: &[&str] = &[
    "Lumen is now described by an executable audio-circuit graph; the UI descriptor mirrors that runtime topology.",
    "This is an opto/tube-inspired greybox compressor, not a component-exact hardware clone.",
    "The slot-level input load remains part of the shared chain boundary; the internal blocks start at the loaded input voltage.",
];

pub static LUMEN_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "lumen",
    label: "Lumen compressor",
    kind: CircuitDescriptorKind::Greybox,
    source_of_truth: "executable-rust-audio-circuit",
    implementation: "core/src/pedal.rs::LUMEN_AUDIO_CIRCUIT",
    summary: "Executable opto/tube-inspired compressor graph with loaded input coupling, emphasized sidechain detection, program-dependent gain-reduction memory, gain cell, tube softening, warm filtering, parallel mix, and low-impedance output.",
    nodes: LUMEN_NODES,
    edges: LUMEN_EDGES,
    groups: LUMEN_GROUPS,
    controls: LUMEN_CONTROLS,
    notes: LUMEN_NOTES,
};

const MINOTAUR_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_load", "input_coupling"];
const MINOTAUR_BLEND_GROUP_NODES: &[&str] = &[
    "path_split",
    "clean_path",
    "drive_gain",
    "soft_clip",
    "drive_filter",
];
const MINOTAUR_OUTPUT_GROUP_NODES: &[&str] = &[
    "treble_presence",
    "output_level",
    "output_driver",
    "output_jack",
];

const MINOTAUR_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input and buffer",
        nodes: MINOTAUR_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "blend",
        label: "Clean / drive blend",
        nodes: MINOTAUR_BLEND_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "output",
        label: "Treble and output driver",
        nodes: MINOTAUR_OUTPUT_GROUP_NODES,
    },
];

const MINOTAUR_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Guitar or previous pedal voltage enters the Minotaur boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Voltage plus source impedance enters the pedal.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_load",
        label: "Buffered input load",
        kind: CircuitNodeKind::InputLoad,
        role: "High input impedance and cable/source loading before the internal signal path.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "ConnectionState source/load division against Minotaur::INPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.17, y: 0.44 },
    },
    CircuitNodeDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: CircuitNodeKind::CouplingFilter,
        role: "DC blocking and low-frequency control before the clean and drive paths.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "One-pole high-pass coupling filter.",
        layout: CircuitLayout { x: 0.30, y: 0.44 },
    },
    CircuitNodeDescriptor {
        id: "path_split",
        label: "Clean / drive split",
        kind: CircuitNodeKind::Split,
        role: "Splits the coupled signal into the clean path and the clipping path.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "The same coupled sample feeds both clean and drive branches.",
        layout: CircuitLayout { x: 0.43, y: 0.44 },
    },
    CircuitNodeDescriptor {
        id: "clean_path",
        label: "Clean path",
        kind: CircuitNodeKind::CleanPath,
        role: "Keeps pick attack and dry body available for the gain-dependent blend.",
        control_id: Some("gain"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "Low-passed clean branch scaled down as gain rises.",
        layout: CircuitLayout { x: 0.55, y: 0.26 },
    },
    CircuitNodeDescriptor {
        id: "drive_gain",
        label: "Drive gain",
        kind: CircuitNodeKind::GainStage,
        role: "Gain-controlled overdrive stage feeding the clipping cell.",
        control_id: Some("gain"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "Drive multiplier is derived from the gain control.",
        layout: CircuitLayout { x: 0.55, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "soft_clip",
        label: "Soft clipping pair",
        kind: CircuitNodeKind::ClippingCell,
        role: "Symmetrical clipping approximation for the drive branch.",
        control_id: None,
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/pedal.rs::diode_pair_clip",
        algorithm: "Tanh soft-knee diode-pair approximation.",
        layout: CircuitLayout { x: 0.67, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "drive_filter",
        label: "Drive smoothing",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Post-clipping high-frequency control before the blend returns to the tone path.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "One-pole low-pass after clipping.",
        layout: CircuitLayout { x: 0.78, y: 0.60 },
    },
    CircuitNodeDescriptor {
        id: "treble_presence",
        label: "Treble / presence",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Presence lift and high-band voicing after the clean/drive blend.",
        control_id: Some("treble"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "Blend main path with a high-passed presence branch.",
        layout: CircuitLayout { x: 0.82, y: 0.36 },
    },
    CircuitNodeDescriptor {
        id: "output_level",
        label: "Output level",
        kind: CircuitNodeKind::LevelControl,
        role: "Final pedal level into the output driver.",
        control_id: Some("output"),
        confidence: CircuitConfidence::SchematicInspired,
        implementation: "core/src/pedal.rs::Minotaur",
        algorithm: "Output control scales the voiced voltage.",
        layout: CircuitLayout { x: 0.92, y: 0.36 },
    },
    CircuitNodeDescriptor {
        id: "output_driver",
        label: "Output driver",
        kind: CircuitNodeKind::OutputDriver,
        role: "Low source impedance output boundary for the next device or amp input.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Emit voltage with Minotaur::OUTPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.97, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "output_jack",
        label: "Output jack",
        kind: CircuitNodeKind::Port,
        role: "Pedal output voltage and source impedance leave the Minotaur boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Signal is handed back to the chain electrical boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.50 },
    },
];

const MINOTAUR_EDGES: &[CircuitEdgeDescriptor] = &[
    edge("input_jack", "input_load", CircuitSignalKind::AudioVoltage),
    edge(
        "input_load",
        "input_coupling",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge(
        "input_coupling",
        "path_split",
        CircuitSignalKind::BufferedAudio,
    ),
    edge("path_split", "clean_path", CircuitSignalKind::BufferedAudio),
    edge("path_split", "drive_gain", CircuitSignalKind::BufferedAudio),
    edge("drive_gain", "soft_clip", CircuitSignalKind::DriveAudio),
    edge("soft_clip", "drive_filter", CircuitSignalKind::ClippedAudio),
    edge(
        "clean_path",
        "treble_presence",
        CircuitSignalKind::MixedAudio,
    ),
    edge(
        "drive_filter",
        "treble_presence",
        CircuitSignalKind::MixedAudio,
    ),
    edge(
        "treble_presence",
        "output_level",
        CircuitSignalKind::VoicedAudio,
    ),
    edge(
        "output_level",
        "output_driver",
        CircuitSignalKind::VoicedAudio,
    ),
    edge(
        "output_driver",
        "output_jack",
        CircuitSignalKind::AudioVoltage,
    ),
];

const MINOTAUR_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "gain",
        node_id: "drive_gain",
        role: "Sets drive amount and participates in the clean/drive balance.",
    },
    CircuitControlBinding {
        control_id: "gain",
        node_id: "clean_path",
        role: "Reduces the clean contribution as gain rises.",
    },
    CircuitControlBinding {
        control_id: "treble",
        node_id: "treble_presence",
        role: "Sets the presence lift after the blend.",
    },
    CircuitControlBinding {
        control_id: "output",
        node_id: "output_level",
        role: "Sets final pedal level.",
    },
];

const MINOTAUR_NOTES: &[&str] = &[
    "This is a model-level circuit descriptor, not a component-exact PCB layout.",
    "The current Rust model is a tuned greybox approximation with explicit electrical boundaries.",
    "A future SPICE-backed descriptor can replace inferred nodes where component values are validated.",
];

pub static MINOTAUR_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "minotaur",
    label: "Minotaur overdrive",
    kind: CircuitDescriptorKind::Greybox,
    source_of_truth: "rust-model",
    implementation: "core/src/pedal.rs::Minotaur",
    summary: "Buffered clean-blend overdrive with gain-dependent clipping, treble presence shaping, output level, and explicit input/output impedance boundaries.",
    nodes: MINOTAUR_NODES,
    edges: MINOTAUR_EDGES,
    groups: MINOTAUR_GROUPS,
    controls: MINOTAUR_CONTROLS,
    notes: MINOTAUR_NOTES,
};

const SPRINGFIELD_INPUT_GROUP_NODES: &[&str] = &["input_jack", "input_load", "input_coupling"];
const SPRINGFIELD_TANK_GROUP_NODES: &[&str] = &[
    "dwell_driver",
    "spring_ir_tank",
    "splash_diffusion",
    "recovery_tone",
];
const SPRINGFIELD_OUTPUT_GROUP_NODES: &[&str] = &["wet_dry_mixer", "output_driver", "output_jack"];

const SPRINGFIELD_GROUPS: &[CircuitGroupDescriptor] = &[
    CircuitGroupDescriptor {
        id: "input",
        label: "Input boundary",
        nodes: SPRINGFIELD_INPUT_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "tank",
        label: "Driver, tank, and recovery",
        nodes: SPRINGFIELD_TANK_GROUP_NODES,
    },
    CircuitGroupDescriptor {
        id: "output",
        label: "Mix and output",
        nodes: SPRINGFIELD_OUTPUT_GROUP_NODES,
    },
];

const SPRINGFIELD_NODES: &[CircuitNodeDescriptor] = &[
    CircuitNodeDescriptor {
        id: "input_jack",
        label: "Input jack",
        kind: CircuitNodeKind::Port,
        role: "Amp send, previous FX device, or pedal voltage enters the Springfield boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Voltage plus source impedance enters the pedal.",
        layout: CircuitLayout { x: 0.04, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "input_load",
        label: "Buffered input load",
        kind: CircuitNodeKind::InputLoad,
        role: "High input impedance loading before the wet/dry processing path.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm:
            "ConnectionState source/load division against Springfield::INPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.17, y: 0.44 },
    },
    CircuitNodeDescriptor {
        id: "input_coupling",
        label: "Input coupling",
        kind: CircuitNodeKind::CouplingFilter,
        role: "DC blocking and low-frequency conditioning before tank drive.",
        control_id: None,
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "One-pole high-pass coupling filter.",
        layout: CircuitLayout { x: 0.30, y: 0.44 },
    },
    CircuitNodeDescriptor {
        id: "dwell_driver",
        label: "Dwell driver",
        kind: CircuitNodeKind::TransducerDriver,
        role: "Saturating driver that pushes the virtual spring tank harder as dwell rises.",
        control_id: Some("dwell"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "Dwell-dependent gain and saturation before the tank response.",
        layout: CircuitLayout { x: 0.45, y: 0.42 },
    },
    CircuitNodeDescriptor {
        id: "spring_ir_tank",
        label: "Spring tank IR",
        kind: CircuitNodeKind::ImpulseResponse,
        role: "Embedded SMAC2023 spring response used as the primary tank body.",
        control_id: None,
        confidence: CircuitConfidence::ExternalReference,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "Convolution with embedded fig7a-full-modal-model.wav impulse response.",
        layout: CircuitLayout { x: 0.60, y: 0.42 },
    },
    CircuitNodeDescriptor {
        id: "splash_diffusion",
        label: "Splash diffusion",
        kind: CircuitNodeKind::DiffusionNetwork,
        role: "Short cross-fed delay layer that adds early spring splash around the IR body.",
        control_id: Some("dwell"),
        confidence: CircuitConfidence::Algorithmic,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "Four short delay lines around the IR tank path.",
        layout: CircuitLayout { x: 0.72, y: 0.34 },
    },
    CircuitNodeDescriptor {
        id: "recovery_tone",
        label: "Recovery tone",
        kind: CircuitNodeKind::ToneNetwork,
        role: "Wet-path brightness and recovery filtering after the spring response.",
        control_id: Some("tone"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "Tone-dependent pre-emphasis and recovery low-pass shaping.",
        layout: CircuitLayout { x: 0.80, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "wet_dry_mixer",
        label: "Wet / dry mix",
        kind: CircuitNodeKind::Mixer,
        role: "Blends direct dry voltage with the recovered wet spring path.",
        control_id: Some("mix"),
        confidence: CircuitConfidence::TunedGreybox,
        implementation: "core/src/pedal.rs::Springfield",
        algorithm: "Conservative wet/dry scaling from the mix control.",
        layout: CircuitLayout { x: 0.90, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "output_driver",
        label: "Output driver",
        kind: CircuitNodeKind::OutputDriver,
        role: "Low source impedance output boundary for the next device or amp return.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Emit voltage with Springfield::OUTPUT_IMPEDANCE_OHMS.",
        layout: CircuitLayout { x: 0.97, y: 0.50 },
    },
    CircuitNodeDescriptor {
        id: "output_jack",
        label: "Output jack",
        kind: CircuitNodeKind::Port,
        role: "Pedal output voltage and source impedance leave the Springfield boundary.",
        control_id: None,
        confidence: CircuitConfidence::KnownBoundary,
        implementation: "core/src/pedal.rs::ElectricalSignal",
        algorithm: "Signal is handed back to the chain electrical boundary.",
        layout: CircuitLayout { x: 0.99, y: 0.50 },
    },
];

const SPRINGFIELD_EDGES: &[CircuitEdgeDescriptor] = &[
    edge("input_jack", "input_load", CircuitSignalKind::AudioVoltage),
    edge(
        "input_load",
        "input_coupling",
        CircuitSignalKind::LoadedAudioVoltage,
    ),
    edge(
        "input_coupling",
        "dwell_driver",
        CircuitSignalKind::BufferedAudio,
    ),
    edge(
        "input_coupling",
        "wet_dry_mixer",
        CircuitSignalKind::BufferedAudio,
    ),
    edge(
        "dwell_driver",
        "spring_ir_tank",
        CircuitSignalKind::DriveAudio,
    ),
    edge(
        "spring_ir_tank",
        "splash_diffusion",
        CircuitSignalKind::WetAudio,
    ),
    edge(
        "splash_diffusion",
        "recovery_tone",
        CircuitSignalKind::WetAudio,
    ),
    edge(
        "recovery_tone",
        "wet_dry_mixer",
        CircuitSignalKind::WetAudio,
    ),
    edge(
        "wet_dry_mixer",
        "output_driver",
        CircuitSignalKind::MixedAudio,
    ),
    edge(
        "output_driver",
        "output_jack",
        CircuitSignalKind::AudioVoltage,
    ),
];

const SPRINGFIELD_CONTROLS: &[CircuitControlBinding] = &[
    CircuitControlBinding {
        control_id: "dwell",
        node_id: "dwell_driver",
        role: "Sets tank drive and saturation.",
    },
    CircuitControlBinding {
        control_id: "tone",
        node_id: "recovery_tone",
        role: "Sets wet-path brightness and splash emphasis.",
    },
    CircuitControlBinding {
        control_id: "mix",
        node_id: "wet_dry_mixer",
        role: "Sets wet/dry blend.",
    },
];

const SPRINGFIELD_NOTES: &[&str] = &[
    "This descriptor is intentionally labelled as a greybox/IR hybrid.",
    "The spring body comes from an embedded modelled spring impulse response, not a committed hardware tank schematic.",
    "Driver, diffusion, recovery tone, and mix are Rust model stages that should be rendered as semantic blocks rather than fake PCB traces.",
];

pub static SPRINGFIELD_CIRCUIT: CircuitDescriptor = CircuitDescriptor {
    schema: CIRCUIT_DESCRIPTOR_SCHEMA,
    model_id: "springfield",
    label: "Springfield spring reverb",
    kind: CircuitDescriptorKind::GreyboxIrHybrid,
    source_of_truth: "rust-model-plus-local-ir-reference",
    implementation: "core/src/pedal.rs::Springfield",
    summary: "Spring-inspired reverb with buffered input, dwell-driven saturated tank feed, embedded spring IR body, splash diffusion, recovery tone, wet/dry mix, and explicit output impedance boundary.",
    nodes: SPRINGFIELD_NODES,
    edges: SPRINGFIELD_EDGES,
    groups: SPRINGFIELD_GROUPS,
    controls: SPRINGFIELD_CONTROLS,
    notes: SPRINGFIELD_NOTES,
};

const fn edge(
    from: &'static str,
    to: &'static str,
    signal: CircuitSignalKind,
) -> CircuitEdgeDescriptor {
    CircuitEdgeDescriptor { from, to, signal }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_descriptors_for_current_greybox_pedals() {
        assert_eq!(
            device_circuit_descriptor(DeviceConfig::Lumen).map(|d| d.model_id),
            Some("lumen")
        );
        assert_eq!(
            device_circuit_descriptor(DeviceConfig::Minotaur).map(|d| d.model_id),
            Some("minotaur")
        );
        assert_eq!(
            device_circuit_descriptor(DeviceConfig::Springfield).map(|d| d.model_id),
            Some("springfield")
        );
        assert!(device_circuit_descriptor(DeviceConfig::StudioVerb).is_none());
    }

    #[test]
    fn exposes_descriptor_for_nox30_and_legacy_experimental_alias() {
        assert_eq!(
            amp_circuit_descriptor("nox30").map(|d| d.model_id),
            Some("nox30")
        );
        assert_eq!(
            amp_circuit_descriptor("nox30-experimental").map(|d| d.model_id),
            Some("nox30")
        );
        assert_eq!(
            amp_circuit_descriptor("nox30-experimental?tone_source_ohms=47000").map(|d| d.model_id),
            Some("nox30")
        );
        assert_eq!(
            amp_circuit_descriptor("none-star").map(|d| d.model_id),
            Some("none-star")
        );
        assert_eq!(
            amp_circuit_descriptor("lonestar-special").map(|d| d.model_id),
            Some("none-star")
        );
        assert_eq!(
            amp_circuit_descriptor("lone-star-special").map(|d| d.model_id),
            Some("none-star")
        );
        assert_eq!(
            amp_circuit_descriptor("boxer-seven-lead").map(|d| d.model_id),
            Some("boxer-seven-lead")
        );
        assert_eq!(
            amp_circuit_descriptor("shiva-20th-lead").map(|d| d.model_id),
            Some("boxer-seven-lead")
        );
        assert!(amp_circuit_descriptor("unknown-amp").is_none());
    }

    #[test]
    fn minotaur_descriptor_binds_real_controls_to_nodes() {
        let descriptor = &MINOTAUR_CIRCUIT;
        for control_id in ["gain", "treble", "output"] {
            assert!(
                descriptor
                    .controls
                    .iter()
                    .any(|binding| binding.control_id == control_id),
                "missing control binding for {control_id}"
            );
        }
        assert!(descriptor
            .nodes
            .iter()
            .any(|node| node.kind == CircuitNodeKind::ClippingCell));
    }

    #[test]
    fn lumen_descriptor_mirrors_executable_audio_circuit() {
        let descriptor = &LUMEN_CIRCUIT;
        assert_eq!(descriptor.source_of_truth, "executable-rust-audio-circuit");

        for block in crate::pedal::LUMEN_AUDIO_CIRCUIT.blocks {
            assert!(
                descriptor.nodes.iter().any(|node| node.id == block.id),
                "missing Lumen descriptor node for executable block {}",
                block.id
            );
        }

        for control_id in ["peak_reduction", "gain", "emphasis", "mix"] {
            assert!(
                descriptor
                    .controls
                    .iter()
                    .any(|binding| binding.control_id == control_id),
                "missing Lumen control binding for {control_id}"
            );
        }
    }

    #[test]
    fn springfield_descriptor_is_marked_as_ir_hybrid() {
        assert_eq!(
            SPRINGFIELD_CIRCUIT.kind,
            CircuitDescriptorKind::GreyboxIrHybrid
        );
        assert!(SPRINGFIELD_CIRCUIT.nodes.iter().any(|node| {
            node.kind == CircuitNodeKind::ImpulseResponse
                && node.confidence == CircuitConfidence::ExternalReference
        }));
    }

    #[test]
    fn nox30_descriptor_marks_supply_as_shared_rail_state() {
        assert_eq!(NOX30_CIRCUIT.kind, CircuitDescriptorKind::CircuitInformed);
        assert!(NOX30_CIRCUIT.nodes.iter().any(|node| {
            node.id == "supply_network" && node.kind == CircuitNodeKind::SupplyNetwork
        }));
        assert!(NOX30_CIRCUIT.edges.iter().any(|edge| {
            edge.from == "supply_network" && edge.signal == CircuitSignalKind::RailVoltage
        }));
    }

    #[test]
    fn none_star_descriptor_marks_clean_edge_scope() {
        assert_eq!(NONE_STAR_CIRCUIT.kind, CircuitDescriptorKind::Greybox);
        assert!(NONE_STAR_CIRCUIT
            .notes
            .iter()
            .any(|note| note.contains("Clean/Edge")));
        assert!(NONE_STAR_CIRCUIT
            .nodes
            .iter()
            .any(|node| node.id == "power_6l6" && node.kind == CircuitNodeKind::PowerStage));
    }

    #[test]
    fn boxer_seven_descriptor_marks_lead_scope() {
        assert_eq!(
            BOXER_SEVEN_LEAD_CIRCUIT.kind,
            CircuitDescriptorKind::Greybox
        );
        assert!(BOXER_SEVEN_LEAD_CIRCUIT
            .notes
            .iter()
            .any(|note| note.contains("lead channel")));
        assert!(BOXER_SEVEN_LEAD_CIRCUIT
            .nodes
            .iter()
            .any(|node| node.id == "kt88_power" && node.kind == CircuitNodeKind::PowerStage));
    }

    #[test]
    fn descriptor_edges_reference_existing_nodes() {
        for descriptor in [
            &LUMEN_CIRCUIT,
            &MINOTAUR_CIRCUIT,
            &SPRINGFIELD_CIRCUIT,
            &NOX30_CIRCUIT,
            &NONE_STAR_CIRCUIT,
            &BOXER_SEVEN_LEAD_CIRCUIT,
        ] {
            for edge in descriptor.edges {
                assert!(
                    descriptor.nodes.iter().any(|node| node.id == edge.from),
                    "{} has edge from unknown node {}",
                    descriptor.model_id,
                    edge.from
                );
                assert!(
                    descriptor.nodes.iter().any(|node| node.id == edge.to),
                    "{} has edge to unknown node {}",
                    descriptor.model_id,
                    edge.to
                );
            }
        }
    }
}
