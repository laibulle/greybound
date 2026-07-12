use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

mod a2;

pub const DEFAULT_NAM_SAMPLE_RATE_HZ: f32 = 48_000.0;

pub use a2::NamA2Processor;

#[derive(Clone, Debug)]
pub struct NamModel {
    pub version: String,
    pub architecture: String,
    pub config: Value,
    pub weights: Vec<f32>,
    pub sample_rate_hz: f32,
    pub metadata: NamMetadata,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NamMetadata {
    pub name: Option<String>,
    pub modeled_by: Option<String>,
    pub gear_make: Option<String>,
    pub gear_model: Option<String>,
    pub gear_type: Option<String>,
    pub tone_type: Option<String>,
    pub input_level_dbu: Option<f32>,
    pub output_level_dbu: Option<f32>,
    pub raw: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamArchitectureFamily {
    Architecture2,
    Lstm,
    WaveNet,
    Unknown,
}

impl NamModel {
    pub fn from_str(text: &str) -> Result<Self> {
        let raw: RawNamModel = serde_json::from_str(text).context("failed to parse .nam JSON")?;
        let raw = normalize_raw_model(raw)?;
        if raw.version.trim().is_empty() {
            anyhow::bail!(".nam version is empty");
        }
        if raw.architecture.trim().is_empty() {
            anyhow::bail!(".nam architecture is empty");
        }
        if raw.weights.is_empty() {
            anyhow::bail!(".nam weights are empty");
        }

        Ok(Self {
            version: raw.version,
            architecture: raw.architecture,
            config: raw.config,
            weights: raw.weights,
            sample_rate_hz: raw
                .sample_rate
                .map(|sample_rate| sample_rate as f32)
                .unwrap_or(DEFAULT_NAM_SAMPLE_RATE_HZ),
            metadata: NamMetadata::from_value(raw.metadata.unwrap_or(Value::Null)),
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read .nam file {}", path.display()))?;
        Self::from_str(&text)
            .with_context(|| format!("failed to load .nam file {}", path.display()))
    }

    pub fn architecture_family(&self) -> NamArchitectureFamily {
        match self.architecture.as_str() {
            "LSTM" | "Lstm" => NamArchitectureFamily::Lstm,
            "WaveNet" | "WaveNetModel" => {
                if NamA2Processor::is_supported(self) {
                    NamArchitectureFamily::Architecture2
                } else {
                    NamArchitectureFamily::WaveNet
                }
            }
            _ => NamArchitectureFamily::Unknown,
        }
    }

    pub fn display_name(&self) -> &str {
        self.metadata
            .name
            .as_deref()
            .or(self.metadata.gear_model.as_deref())
            .unwrap_or("NAM model")
    }
}

impl NamMetadata {
    fn from_value(value: Value) -> Self {
        Self {
            name: string_field(&value, "name"),
            modeled_by: string_field(&value, "modeled_by"),
            gear_make: string_field(&value, "gear_make"),
            gear_model: string_field(&value, "gear_model"),
            gear_type: string_field(&value, "gear_type"),
            tone_type: string_field(&value, "tone_type"),
            input_level_dbu: f32_field(&value, "input_level_dbu"),
            output_level_dbu: f32_field(&value, "output_level_dbu"),
            raw: value,
        }
    }
}

#[derive(Deserialize)]
struct RawNamModel {
    version: String,
    architecture: String,
    #[serde(default)]
    config: Value,
    weights: Vec<f32>,
    sample_rate: Option<f64>,
    metadata: Option<Value>,
}

fn normalize_raw_model(raw: RawNamModel) -> Result<RawNamModel> {
    if raw.architecture != "SlimmableContainer" {
        return Ok(raw);
    }

    let submodels = raw
        .config
        .get("submodels")
        .and_then(Value::as_array)
        .filter(|submodels| !submodels.is_empty())
        .context("SlimmableContainer has no submodels")?;

    let selected = submodels
        .iter()
        .max_by(|left, right| {
            let left_max = left
                .get("max_value")
                .and_then(Value::as_f64)
                .unwrap_or(f64::NEG_INFINITY);
            let right_max = right
                .get("max_value")
                .and_then(Value::as_f64)
                .unwrap_or(f64::NEG_INFINITY);
            left_max.total_cmp(&right_max)
        })
        .expect("non-empty submodels already checked");

    let mut selected_model: RawNamModel = serde_json::from_value(
        selected
            .get("model")
            .cloned()
            .context("submodel has no model")?,
    )
    .context("failed to parse SlimmableContainer selected submodel")?;
    if selected_model.sample_rate.is_none() {
        selected_model.sample_rate = raw.sample_rate;
    }
    if selected_model.metadata.is_none() {
        selected_model.metadata = raw.metadata;
    }

    Ok(selected_model)
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn f32_field(value: &Value, key: &str) -> Option<f32> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_architecture_2_nam_metadata() {
        let weights = vec![0.0; a2_test_weight_count(8)];
        let json = serde_json::json!({
            "version": "0.7.0",
            "architecture": "SlimmableContainer",
            "config": {
                "submodels": [
                    {
                        "max_value": 1.0,
                        "model": {
                            "version": "0.7.0",
                            "architecture": "WaveNet",
                            "config": a2_test_config(8),
                            "weights": weights,
                            "sample_rate": 48000.0,
                            "metadata": {
                                "name": "TopBoost-Gain5",
                                "modeled_by": "bjeffhind",
                                "gear_model": "AC30HWH",
                                "gear_type": "amp",
                                "tone_type": "crunch",
                                "input_level_dbu": 12.0,
                                "output_level_dbu": 6.0
                            }
                        }
                    }
                ]
            },
            "weights": [],
            "sample_rate": 48000.0
        });
        let model = NamModel::from_str(&serde_json::to_string(&json).unwrap()).unwrap();

        assert_eq!(model.version, "0.7.0");
        assert_eq!(model.architecture, "WaveNet");
        assert_eq!(
            model.architecture_family(),
            NamArchitectureFamily::Architecture2
        );
        assert_eq!(model.display_name(), "TopBoost-Gain5");
        assert_eq!(model.sample_rate_hz, 48_000.0);
        assert_eq!(model.weights.len(), a2_test_weight_count(8));
        assert_eq!(model.metadata.gear_type.as_deref(), Some("amp"));
    }

    #[test]
    fn defaults_missing_sample_rate_to_48k() {
        let model = NamModel::from_str(
            r#"{
                "version": "0.5.4",
                "architecture": "WaveNet",
                "config": {},
                "weights": [0.1]
            }"#,
        )
        .unwrap();

        assert_eq!(model.sample_rate_hz, DEFAULT_NAM_SAMPLE_RATE_HZ);
        assert_eq!(model.architecture_family(), NamArchitectureFamily::WaveNet);
    }

    #[test]
    fn rejects_empty_weights() {
        let error = NamModel::from_str(
            r#"{
                "version": "0.7.0",
                "architecture": "WaveNet",
                "config": {},
                "weights": []
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("weights"));
    }

    fn a2_test_weight_count(channels: usize) -> usize {
        const KERNEL_SIZES: [usize; 23] = [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 15, 15, 6, 6, 6, 6, 6, 6, 6,
        ];
        channels
            + KERNEL_SIZES
                .iter()
                .map(|kernel_size| {
                    kernel_size * channels * channels
                        + channels
                        + channels
                        + channels * channels
                        + channels
                })
                .sum::<usize>()
            + 16 * channels
            + 1
            + 1
    }

    fn a2_test_config(channels: usize) -> Value {
        serde_json::json!({
            "in_channels": 1,
            "layers": [
                {
                    "input_size": 1,
                    "condition_size": 1,
                    "channels": channels,
                    "bottleneck": channels,
                    "kernel_sizes": [6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 15, 15, 6, 6, 6, 6, 6, 6, 6],
                    "dilations": [1, 3, 7, 17, 41, 101, 239, 1, 3, 7, 17, 41, 101, 239, 1, 13, 1, 3, 7, 17, 41, 101, 239],
                    "activation": vec![serde_json::json!({"type": "LeakyReLU", "negative_slope": 0.01}); 23],
                    "gating_mode": vec!["none"; 23],
                    "secondary_activation": vec![Value::Null; 23],
                    "head": {"out_channels": 1, "kernel_size": 16, "bias": true},
                    "layer1x1": {"active": true, "groups": 1},
                    "head1x1": {"active": false, "out_channels": 1, "groups": 1},
                    "groups_input": 1,
                    "groups_input_mixin": 1
                }
            ],
            "head": null,
            "head_scale": 0.01
        })
    }
}
