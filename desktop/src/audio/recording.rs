use anyhow::{Context, Result};
use rtrb::{Consumer, Producer, RingBuffer};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const RECORDING_CHANNELS: u16 = 2;
const RECORDING_QUEUE_SECONDS: usize = 3;

pub(super) struct RecordingWorker {
    producer: Producer<(f32, f32)>,
    stop: Arc<AtomicBool>,
    dropped_frames: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
    path: PathBuf,
}

impl RecordingWorker {
    pub(super) fn start(path: PathBuf, sample_rate: u32, period_size: u32) -> Result<Self> {
        let writer = create_wav_writer(&path, sample_rate)?;
        let capacity =
            (sample_rate as usize * RECORDING_QUEUE_SECONDS).max(period_size as usize * 64);
        let (producer, consumer) = RingBuffer::<(f32, f32)>::new(capacity);
        let stop = Arc::new(AtomicBool::new(false));
        let dropped_frames = Arc::new(AtomicU64::new(0));
        let worker_stop = stop.clone();
        let worker_path = path.clone();
        let handle = thread::spawn(move || {
            write_recording_frames(writer, consumer, worker_stop, worker_path);
        });

        Ok(Self {
            producer,
            stop,
            dropped_frames,
            handle: Some(handle),
            path,
        })
    }

    pub(super) fn record(&mut self, left: f32, right: f32) {
        if self.producer.push((left, right)).is_err() {
            self.dropped_frames.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    #[allow(dead_code)]
    pub(super) fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }
}

impl Drop for RecordingWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn create_wav_writer(
    path: &Path,
    sample_rate: u32,
) -> Result<hound::WavWriter<std::io::BufWriter<std::fs::File>>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create recording directory {}", parent.display())
        })?;
    }

    hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: RECORDING_CHANNELS,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .with_context(|| format!("failed to create recording WAV {}", path.display()))
}

fn write_recording_frames(
    mut writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    mut consumer: Consumer<(f32, f32)>,
    stop: Arc<AtomicBool>,
    path: PathBuf,
) {
    while !stop.load(Ordering::Relaxed) {
        drain_recording_queue(&mut writer, &mut consumer);
        thread::sleep(Duration::from_millis(5));
    }

    drain_recording_queue(&mut writer, &mut consumer);
    if let Err(error) = writer.finalize() {
        eprintln!(
            "Greybound recording finalize error on {}: {error}",
            path.display()
        );
    }
}

fn drain_recording_queue(
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    consumer: &mut Consumer<(f32, f32)>,
) {
    while let Ok((left, right)) = consumer.pop() {
        if writer.write_sample(left).is_err() || writer.write_sample(right).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_worker_writes_stereo_float_wav() {
        let path = std::env::temp_dir().join(format!(
            "greybound-recording-worker-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        {
            let mut worker = RecordingWorker::start(path.clone(), 48_000, 32).unwrap();
            worker.record(0.25, -0.25);
            worker.record(0.5, -0.5);
        }

        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);

        let samples = hound::WavReader::open(&path)
            .unwrap()
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(samples.len(), 4);
        assert!((samples[0] - 0.25).abs() < 1.0e-6);
        assert!((samples[1] + 0.25).abs() < 1.0e-6);

        let _ = std::fs::remove_file(path);
    }
}
