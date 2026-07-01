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
        DeviceConfig::Minotaur => Some(&MINOTAUR_CIRCUIT),
        DeviceConfig::Springfield => Some(&SPRINGFIELD_CIRCUIT),
        _ => None,
    }
}

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
    fn descriptor_edges_reference_existing_nodes() {
        for descriptor in [&MINOTAUR_CIRCUIT, &SPRINGFIELD_CIRCUIT] {
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
