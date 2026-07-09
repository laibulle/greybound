use rtrb::Consumer;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::controls::SharedRuntimeControls;
use super::util::{load_f32, store_f32};

pub(super) struct TunerAnalysisWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TunerAnalysisWorker {
    pub(super) fn start(
        sample_rate: f32,
        mut input: Consumer<f32>,
        controls: SharedRuntimeControls,
        stats: Arc<TunerStats>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let handle = thread::spawn(move || {
            const WINDOW: usize = 4096;
            const HOP: usize = 1024;
            let mut buffer = Vec::with_capacity(WINDOW * 2);
            let mut samples_since_analysis = 0usize;

            while !worker_stop.load(Ordering::Relaxed) {
                let mut drained = 0usize;
                while let Ok(sample) = input.pop() {
                    buffer.push(sample);
                    drained += 1;
                }

                if drained == 0 {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }

                if buffer.len() > WINDOW * 2 {
                    let excess = buffer.len() - WINDOW * 2;
                    buffer.drain(0..excess);
                }

                samples_since_analysis += drained;
                if samples_since_analysis < HOP {
                    continue;
                }
                samples_since_analysis = 0;

                if !controls.tuner_live() || buffer.len() < WINDOW {
                    stats.store_empty();
                    continue;
                }

                let start = buffer.len() - WINDOW;
                let mut window = Vec::with_capacity(WINDOW);
                let input_gain = controls.input_gain();
                window.extend(buffer[start..].iter().map(|sample| sample * input_gain));
                let reading = detect_pitch_autocorrelation(
                    &window,
                    sample_rate,
                    controls.tuner_reference_hz(),
                );
                stats.store(reading);
            }
        });

        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for TunerAnalysisWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TunerReading {
    pub(crate) frequency_hz: f32,
    pub(crate) cents: f32,
    pub(crate) confidence: f32,
}

#[derive(Default)]
pub(super) struct TunerStats {
    frequency_hz: AtomicU32,
    cents: AtomicU32,
    confidence: AtomicU32,
}

impl TunerStats {
    fn store(&self, reading: TunerReading) {
        store_f32(&self.frequency_hz, reading.frequency_hz);
        store_f32(&self.cents, reading.cents);
        store_f32(&self.confidence, reading.confidence);
    }

    fn store_empty(&self) {
        self.store(TunerReading {
            frequency_hz: 0.0,
            cents: 0.0,
            confidence: 0.0,
        });
    }

    pub(super) fn snapshot(&self) -> TunerReading {
        TunerReading {
            frequency_hz: load_f32(&self.frequency_hz),
            cents: load_f32(&self.cents),
            confidence: load_f32(&self.confidence),
        }
    }
}

fn detect_pitch_autocorrelation(
    samples: &[f32],
    sample_rate: f32,
    reference_hz: f32,
) -> TunerReading {
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let mut centered = Vec::with_capacity(samples.len());
    centered.extend(samples.iter().map(|sample| sample - mean));
    let rms =
        (centered.iter().map(|sample| sample * sample).sum::<f32>() / centered.len() as f32).sqrt();
    if rms < 0.003 {
        return empty_tuner_reading();
    }

    let min_lag = (sample_rate / 1_200.0).floor().max(1.0) as usize;
    let max_lag = (sample_rate / 65.0).ceil().min((centered.len() / 2) as f32) as usize;
    let mut best_lag = 0usize;
    let mut best_score = 0.0f32;

    for lag in min_lag..=max_lag {
        let mut xy = 0.0;
        let mut xx = 0.0;
        let mut yy = 0.0;
        for index in 0..(centered.len() - lag) {
            let a = centered[index];
            let b = centered[index + lag];
            xy += a * b;
            xx += a * a;
            yy += b * b;
        }
        let score = xy / (xx.sqrt() * yy.sqrt()).max(0.000_001);
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    if best_lag == 0 || best_score < 0.36 {
        return empty_tuner_reading();
    }

    let frequency_hz = sample_rate / best_lag as f32;
    let midi_note = (69.0 + 12.0 * (frequency_hz / reference_hz).log2()).round();
    let target_hz = reference_hz * 2.0_f32.powf((midi_note - 69.0) / 12.0);
    let cents = 1_200.0 * (frequency_hz / target_hz).log2();

    TunerReading {
        frequency_hz,
        cents: cents.clamp(-50.0, 50.0),
        confidence: ((best_score - 0.36) / 0.54).clamp(0.0, 1.0),
    }
}

fn empty_tuner_reading() -> TunerReading {
    TunerReading {
        frequency_hz: 0.0,
        cents: 0.0,
        confidence: 0.0,
    }
}
