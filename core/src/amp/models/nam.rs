use super::AmpModel;
use crate::amp::AmpControls;
use crate::nam::{NamA2Processor, NamModel};
use std::path::Path;

#[derive(Debug)]
pub(in crate::amp) struct NamAmp {
    processor: Option<NamA2Processor>,
}

impl NamAmp {
    pub(super) fn new(model_spec: &str) -> Self {
        let model =
            nam_path_from_model_spec(model_spec).and_then(|path| {
                match NamModel::from_path(Path::new(path)) {
                    Ok(model) => Some(model),
                    Err(error) => {
                        #[cfg(debug_assertions)]
                        eprintln!("Greybound NAM loader disabled: {error:#}");
                        #[cfg(not(debug_assertions))]
                        let _ = error;
                        None
                    }
                }
            });
        let processor = model
            .as_ref()
            .and_then(|model| match NamA2Processor::from_model(model) {
                Ok(processor) => Some(processor),
                Err(error) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "Greybound NAM loader '{}' bypassed: {error:#}",
                        model.display_name()
                    );
                    #[cfg(not(debug_assertions))]
                    let _ = error;
                    None
                }
            });

        Self { processor }
    }

    #[cfg(test)]
    fn has_processor(&self) -> bool {
        self.processor.is_some()
    }
}

impl AmpModel for NamAmp {
    fn reset(&mut self) {
        if let Some(processor) = &mut self.processor {
            processor.reset();
        }
    }

    fn process(&mut self, input: f32, _controls: AmpControls) -> f32 {
        if let Some(processor) = &mut self.processor {
            processor.process(input).clamp(-4.0, 4.0)
        } else {
            input
        }
    }
}

fn nam_path_from_model_spec(model_spec: &str) -> Option<&str> {
    let (_, query) = model_spec.split_once('?')?;
    query
        .split('&')
        .find_map(|part| part.strip_prefix("path="))
        .filter(|path| !path.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_from_model_spec() {
        assert_eq!(
            nam_path_from_model_spec("nam2?path=/tmp/TopBoost-Gain5.nam"),
            Some("/tmp/TopBoost-Gain5.nam")
        );
    }

    #[test]
    fn missing_path_keeps_loader_as_bypass() {
        let mut amp = NamAmp::new("nam2");

        assert!(!amp.has_processor());
        assert_eq!(amp.process(0.25, test_controls()), 0.25);
    }

    fn test_controls() -> AmpControls {
        AmpControls {
            volume: 0.5,
            bass: 0.5,
            treble: 0.5,
            cut: 0.5,
            output: 0.5,
            drive: 0.0,
            presence: 0.5,
            sag: 0.5,
        }
    }
}
