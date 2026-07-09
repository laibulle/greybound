use super::controls::SharedRuntimeControls;

pub(super) struct MetronomeGenerator {
    sample_rate: f32,
    samples_until_tick: f32,
    envelope: f32,
    phase: f32,
    frequency: f32,
    beat_index: u32,
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
            self.trigger(controls.metronome_beats_per_bar());
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
        self.beat_index = (self.beat_index + 1) % beats_per_bar.max(1);
    }

    fn samples_per_tick(&self, controls: &SharedRuntimeControls) -> f32 {
        let beats_per_second = controls.metronome_bpm() / 60.0;
        let ticks_per_beat = controls.metronome_rhythm_division() as f32;
        (self.sample_rate / (beats_per_second * ticks_per_beat)).max(1.0)
    }
}
