use anyhow::{bail, Context, Result};
use greybound::amp::{
    configure_none_star_power_6l6_neural, configure_nox30_first_stage_graybox,
    configure_nox30_first_stage_neural, NeuralCellMode,
};
use greybound::{configure_minotaur_clip_neural, configure_minotaur_tone_neural};
use std::path::PathBuf;

pub struct NeuralCellOverride {
    pub stage: String,
    pub descriptor_path: PathBuf,
}

pub struct GrayboxCellOverride {
    pub stage: String,
    pub config_path: PathBuf,
}

pub fn parse_neural_cell_override(value: &str) -> Result<NeuralCellOverride> {
    let (stage, descriptor) = value
        .split_once('=')
        .context("--neural-cell expects STAGE=DESCRIPTOR, for example nox30.first_stage=lab/models/common-cathode-12ax7-mlp-v1/model.greybound.json or minotaur.tone=lab/models/klon-tone-mlp-current/model.greybound.json")?;
    if stage.trim().is_empty() || descriptor.trim().is_empty() {
        bail!("--neural-cell expects non-empty STAGE=DESCRIPTOR");
    }
    Ok(NeuralCellOverride {
        stage: stage.trim().to_owned(),
        descriptor_path: PathBuf::from(descriptor.trim()),
    })
}

pub fn parse_graybox_cell_override(value: &str) -> Result<GrayboxCellOverride> {
    let (stage, config) = value
        .split_once('=')
        .context("--graybox-cell expects STAGE=CONFIG, for example nox30.first_stage=accepted-live, nox30.first_stage=accepted, or nox30.first_stage=lab/models/common-cathode-12ax7-graybox-state-v0/common-cathode-graybox-state.json")?;
    if stage.trim().is_empty() || config.trim().is_empty() {
        bail!("--graybox-cell expects non-empty STAGE=CONFIG");
    }
    Ok(GrayboxCellOverride {
        stage: stage.trim().to_owned(),
        config_path: PathBuf::from(config.trim()),
    })
}

pub fn parse_neural_cell_mode(value: &str) -> Result<NeuralCellMode> {
    match value {
        "shadow" => Ok(NeuralCellMode::Shadow),
        "replace" => Ok(NeuralCellMode::Replace),
        _ => bail!("--neural-cell-mode must be 'shadow' or 'replace'"),
    }
}

pub fn neural_cell_mode_name(mode: NeuralCellMode) -> &'static str {
    match mode {
        NeuralCellMode::Shadow => "shadow",
        NeuralCellMode::Replace => "replace",
    }
}

pub fn apply_neural_overrides(
    overrides: &[NeuralCellOverride],
    graybox_overrides: &[GrayboxCellOverride],
    mode: NeuralCellMode,
    disable_neural_cell: bool,
) -> Result<()> {
    if disable_neural_cell {
        configure_nox30_first_stage_neural(None, NeuralCellMode::Shadow);
        configure_none_star_power_6l6_neural(None, NeuralCellMode::Shadow);
        configure_minotaur_clip_neural(None, NeuralCellMode::Shadow);
        configure_minotaur_tone_neural(None, NeuralCellMode::Shadow);
        return Ok(());
    }
    if !overrides.is_empty() && !graybox_overrides.is_empty() {
        bail!("--neural-cell and --graybox-cell are mutually exclusive for the same run");
    }
    if overrides.is_empty() && graybox_overrides.is_empty() {
        return Ok(());
    }
    for override_ in overrides {
        match override_.stage.as_str() {
            "nox30.first_stage" => {
                configure_nox30_first_stage_neural(Some(override_.descriptor_path.clone()), mode);
            }
            "none-star.power_6l6" | "nonestar.power_6l6" => {
                configure_none_star_power_6l6_neural(Some(override_.descriptor_path.clone()), mode);
            }
            "minotaur.clip" => {
                configure_minotaur_clip_neural(Some(override_.descriptor_path.clone()), mode);
            }
            "minotaur.tone" => {
                configure_minotaur_tone_neural(Some(override_.descriptor_path.clone()), mode);
            }
            other => bail!(
                "unsupported --neural-cell stage '{}'; supported: nox30.first_stage, none-star.power_6l6, minotaur.clip, minotaur.tone",
                other
            ),
        }
    }
    for override_ in graybox_overrides {
        match override_.stage.as_str() {
            "nox30.first_stage" => {
                configure_nox30_first_stage_graybox(Some(override_.config_path.clone()), mode);
            }
            other => bail!(
                "unsupported --graybox-cell stage '{}'; supported: nox30.first_stage",
                other
            ),
        }
    }
    Ok(())
}
