use super::controls::SharedRuntimeControls;

const EQ_BAND_FREQUENCIES_HZ: [f32; greybound_ui::EQ_BAND_COUNT] = [
    65.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

pub(super) struct GraphicEqProcessor {
    sample_rate: f32,
    highpass: PeakingBiquad,
    lowpass: PeakingBiquad,
    last_hpf_hz: f32,
    last_lpf_hz: f32,
    bands: [PeakingBiquad; greybound_ui::EQ_BAND_COUNT],
    last_gains_db: [f32; greybound_ui::EQ_BAND_COUNT],
}

impl GraphicEqProcessor {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            highpass: PeakingBiquad::default(),
            lowpass: PeakingBiquad::default(),
            last_hpf_hz: f32::NAN,
            last_lpf_hz: f32::NAN,
            bands: std::array::from_fn(|_| PeakingBiquad::default()),
            last_gains_db: [f32::NAN; greybound_ui::EQ_BAND_COUNT],
        }
    }

    pub(super) fn process(&mut self, input: f32, controls: &SharedRuntimeControls) -> f32 {
        if !controls.eq_enabled() {
            return input;
        }

        let mut sample = input;
        let hpf_hz = controls.eq_hpf_hz().unwrap_or(0.0);
        if !self.last_hpf_hz.is_finite() || (hpf_hz - self.last_hpf_hz).abs() > 0.01 {
            if hpf_hz > 0.0 {
                self.highpass.set_highpass(self.sample_rate, hpf_hz, 0.707);
            } else {
                self.highpass.set_identity();
            }
            self.last_hpf_hz = hpf_hz;
        }
        sample = self.highpass.process(sample);

        for index in 0..greybound_ui::EQ_BAND_COUNT {
            let gain_db = controls.eq_band_gain_db(index);
            let last_gain_db = self.last_gains_db[index];
            if !last_gain_db.is_finite() || (gain_db - last_gain_db).abs() > 0.001 {
                self.bands[index].set_peaking(
                    self.sample_rate,
                    EQ_BAND_FREQUENCIES_HZ[index],
                    1.18,
                    gain_db,
                );
                self.last_gains_db[index] = gain_db;
            }
            sample = self.bands[index].process(sample);
        }

        let lpf_hz = controls.eq_lpf_hz().unwrap_or(0.0);
        if !self.last_lpf_hz.is_finite() || (lpf_hz - self.last_lpf_hz).abs() > 0.01 {
            if lpf_hz > 0.0 {
                self.lowpass.set_lowpass(self.sample_rate, lpf_hz, 0.707);
            } else {
                self.lowpass.set_identity();
            }
            self.last_lpf_hz = lpf_hz;
        }
        sample = self.lowpass.process(sample);
        sample.clamp(-8.0, 8.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct PeakingBiquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for PeakingBiquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl PeakingBiquad {
    fn set_identity(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
    }

    fn set_peaking(&mut self, sample_rate: f32, frequency_hz: f32, q: f32, gain_db: f32) {
        if gain_db.abs() < 0.001 {
            self.set_identity();
            return;
        }

        let nyquist = sample_rate * 0.5;
        let frequency_hz = frequency_hz.clamp(10.0, nyquist * 0.92);
        let omega = std::f32::consts::TAU * frequency_hz / sample_rate;
        let sin = omega.sin();
        let cos = omega.cos();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin / (2.0 * q.max(0.1));

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / a;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    fn set_highpass(&mut self, sample_rate: f32, frequency_hz: f32, q: f32) {
        let (sin, cos) = filter_sin_cos(sample_rate, frequency_hz);
        let alpha = sin / (2.0 * q.max(0.1));
        let b0 = (1.0 + cos) * 0.5;
        let b1 = -(1.0 + cos);
        let b2 = (1.0 + cos) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        self.set_normalized_coefficients(b0, b1, b2, a0, a1, a2);
    }

    fn set_lowpass(&mut self, sample_rate: f32, frequency_hz: f32, q: f32) {
        let (sin, cos) = filter_sin_cos(sample_rate, frequency_hz);
        let alpha = sin / (2.0 * q.max(0.1));
        let b0 = (1.0 - cos) * 0.5;
        let b1 = 1.0 - cos;
        let b2 = (1.0 - cos) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        self.set_normalized_coefficients(b0, b1, b2, a0, a1, a2);
    }

    fn set_normalized_coefficients(
        &mut self,
        b0: f32,
        b1: f32,
        b2: f32,
        a0: f32,
        a1: f32,
        a2: f32,
    ) {
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = input * self.b0 + self.z1;
        self.z1 = input * self.b1 + self.z2 - self.a1 * output;
        self.z2 = input * self.b2 - self.a2 * output;
        output
    }
}

fn filter_sin_cos(sample_rate: f32, frequency_hz: f32) -> (f32, f32) {
    let nyquist = sample_rate * 0.5;
    let frequency_hz = frequency_hz.clamp(10.0, nyquist * 0.92);
    let omega = std::f32::consts::TAU * frequency_hz / sample_rate;
    (omega.sin(), omega.cos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::controls::SharedRuntimeControls;
    use greybound_ui::GreyboundUi;

    #[test]
    fn disabled_preserves_input() {
        let mut ui = GreyboundUi::default();
        ui.eq.enabled = false;
        ui.eq.bands[4] = 1.0;
        let controls = SharedRuntimeControls::new(&ui);
        let mut eq = GraphicEqProcessor::new(48_000.0);

        for index in 0..256 {
            let sample = (index as f32 * 0.07).sin() * 0.25;
            let output = eq.process(sample, &controls);
            assert!((output - sample).abs() < 1.0e-7);
        }
    }

    #[test]
    fn band_boost_changes_signal_energy() {
        let mut flat_ui = GreyboundUi::default();
        flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        let flat_controls = SharedRuntimeControls::new(&flat_ui);
        let mut flat_eq = GraphicEqProcessor::new(48_000.0);

        let mut boost_ui = GreyboundUi::default();
        boost_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        boost_ui.eq.bands[4] = 1.0;
        let boost_controls = SharedRuntimeControls::new(&boost_ui);
        let mut boost_eq = GraphicEqProcessor::new(48_000.0);

        let mut flat_energy = 0.0;
        let mut boost_energy = 0.0;
        for index in 0..1024 {
            let phase = std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0;
            let sample = phase.sin() * 0.1;
            let flat = flat_eq.process(sample, &flat_controls);
            let boosted = boost_eq.process(sample, &boost_controls);
            flat_energy += flat * flat;
            boost_energy += boosted * boosted;
        }

        assert!(boost_energy.is_finite());
        assert!(
            boost_energy > flat_energy * 1.4,
            "flat={flat_energy}, boost={boost_energy}"
        );
    }

    #[test]
    fn hpf_reduces_low_frequency_energy() {
        let mut flat_ui = GreyboundUi::default();
        flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        let flat_controls = SharedRuntimeControls::new(&flat_ui);
        let mut flat_eq = GraphicEqProcessor::new(48_000.0);

        let mut filtered_ui = GreyboundUi::default();
        filtered_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        filtered_ui.eq.hpf = 1.0;
        let filtered_controls = SharedRuntimeControls::new(&filtered_ui);
        let mut filtered_eq = GraphicEqProcessor::new(48_000.0);

        let mut flat_energy = 0.0;
        let mut filtered_energy = 0.0;
        for index in 0..2_048 {
            let phase = std::f32::consts::TAU * 60.0 * index as f32 / 48_000.0;
            let sample = phase.sin() * 0.1;
            flat_energy += flat_eq.process(sample, &flat_controls).powi(2);
            filtered_energy += filtered_eq.process(sample, &filtered_controls).powi(2);
        }

        assert!(
            filtered_energy < flat_energy * 0.45,
            "flat={flat_energy}, filtered={filtered_energy}"
        );
    }

    #[test]
    fn lpf_reduces_high_frequency_energy() {
        let mut flat_ui = GreyboundUi::default();
        flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        let flat_controls = SharedRuntimeControls::new(&flat_ui);
        let mut flat_eq = GraphicEqProcessor::new(48_000.0);

        let mut filtered_ui = GreyboundUi::default();
        filtered_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
        filtered_ui.eq.lpf = 1.0;
        let filtered_controls = SharedRuntimeControls::new(&filtered_ui);
        let mut filtered_eq = GraphicEqProcessor::new(48_000.0);

        let mut flat_energy = 0.0;
        let mut filtered_energy = 0.0;
        for index in 0..2_048 {
            let phase = std::f32::consts::TAU * 8_000.0 * index as f32 / 48_000.0;
            let sample = phase.sin() * 0.1;
            flat_energy += flat_eq.process(sample, &flat_controls).powi(2);
            filtered_energy += filtered_eq.process(sample, &filtered_controls).powi(2);
        }

        assert!(
            filtered_energy < flat_energy * 0.25,
            "flat={flat_energy}, filtered={filtered_energy}"
        );
    }
}
