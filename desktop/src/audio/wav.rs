use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(super) struct WavPlaybackBuffer {
    pub(super) label: String,
    samples: Vec<f32>,
}

impl WavPlaybackBuffer {
    pub(super) fn load(path: &Path, target_sample_rate: u32) -> Result<Self> {
        let mut reader = hound::WavReader::open(path)
            .with_context(|| format!("failed to open WAV file {}", path.display()))?;
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;
        let mono = match spec.sample_format {
            hound::SampleFormat::Float => {
                let samples = reader
                    .samples::<f32>()
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .context("failed to read floating-point WAV samples")?;
                mix_interleaved_to_mono(&samples, channels)
            }
            hound::SampleFormat::Int => {
                if spec.bits_per_sample <= 16 {
                    let samples = reader
                        .samples::<i16>()
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .context("failed to read 16-bit WAV samples")?;
                    let samples: Vec<f32> = samples
                        .into_iter()
                        .map(|sample| sample as f32 / i16::MAX as f32)
                        .collect();
                    mix_interleaved_to_mono(&samples, channels)
                } else {
                    let scale = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
                    let samples = reader
                        .samples::<i32>()
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .context("failed to read high-bit-depth WAV samples")?;
                    let samples: Vec<f32> = samples
                        .into_iter()
                        .map(|sample| sample as f32 / scale)
                        .collect();
                    mix_interleaved_to_mono(&samples, channels)
                }
            }
        };
        let samples = resample_linear(&mono, spec.sample_rate, target_sample_rate);
        if samples.is_empty() {
            anyhow::bail!("WAV file {} contains no audio samples", path.display());
        }

        Ok(Self {
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file")
                .to_string(),
            samples,
        })
    }
}

fn mix_interleaved_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples
            .iter()
            .copied()
            .map(|sample| sample.clamp(-1.0, 1.0))
            .collect();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| (frame.iter().sum::<f32>() / channels as f32).clamp(-1.0, 1.0))
        .collect()
}

fn resample_linear(samples: &[f32], source_sample_rate: u32, target_sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_sample_rate == 0 || target_sample_rate == 0 {
        return Vec::new();
    }
    if source_sample_rate == target_sample_rate {
        return samples.to_vec();
    }

    let output_len = ((samples.len() as u64 * target_sample_rate as u64)
        / source_sample_rate as u64)
        .max(1) as usize;
    let ratio = source_sample_rate as f64 / target_sample_rate as f64;
    (0..output_len)
        .map(|index| {
            let source_position = index as f64 * ratio;
            let base = source_position.floor() as usize;
            let next = (base + 1).min(samples.len() - 1);
            let fraction = (source_position - base as f64) as f32;
            samples[base] * (1.0 - fraction) + samples[next] * fraction
        })
        .collect()
}

pub(super) struct FilePlaybackWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FilePlaybackWorker {
    pub(super) fn start(
        file: WavPlaybackBuffer,
        mut producer: rtrb::Producer<f32>,
        mut tuner_producer: rtrb::Producer<f32>,
        sample_rate: u32,
        period_size: u32,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let handle = thread::spawn(move || {
            let samples = file.samples;
            let mut position = 0usize;
            let chunk_size = period_size.max(16) as usize;
            let chunk_duration =
                Duration::from_secs_f64(chunk_size as f64 / sample_rate.max(1) as f64);

            while !worker_stop.load(Ordering::Relaxed) {
                for _ in 0..chunk_size {
                    let sample = samples[position];
                    let _ = producer.push(sample);
                    let _ = tuner_producer.push(sample);
                    position = (position + 1) % samples.len();
                }
                thread::sleep(chunk_duration);
            }
        });

        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for FilePlaybackWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_interleaved_stereo_to_mono_averages_channels() {
        let mono = mix_interleaved_to_mono(&[1.0, -1.0, 0.5, 0.25], 2);

        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < 1.0e-7);
        assert!((mono[1] - 0.375).abs() < 1.0e-7);
    }

    #[test]
    fn resampler_converts_sample_count_to_target_rate() {
        let samples = [0.0, 1.0, 0.0, -1.0];
        let resampled = resample_linear(&samples, 4, 8);

        assert_eq!(resampled.len(), 8);
        assert!((resampled[0] - 0.0).abs() < 1.0e-7);
        assert!((resampled[1] - 0.5).abs() < 1.0e-7);
        assert!((resampled[2] - 1.0).abs() < 1.0e-7);
    }
}
