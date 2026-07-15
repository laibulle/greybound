use super::{ElectricalSignal, MinotaurControls};
use crate::nam::{NamA2Processor, NamModel};
use std::path::Path;

/// A pedal-position NAM capture with pre- and post-model trims.
pub struct NamPedal {
    processor: Option<NamA2Processor>,
}

impl NamPedal {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(path: Option<&Path>) -> Self {
        let processor = path
            .and_then(|path| NamModel::from_path(path).ok())
            .and_then(|model| NamA2Processor::from_model(&model).ok());
        Self { processor }
    }

    pub fn reset(&mut self) {
        if let Some(processor) = &mut self.processor {
            processor.reset();
        }
    }

    pub fn process_loaded_voltage(
        &mut self,
        input_voltage: f32,
        controls: MinotaurControls,
    ) -> ElectricalSignal {
        let input = input_voltage * trim_gain(controls.gain);
        let output = self
            .processor
            .as_mut()
            .map(|processor| processor.process(input))
            .unwrap_or(input)
            * trim_gain(controls.output);
        ElectricalSignal::new(output.clamp(-4.0, 4.0), Self::OUTPUT_IMPEDANCE_OHMS)
    }
}

fn trim_gain(value: f32) -> f32 {
    10.0_f32.powf((value.clamp(0.0, 1.0) - 0.5) * 36.0 / 20.0)
}
