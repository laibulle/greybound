use super::controls::SharedRuntimeControls;

pub(super) struct DoublerProcessor {
    sample_rate: f32,
    buffer: Vec<f32>,
    write_index: usize,
    modulation_phase: f32,
}

impl DoublerProcessor {
    pub(super) fn new(sample_rate: f32) -> Self {
        let capacity = (sample_rate * 0.030).ceil() as usize + 4;
        Self {
            sample_rate,
            buffer: vec![0.0; capacity.max(8)],
            write_index: 0,
            modulation_phase: 0.0,
        }
    }

    pub(super) fn process(&mut self, input: f32, controls: &SharedRuntimeControls) -> (f32, f32) {
        let delayed = self.read_delayed_sample(controls.doubler_delay_ms());
        self.buffer[self.write_index] = input;
        self.write_index = (self.write_index + 1) % self.buffer.len();
        self.advance_modulation();

        if !controls.doubler_enabled() {
            return (input, input);
        }

        let left = input * 0.92 + delayed * 0.08;
        let right = delayed * 0.92 + input * 0.08;
        (left, right)
    }

    fn read_delayed_sample(&self, delay_ms: f32) -> f32 {
        let modulation_depth_ms = (delay_ms * 0.04).min(0.35);
        let modulated_delay_ms = delay_ms + self.modulation_phase.sin() * modulation_depth_ms;
        let delay_samples = (modulated_delay_ms.max(0.0) * self.sample_rate / 1_000.0)
            .min((self.buffer.len() - 2) as f32);
        let read_position = self.write_index as f32 - delay_samples + self.buffer.len() as f32;
        let base_index = read_position.floor() as usize % self.buffer.len();
        let next_index = (base_index + 1) % self.buffer.len();
        let fraction = read_position.fract();
        self.buffer[base_index] * (1.0 - fraction) + self.buffer[next_index] * fraction
    }

    fn advance_modulation(&mut self) {
        let increment = std::f32::consts::TAU * 0.19 / self.sample_rate;
        self.modulation_phase = (self.modulation_phase + increment) % std::f32::consts::TAU;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::controls::SharedRuntimeControls;
    use greybound_ui::GreyboundUi;

    #[test]
    fn disabled_preserves_dual_mono() {
        let ui = GreyboundUi::default();
        let controls = SharedRuntimeControls::new(&ui);
        let mut doubler = DoublerProcessor::new(48_000.0);

        for index in 0..256 {
            let sample = (index as f32 * 0.01).sin() * 0.2;
            let (left, right) = doubler.process(sample, &controls);
            assert!((left - sample).abs() < 1.0e-7);
            assert!((right - sample).abs() < 1.0e-7);
        }
    }

    #[test]
    fn enabled_decorrelates_left_and_right() {
        let mut ui = GreyboundUi::default();
        ui.doubler.enabled = true;
        ui.doubler.delay_ms = 7.15;
        let controls = SharedRuntimeControls::new(&ui);
        let mut doubler = DoublerProcessor::new(48_000.0);

        let (left, right) = doubler.process(0.5, &controls);

        assert!(left > right, "left={left}, right={right}");
        assert!((left - right).abs() > 0.1, "left={left}, right={right}");
    }
}
