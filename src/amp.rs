use rill_core_wdf::{filters::RcPole, WdfElement};

pub const AMP_LATENCY: usize = 16;
const OVERSAMPLING_FACTOR: f32 = 2.0;
const HALF_BAND_TAPS: usize = 33;

#[derive(Clone, Copy)]
pub struct AmpControls {
    pub volume: f32,
    pub bass: f32,
    pub treble: f32,
    pub cut: f32,
    pub output: f32,
}

/// Real-time graybox model of a JMI AC30/6 fitted with the OS/010 Top Boost unit.
///
/// The processing stages follow the reference topology while keeping the
/// nonlinear tube and transformer behavior deliberately compact. The complete
/// amp runs at 2x sample rate to reduce aliasing from every nonlinear stage.
pub struct VoxAmp {
    upsampler: FirFilter,
    core: AmpCore,
    downsampler: FirFilter,
}

impl VoxAmp {
    pub fn new(sample_rate: f32) -> Self {
        let coefficients = half_band_coefficients();
        Self {
            upsampler: FirFilter::new(coefficients),
            core: AmpCore::new(sample_rate * OVERSAMPLING_FACTOR),
            downsampler: FirFilter::new(coefficients),
        }
    }

    pub fn reset(&mut self) {
        self.upsampler.reset();
        self.core.reset();
        self.downsampler.reset();
    }

    #[inline]
    pub fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let upsampled = self.upsampler.process(input * OVERSAMPLING_FACTOR);
        let output = self
            .downsampler
            .process(self.core.process(upsampled, controls));

        let upsampled = self.upsampler.process(0.0);
        self.downsampler
            .process(self.core.process(upsampled, controls));
        output
    }
}

struct AmpCore {
    sample_rate: f32,
    input_coupling: WdfHighpass,
    first_cathode_bypass: WdfHighpass,
    recovery_cathode_bypass: WdfHighpass,
    bright_filter: OnePoleLowpass,
    tone_stack: TopBoostToneStack,
    phase_inverter_coupling: WdfHighpass,
    cut_filter: OnePoleLowpass,
    transformer_highpass: WdfHighpass,
    transformer_lowpass: OnePoleLowpass,
    bias_envelope: EnvelopeFollower,
    supply_sag: EnvelopeFollower,
}

impl AmpCore {
    fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            input_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            first_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_500.0, 25e-6),
            recovery_cathode_bypass: WdfHighpass::from_rc(sample_rate, 1_500.0, 25e-6),
            bright_filter: OnePoleLowpass::new(sample_rate, 2_900.0),
            tone_stack: TopBoostToneStack::new(sample_rate),
            phase_inverter_coupling: WdfHighpass::from_rc(sample_rate, 1_000_000.0, 47e-9),
            cut_filter: OnePoleLowpass::new(sample_rate, 12_000.0),
            transformer_highpass: WdfHighpass::from_rc(sample_rate, 100_000.0, 47e-9),
            transformer_lowpass: OnePoleLowpass::new(sample_rate, 14_000.0),
            bias_envelope: EnvelopeFollower::new(sample_rate, 0.004, 0.120),
            supply_sag: EnvelopeFollower::new(sample_rate, 0.012, 0.260),
        }
    }

    fn reset(&mut self) {
        *self = Self::new(self.sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32, controls: AmpControls) -> f32 {
        let input = self.input_coupling.process(input);

        // OS/010 uses a 500k volume control with a 100pF bright capacitor.
        // At lower settings the capacitor bypasses more high-frequency signal.
        let volume = controls.volume * controls.volume;
        let high = input - self.bright_filter.process(input);
        let volume_output = input * volume + high * (1.0 - volume) * 0.18;

        let first_bypass = self.first_cathode_bypass.process(volume_output);
        let first_drive = volume_output * 4.8 + first_bypass * 0.8;
        let first_stage = triode_stage(first_drive, 0.16);

        let toned = self
            .tone_stack
            .process(first_stage, controls.bass, controls.treble);
        let recovery_bypass = self.recovery_cathode_bypass.process(toned);
        let recovery = triode_stage(toned * 4.2 + recovery_bypass * 0.7, 0.12);

        // The long-tail pair produces opposed, slightly imbalanced outputs. The
        // Cut network sits across those outputs, before the EL84 grid couplers.
        let pi_input = self.phase_inverter_coupling.process(recovery);
        let phase_a = triode_stage(pi_input * 1.38, 0.045);
        let phase_b = triode_stage(-pi_input * 1.32, -0.035);
        let differential = (phase_a - phase_b) * 0.5;
        let cut_hz = 13_500.0 * (1.0 - controls.cut).powi(2) + 1_150.0;
        self.cut_filter.set_cutoff(self.sample_rate, cut_hz);
        let cut_output = self.cut_filter.process(differential);

        // Four hot cathode-biased EL84s behave as push-pull class AB, not ideal
        // class A. Bias shift and supply sag mainly appear when the output stage
        // is driven beyond its clean-current region.
        let current_demand = (cut_output.abs() * 1.60 - 0.62).max(0.0);
        let bias_shift = self.bias_envelope.process(current_demand);
        let sag = self.supply_sag.process(current_demand * current_demand);
        let drive = cut_output * 1.60 / (1.0 + bias_shift * 0.55 + sag * 0.22);
        let positive_bank = el84_bank(drive - bias_shift * 0.055);
        let negative_bank = el84_bank(-drive - bias_shift * 0.045);
        let power_output = (positive_bank - negative_bank) * 0.72;

        let transformer = self.transformer_highpass.process(power_output);
        let transformer = self.transformer_lowpass.process(transformer);
        transformer * controls.output
    }
}

struct FirFilter {
    coefficients: [f32; HALF_BAND_TAPS],
    history: [f32; HALF_BAND_TAPS],
    position: usize,
}

impl FirFilter {
    fn new(coefficients: [f32; HALF_BAND_TAPS]) -> Self {
        Self {
            coefficients,
            history: [0.0; HALF_BAND_TAPS],
            position: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.history[self.position] = input;
        let mut output = 0.0;
        let mut history_position = self.position;
        for coefficient in self.coefficients {
            output += coefficient * self.history[history_position];
            history_position = if history_position == 0 {
                HALF_BAND_TAPS - 1
            } else {
                history_position - 1
            };
        }
        self.position = (self.position + 1) % HALF_BAND_TAPS;
        output
    }

    fn reset(&mut self) {
        self.history.fill(0.0);
        self.position = 0;
    }
}

fn half_band_coefficients() -> [f32; HALF_BAND_TAPS] {
    let center = (HALF_BAND_TAPS - 1) as f32 * 0.5;
    let mut coefficients = [0.0; HALF_BAND_TAPS];
    let mut sum = 0.0;
    for (index, coefficient) in coefficients.iter_mut().enumerate() {
        let offset = index as f32 - center;
        let sinc = if offset == 0.0 {
            0.5
        } else {
            (std::f32::consts::PI * offset * 0.5).sin() / (std::f32::consts::PI * offset)
        };
        let phase = std::f32::consts::TAU * index as f32 / (HALF_BAND_TAPS - 1) as f32;
        let blackman = 0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos();
        *coefficient = sinc * blackman;
        sum += *coefficient;
    }
    for coefficient in &mut coefficients {
        *coefficient /= sum;
    }
    coefficients
}

struct TopBoostToneStack {
    bass_split: OnePoleLowpass,
    treble_split: OnePoleLowpass,
}

impl TopBoostToneStack {
    fn new(sample_rate: f32) -> Self {
        Self {
            bass_split: OnePoleLowpass::new(sample_rate, 170.0),
            treble_split: OnePoleLowpass::new(sample_rate, 1_850.0),
        }
    }

    #[inline]
    fn process(&mut self, input: f32, bass: f32, treble: f32) -> f32 {
        let low = self.bass_split.process(input);
        let high = input - self.treble_split.process(input);
        let mid = input - low - high;
        let bass = bass * bass;
        let treble = treble * treble;

        // The OS/010 1M controls are strongly interactive. Raising both creates
        // the characteristic mid scoop; backing either down restores mids.
        let low_gain = 0.08 + bass * 0.58;
        let high_gain = 0.07 + treble * 0.78;
        let mid_gain = 0.30 - bass * treble * 0.17 + (1.0 - bass) * (1.0 - treble) * 0.08;
        low * low_gain + mid * mid_gain + high * high_gain
    }
}

struct EnvelopeFollower {
    attack: f32,
    release: f32,
    state: f32,
}

impl EnvelopeFollower {
    fn new(sample_rate: f32, attack_seconds: f32, release_seconds: f32) -> Self {
        Self {
            attack: (-1.0 / (sample_rate * attack_seconds)).exp(),
            release: (-1.0 / (sample_rate * release_seconds)).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let coefficient = if input > self.state {
            self.attack
        } else {
            self.release
        };
        self.state = input + coefficient * (self.state - input);
        self.state
    }
}

struct WdfHighpass {
    lowpass: RcPole<f32>,
}

impl WdfHighpass {
    fn from_rc(sample_rate: f32, resistance: f32, capacitance: f32) -> Self {
        let cutoff = 1.0 / (std::f32::consts::TAU * resistance * capacitance);
        let g = std::f32::consts::PI * cutoff / sample_rate;
        Self {
            lowpass: RcPole::new(g / (1.0 + g)),
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        input - self.lowpass.process_incident(input)
    }
}

#[inline]
fn triode_stage(input: f32, bias: f32) -> f32 {
    let biased = input + bias;
    (biased.tanh() - bias.tanh()) * 1.08
}

#[inline]
fn el84_bank(input: f32) -> f32 {
    let conducting = (input + 0.18).max(0.0);
    (conducting - 0.055 * conducting * conducting * conducting).tanh()
}

struct OnePoleLowpass {
    coefficient: f32,
    cutoff: f32,
    state: f32,
}

impl OnePoleLowpass {
    fn new(sample_rate: f32, cutoff: f32) -> Self {
        let mut filter = Self {
            coefficient: 0.0,
            cutoff: f32::NAN,
            state: 0.0,
        };
        filter.set_cutoff(sample_rate, cutoff);
        filter
    }

    fn set_cutoff(&mut self, sample_rate: f32, cutoff: f32) {
        if cutoff != self.cutoff {
            self.coefficient = 1.0 - (-std::f32::consts::TAU * cutoff / sample_rate).exp();
            self.cutoff = cutoff;
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input - self.state);
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls() -> AmpControls {
        AmpControls {
            volume: 0.5,
            bass: 0.5,
            treble: 0.5,
            cut: 0.5,
            output: 1.0,
        }
    }

    fn sine_rms_at(amp: &mut VoxAmp, frequency: f32, amplitude: f32, controls: AmpControls) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum = 0.0;
        for sample_idx in 0..9_600 {
            let input = (std::f32::consts::TAU * frequency * sample_idx as f32 / sample_rate).sin()
                * amplitude;
            let output = amp.process(input, controls);
            if sample_idx >= 4_800 {
                sum += output * output;
            }
        }
        (sum / 4_800.0).sqrt()
    }

    fn sine_rms(amp: &mut VoxAmp, frequency: f32, controls: AmpControls) -> f32 {
        sine_rms_at(amp, frequency, 0.02, controls)
    }

    #[test]
    fn silence_stays_silent() {
        let mut amp = VoxAmp::new(48_000.0);
        for _ in 0..1024 {
            assert!(amp.process(0.0, controls()).abs() < 1e-6);
        }
    }

    #[test]
    fn output_is_finite_under_extreme_input() {
        let mut amp = VoxAmp::new(48_000.0);
        let mut controls = controls();
        controls.volume = 1.0;
        controls.bass = 1.0;
        controls.treble = 1.0;
        controls.cut = 0.0;
        controls.output = 2.0;

        for sample in [0.0, 1.0, -1.0, 100.0, -100.0]
            .into_iter()
            .cycle()
            .take(4096)
        {
            assert!(amp.process(sample, controls).is_finite());
        }
    }

    #[test]
    fn bass_control_changes_low_frequency_response() {
        let mut low_bass = controls();
        low_bass.bass = 0.0;
        let mut high_bass = low_bass;
        high_bass.bass = 1.0;

        let low = sine_rms(&mut VoxAmp::new(48_000.0), 90.0, low_bass);
        let high = sine_rms(&mut VoxAmp::new(48_000.0), 90.0, high_bass);
        assert!(high > low * 1.2);
    }

    #[test]
    fn cut_control_reduces_high_frequency_response() {
        let mut open = controls();
        open.cut = 0.0;
        let mut cut = open;
        cut.cut = 1.0;

        let open_level = sine_rms(&mut VoxAmp::new(48_000.0), 5_000.0, open);
        let cut_level = sine_rms(&mut VoxAmp::new(48_000.0), 5_000.0, cut);
        assert!(open_level > cut_level * 1.4);
    }

    #[test]
    fn treble_control_changes_high_frequency_response() {
        let mut low_treble = controls();
        low_treble.volume = 0.1;
        low_treble.treble = 0.0;
        let mut high_treble = low_treble;
        high_treble.treble = 1.0;

        let low = sine_rms(&mut VoxAmp::new(48_000.0), 4_000.0, low_treble);
        let high = sine_rms(&mut VoxAmp::new(48_000.0), 4_000.0, high_treble);
        assert!(high > low * 1.2);
    }

    #[test]
    fn clean_setting_preserves_pick_dynamics() {
        let mut clean = controls();
        clean.volume = 0.28;

        let quiet = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.025, clean);
        let loud = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.05, clean);
        assert!(loud > quiet * 1.8);
    }

    #[test]
    fn volume_does_not_change_fixed_power_stage_gain() {
        let mut low = controls();
        low.volume = 0.2;
        let mut high = low;
        high.volume = 0.4;

        let low_level = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.01, low);
        let high_level = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.01, high);
        assert!(high_level > low_level * 2.5);
        assert!(high_level < low_level * 6.0);
    }

    #[test]
    fn fully_driven_setting_compresses_more_than_clean() {
        let mut clean = controls();
        clean.volume = 0.32;
        let mut driven = clean;
        driven.volume = 1.0;

        let clean_quiet = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.05, clean);
        let clean_loud = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.10, clean);
        let driven_quiet = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.05, driven);
        let driven_loud = sine_rms_at(&mut VoxAmp::new(48_000.0), 440.0, 0.10, driven);

        assert!(clean_loud / clean_quiet > driven_loud / driven_quiet);
    }

    #[test]
    fn oversampling_latency_matches_reported_latency() {
        let coefficients = half_band_coefficients();
        let mut upsampler = FirFilter::new(coefficients);
        let mut downsampler = FirFilter::new(coefficients);
        let mut output = Vec::new();

        for sample_idx in 0..64 {
            let first = upsampler.process((sample_idx == 0) as u8 as f32 * OVERSAMPLING_FACTOR);
            output.push(downsampler.process(first));
            let second = upsampler.process(0.0);
            downsampler.process(second);
        }

        let peak = output
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
            .unwrap()
            .0;
        assert_eq!(peak, AMP_LATENCY);
    }

    #[test]
    fn oversampling_reduces_high_frequency_aliasing() {
        const SAMPLE_RATE: f32 = 48_000.0;
        const INPUT_FREQUENCY: f32 = 10_000.0;
        const ALIAS_FREQUENCY: f32 = 18_000.0;
        const SAMPLES: usize = 24_000;

        let mut driven = controls();
        driven.volume = 1.0;
        driven.treble = 1.0;
        driven.cut = 0.0;

        let mut base_rate = AmpCore::new(SAMPLE_RATE);
        let mut oversampled = VoxAmp::new(SAMPLE_RATE);
        let mut base_output = Vec::with_capacity(SAMPLES);
        let mut oversampled_output = Vec::with_capacity(SAMPLES);
        for sample_idx in 0..SAMPLES {
            let input = (std::f32::consts::TAU * INPUT_FREQUENCY * sample_idx as f32 / SAMPLE_RATE)
                .sin()
                * 0.35;
            base_output.push(base_rate.process(input, driven));
            oversampled_output.push(oversampled.process(input, driven));
        }

        let base_alias = tone_magnitude(&base_output[SAMPLES / 2..], ALIAS_FREQUENCY, SAMPLE_RATE);
        let oversampled_alias = tone_magnitude(
            &oversampled_output[SAMPLES / 2..],
            ALIAS_FREQUENCY,
            SAMPLE_RATE,
        );
        assert!(
            oversampled_alias < base_alias * 0.5,
            "alias magnitude: base={base_alias}, oversampled={oversampled_alias}"
        );
    }

    fn tone_magnitude(samples: &[f32], frequency: f32, sample_rate: f32) -> f32 {
        let (real, imaginary) =
            samples
                .iter()
                .enumerate()
                .fold((0.0, 0.0), |(real, imaginary), (index, sample)| {
                    let phase = std::f32::consts::TAU * frequency * index as f32 / sample_rate;
                    (
                        real + sample * phase.cos(),
                        imaginary - sample * phase.sin(),
                    )
                });
        (real * real + imaginary * imaginary).sqrt() * 2.0 / samples.len() as f32
    }
}
