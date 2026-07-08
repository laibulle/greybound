use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub struct WavInput {
    pub path: PathBuf,
    pub samples: Vec<f32>,
    pub channels: usize,
    pub sample_rate: u32,
}

pub fn load_wav_input(path: &Path, input_channel: usize) -> Result<WavInput> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("could not open input WAV '{}'", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    if input_channel >= channels {
        bail!(
            "input channel {} is unavailable; '{}' has {} channel(s)",
            input_channel + 1,
            path.display(),
            channels
        );
    }

    let mut samples = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for (index, sample) in reader.samples::<f32>().enumerate() {
                let sample = sample.with_context(|| {
                    format!("could not read float sample from '{}'", path.display())
                })?;
                if index % channels == input_channel {
                    samples.push(sample);
                }
            }
        }
        hound::SampleFormat::Int => {
            let scale = 2.0_f32.powi(spec.bits_per_sample as i32 - 1);
            for (index, sample) in reader.samples::<i32>().enumerate() {
                let sample = sample.with_context(|| {
                    format!("could not read int sample from '{}'", path.display())
                })? as f32
                    / scale;
                if index % channels == input_channel {
                    samples.push(sample);
                }
            }
        }
    }

    if samples.is_empty() {
        bail!("input WAV '{}' contains no samples", path.display());
    }

    Ok(WavInput {
        path: path.to_path_buf(),
        samples,
        channels,
        sample_rate: spec.sample_rate,
    })
}

pub fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if source_rate == target_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let target_len = ((samples.len() as u64 * target_rate as u64 + source_rate as u64 / 2)
        / source_rate as u64)
        .max(1) as usize;
    let rate_ratio = source_rate as f64 / target_rate as f64;
    let mut resampled = Vec::with_capacity(target_len);

    for index in 0..target_len {
        let source_position = index as f64 * rate_ratio;
        let left_index = source_position.floor() as usize;
        let right_index = (left_index + 1).min(samples.len() - 1);
        let fraction = (source_position - left_index as f64) as f32;
        let left = samples[left_index.min(samples.len() - 1)];
        let right = samples[right_index];
        resampled.push(left + (right - left) * fraction);
    }

    resampled
}
