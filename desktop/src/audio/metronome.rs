use super::controls::SharedRuntimeControls;

pub(super) struct MetronomeGenerator {
    sample_rate: f32,
    samples_until_tick: f32,
    envelope: f32,
    phase: f32,
    frequency: f32,
    beat_index: u32,
    random_state: u32,
    was_enabled: bool,
}

impl MetronomeGenerator {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            samples_until_tick: 0.0,
            envelope: 0.0,
            phase: 0.0,
            frequency: 1_700.0,
            beat_index: 0,
            random_state: 0x6D2B_79F5,
            was_enabled: false,
        }
    }

    pub(super) fn process(&mut self, controls: &SharedRuntimeControls) -> (f32, f32) {
        let enabled = controls.metronome_enabled();
        if !enabled {
            self.was_enabled = false;
            self.samples_until_tick = 0.0;
            self.envelope = 0.0;
            self.phase = 0.0;
            self.beat_index = 0;
            return (0.0, 0.0);
        }

        if !self.was_enabled || self.samples_until_tick <= 0.0 {
            if self.should_mute(controls.metronome_mute_probability()) {
                self.skip_tick(controls.metronome_beats_per_bar());
            } else {
                self.trigger(controls.metronome_beats_per_bar());
            }
            self.samples_until_tick += self.samples_per_tick(controls);
        }
        self.was_enabled = true;
        self.samples_until_tick -= 1.0;

        if self.envelope <= 0.000_1 {
            return (0.0, 0.0);
        }

        let phase_increment = std::f32::consts::TAU * self.frequency / self.sample_rate;
        self.phase = (self.phase + phase_increment) % std::f32::consts::TAU;
        let transient = self.phase.sin().signum() * 0.35 + self.phase.sin() * 0.65;
        let sample = transient * self.envelope * controls.metronome_volume() * 0.20;
        let decay = (-1.0 / (self.sample_rate * 0.005)).exp();
        self.envelope *= decay;

        let pan = controls.metronome_pan();
        let left_gain = (pan * std::f32::consts::FRAC_PI_2).cos();
        let right_gain = (pan * std::f32::consts::FRAC_PI_2).sin();
        (
            (sample * left_gain).clamp(-0.22, 0.22),
            (sample * right_gain).clamp(-0.22, 0.22),
        )
    }

    fn trigger(&mut self, beats_per_bar: u32) {
        let accent = self.beat_index == 0;
        self.frequency = if accent { 1_700.0 } else { 1_100.0 };
        self.envelope = if accent { 1.0 } else { 0.78 };
        self.phase = 0.0;
        self.advance_beat(beats_per_bar);
    }

    fn skip_tick(&mut self, beats_per_bar: u32) {
        self.envelope = 0.0;
        self.phase = 0.0;
        self.advance_beat(beats_per_bar);
    }

    fn advance_beat(&mut self, beats_per_bar: u32) {
        self.beat_index = (self.beat_index + 1) % beats_per_bar.max(1);
    }

    fn should_mute(&mut self, probability: f32) -> bool {
        let probability = probability.clamp(0.0, 1.0);
        if probability <= 0.0 {
            return false;
        }
        if probability >= 1.0 {
            return true;
        }

        self.random_state ^= self.random_state << 13;
        self.random_state ^= self.random_state >> 17;
        self.random_state ^= self.random_state << 5;
        let sample = self.random_state as f32 / u32::MAX as f32;
        sample < probability
    }

    fn samples_per_tick(&self, controls: &SharedRuntimeControls) -> f32 {
        let beats_per_second = controls.metronome_bpm() / 60.0;
        let ticks_per_beat = controls.metronome_rhythm_division() as f32;
        (self.sample_rate / (beats_per_second * ticks_per_beat)).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use greybound_ui::GreyboundUi;

    #[test]
    fn zero_mute_probability_keeps_the_first_tick_audible() {
        let mut ui = GreyboundUi::default();
        ui.metronome.enabled = true;
        let controls = SharedRuntimeControls::new(&ui);
        let mut metronome = MetronomeGenerator::new(48_000.0);

        let (left, right) = metronome.process(&controls);

        assert!(left.abs() > 0.0 || right.abs() > 0.0);
    }

    #[test]
    fn full_mute_probability_silences_scheduled_ticks() {
        let mut ui = GreyboundUi::default();
        ui.metronome.enabled = true;
        ui.metronome.mute_probability = 1.0;
        let controls = SharedRuntimeControls::new(&ui);
        let mut metronome = MetronomeGenerator::new(48_000.0);

        for _ in 0..48_000 {
            assert_eq!(metronome.process(&controls), (0.0, 0.0));
        }
    }

    #[test]
    fn skipped_tick_advances_the_accent_sequence() {
        let mut metronome = MetronomeGenerator::new(48_000.0);

        metronome.skip_tick(4);
        metronome.trigger(4);

        assert_eq!(metronome.frequency, 1_100.0);
    }
}
