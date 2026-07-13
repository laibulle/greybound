use std::sync::atomic::{AtomicU64, Ordering};

const RMS_SCALE: f64 = 1_000_000_000.0;

#[derive(Default)]
pub(super) struct MeterStats {
    input_sum_squares: AtomicU64,
    input_count: AtomicU64,
    output_left_sum_squares: AtomicU64,
    output_right_sum_squares: AtomicU64,
    output_left_count: AtomicU64,
    output_right_count: AtomicU64,
    input_underruns: AtomicU64,
    input_overruns: AtomicU64,
}

impl MeterStats {
    pub(super) fn record_input(&self, sample: f32) {
        self.input_sum_squares
            .fetch_add(square(sample), Ordering::Relaxed);
        self.input_count.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_output(&self, left: f32, right: f32) {
        self.output_left_sum_squares
            .fetch_add(square(left), Ordering::Relaxed);
        self.output_right_sum_squares
            .fetch_add(square(right), Ordering::Relaxed);
        self.output_left_count.fetch_add(1, Ordering::Relaxed);
        self.output_right_count.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_input_underrun(&self) {
        self.input_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn record_input_overrun(&self) {
        self.input_overruns.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn input_queue_xruns(&self) -> (u64, u64) {
        (
            self.input_underruns.load(Ordering::Relaxed),
            self.input_overruns.load(Ordering::Relaxed),
        )
    }

    pub(super) fn snapshot_levels(&self) -> (f32, f32, f32) {
        let input_level = meter_from_accumulators(&self.input_sum_squares, &self.input_count);
        let output_left_level =
            meter_from_accumulators(&self.output_left_sum_squares, &self.output_left_count);
        let output_right_level =
            meter_from_accumulators(&self.output_right_sum_squares, &self.output_right_count);
        (input_level, output_left_level, output_right_level)
    }
}

fn square(sample: f32) -> u64 {
    let magnitude = sample.abs() as f64;
    (magnitude * magnitude * RMS_SCALE).round() as u64
}

fn meter_from_accumulators(sum_squares: &AtomicU64, count: &AtomicU64) -> f32 {
    let sum = sum_squares.swap(0, Ordering::Relaxed) as f64 / RMS_SCALE;
    let count = count.swap(0, Ordering::Relaxed);
    if count == 0 {
        return 0.0;
    }
    rms_to_meter((sum / count as f64).sqrt() as f32)
}

fn rms_to_meter(rms: f32) -> f32 {
    let db = 20.0 * rms.max(0.000_001).log10();
    ((db + 54.0) / 54.0).clamp(0.0, 1.0)
}
