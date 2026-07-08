pub(super) const OVERSAMPLING_FACTOR: f32 = 2.0;
const HALF_BAND_TAPS: usize = 33;

pub(super) struct FirFilter {
    coefficients: [f32; HALF_BAND_TAPS],
    history: [f32; HALF_BAND_TAPS],
    position: usize,
}

impl FirFilter {
    pub(super) fn new(coefficients: [f32; HALF_BAND_TAPS]) -> Self {
        Self {
            coefficients,
            history: [0.0; HALF_BAND_TAPS],
            position: 0,
        }
    }

    #[inline]
    pub(super) fn process(&mut self, input: f32) -> f32 {
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

    pub(super) fn reset(&mut self) {
        self.history.fill(0.0);
        self.position = 0;
    }
}

pub(super) fn half_band_coefficients() -> [f32; HALF_BAND_TAPS] {
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
