mod boxer_seven;
mod daybreaker_50;
mod nam;
mod none_star;
mod nox30;

use super::{AmpControls, NeuralCellMode, Nox30OperatingPoint, StageBoundaryState};
use boxer_seven::BoxerSevenLead;
use daybreaker_50::Daybreaker50;
use nam::NamAmp;
use none_star::NoneStar;
use nox30::Nox30Current;
use std::path::PathBuf;

pub(super) fn configure_nox30_first_stage_neural(
    descriptor_path: Option<PathBuf>,
    mode: NeuralCellMode,
) {
    nox30::configure_first_stage_neural(descriptor_path, mode);
}

pub(super) fn configure_nox30_first_stage_graybox(
    config_path: Option<PathBuf>,
    mode: NeuralCellMode,
) {
    nox30::configure_first_stage_graybox(config_path, mode);
}

pub(super) fn configure_none_star_power_6l6_neural(
    descriptor_path: Option<PathBuf>,
    mode: NeuralCellMode,
) {
    none_star::configure_power_6l6_neural(descriptor_path, mode);
}

pub(in crate::amp) enum AmpCore {
    BoxerSevenLead(BoxerSevenLead),
    Daybreaker50(Daybreaker50),
    Nam(NamAmp),
    NoneStar(NoneStar),
    Nox30(Nox30Current),
}

impl AmpCore {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self::Nox30(Nox30Current::new_with_model(sample_rate, "nox30"))
    }

    pub(super) fn new_with_model(sample_rate: f32, model: &str) -> Self {
        let model_base = model.split_once('?').map_or(model, |(base, _)| base);
        match model_base {
            "boxer-seven-lead" | "boxer-seven" | "shiva-20th-lead" | "shiva20-lead" => {
                Self::BoxerSevenLead(BoxerSevenLead::new(sample_rate))
            }
            "daybreaker-50" | "daybreaker" => Self::Daybreaker50(Daybreaker50::new(sample_rate)),
            "none-star" | "lonestar-special" | "lone-star-special" | "lonestar" => {
                Self::NoneStar(NoneStar::new(sample_rate))
            }
            "nam2" | "nam-loader" => Self::Nam(NamAmp::new(model)),
            "nox30" | "nox30-experimental" => {
                Self::Nox30(Nox30Current::new_with_model(sample_rate, model))
            }
            _ => Self::new(sample_rate),
        }
    }

    pub(super) fn reset(&mut self) {
        match self {
            Self::BoxerSevenLead(model) => model.reset(),
            Self::Daybreaker50(model) => model.reset(),
            Self::Nam(model) => model.reset(),
            Self::NoneStar(model) => model.reset(),
            Self::Nox30(model) => model.reset(),
        }
    }

    pub(super) fn nox30_operating_point(&self) -> Option<Nox30OperatingPoint> {
        match self {
            Self::Nox30(model) => Some(model.operating_point()),
            Self::BoxerSevenLead(_) | Self::Daybreaker50(_) | Self::Nam(_) | Self::NoneStar(_) => {
                None
            }
        }
    }

    pub(super) fn nox30_boundary_states(&self) -> Option<[StageBoundaryState; 11]> {
        match self {
            Self::Nox30(model) => Some(model.boundary_states()),
            Self::BoxerSevenLead(_) | Self::Daybreaker50(_) | Self::Nam(_) | Self::NoneStar(_) => {
                None
            }
        }
    }

    #[inline]
    pub(super) fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        match self {
            Self::BoxerSevenLead(model) => model.process(input, controls),
            Self::Daybreaker50(model) => model.process(input, controls),
            Self::Nam(model) => model.process(input, controls),
            Self::NoneStar(model) => model.process(input, controls),
            Self::Nox30(model) => model.process(input, controls),
        }
    }

    #[inline]
    pub(super) fn process_with_fx_loop(
        &mut self,
        input: f32,
        controls: AmpControls,
        mut process_fx: impl FnMut(f32) -> f32,
    ) -> f32 {
        match self {
            Self::Nox30(model) => {
                let preamp = model.process_preamp(input, controls);
                let return_voltage = process_fx(preamp.send_voltage);
                model.process_power_amp(return_voltage, preamp, controls)
            }
            _ => process_fx(self.process(input, controls)),
        }
    }
}

pub(super) trait AmpModel {
    fn reset(&mut self);
    fn process(&mut self, input: f32, controls: AmpControls) -> f32;
}
