use anyhow::{bail, Context, Result};
use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

pub const CONVOLUTION_LATENCY: usize = 0;
const CONVOLUTION_PARTITION_SIZE: usize = 256;
const FFT_SIZE: usize = CONVOLUTION_PARTITION_SIZE * 2;

pub struct SpeakerStage {
    head: Option<DirectConvolver>,
    tail: Option<PartitionedConvolver>,
}

impl SpeakerStage {
    pub fn from_embedded_ir(sample_rate: u32) -> Result<Self> {
        Self::new(reference_speaker_ir(sample_rate))
    }

    pub fn from_wav_path(path: impl AsRef<Path>, sample_rate: u32) -> Result<Self> {
        Self::new(load_wav_ir(path.as_ref(), sample_rate)?)
    }

    pub fn from_wav_bytes(bytes: &[u8], sample_rate: u32) -> Result<Self> {
        Self::new(decode_wav_ir(bytes, sample_rate)?)
    }

    pub fn new(ir: Vec<f32>) -> Result<Self> {
        if ir.is_empty() {
            bail!("speaker IR is empty");
        }
        let head_len = ir.len().min(CONVOLUTION_PARTITION_SIZE);
        let head = DirectConvolver::new(&ir[..head_len]);
        let tail = (ir.len() > CONVOLUTION_PARTITION_SIZE)
            .then(|| PartitionedConvolver::new(&ir[CONVOLUTION_PARTITION_SIZE..]))
            .transpose()?;

        Ok(Self {
            head: Some(head),
            tail,
        })
    }

    pub fn bypassed() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32, enabled: bool) -> f32 {
        if !enabled {
            return input;
        }
        let head = self
            .head
            .as_mut()
            .map_or(input, |convolver| convolver.process(input));
        let tail = self
            .tail
            .as_mut()
            .map_or(0.0, |convolver| convolver.process(input));
        head + tail
    }

    pub fn reset(&mut self) {
        if let Some(convolver) = &mut self.head {
            convolver.reset();
        }
        if let Some(convolver) = &mut self.tail {
            convolver.reset();
        }
    }
}

struct DirectConvolver {
    taps: Vec<f32>,
    delay: Vec<f32>,
    pos: usize,
}

impl DirectConvolver {
    fn new(taps: &[f32]) -> Self {
        Self {
            taps: taps.to_vec(),
            delay: vec![0.0; taps.len()],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.delay[self.pos] = input;
        let mut output = 0.0;
        let mut delay_idx = self.pos;
        for tap in &self.taps {
            output += *tap * self.delay[delay_idx];
            delay_idx = if delay_idx == 0 {
                self.delay.len() - 1
            } else {
                delay_idx - 1
            };
        }
        self.pos = (self.pos + 1) % self.delay.len();
        output
    }

    fn reset(&mut self) {
        self.delay.fill(0.0);
        self.pos = 0;
    }
}

struct PartitionedConvolver {
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    ir_partitions: Vec<Vec<Complex32>>,
    input_history: Vec<Vec<Complex32>>,
    history_pos: usize,
    input_block: [f32; CONVOLUTION_PARTITION_SIZE],
    output_block: [f32; CONVOLUTION_PARTITION_SIZE],
    overlap: [f32; CONVOLUTION_PARTITION_SIZE],
    block_pos: usize,
    real_buffer: Vec<f32>,
    input_spectrum: Vec<Complex32>,
    output_spectrum: Vec<Complex32>,
    r2c_scratch: Vec<Complex32>,
    c2r_scratch: Vec<Complex32>,
}

impl PartitionedConvolver {
    fn new(ir: &[f32]) -> Result<Self> {
        if ir.is_empty() {
            bail!("speaker IR is empty");
        }

        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(FFT_SIZE);
        let c2r = planner.plan_fft_inverse(FFT_SIZE);
        let spectrum_len = r2c.make_output_vec().len();
        let partition_count = ir.len().div_ceil(CONVOLUTION_PARTITION_SIZE);
        let mut real_buffer = r2c.make_input_vec();
        let mut spectrum = r2c.make_output_vec();
        let mut r2c_scratch = r2c.make_scratch_vec();
        let c2r_scratch = c2r.make_scratch_vec();
        let mut ir_partitions = Vec::with_capacity(partition_count);

        for partition in ir.chunks(CONVOLUTION_PARTITION_SIZE) {
            real_buffer.fill(0.0);
            real_buffer[..partition.len()].copy_from_slice(partition);
            r2c.process_with_scratch(&mut real_buffer, &mut spectrum, &mut r2c_scratch)
                .context("could not transform speaker IR")?;
            ir_partitions.push(spectrum.clone());
        }

        Ok(Self {
            r2c,
            c2r,
            ir_partitions,
            input_history: vec![vec![Complex32::default(); spectrum_len]; partition_count],
            history_pos: 0,
            input_block: [0.0; CONVOLUTION_PARTITION_SIZE],
            output_block: [0.0; CONVOLUTION_PARTITION_SIZE],
            overlap: [0.0; CONVOLUTION_PARTITION_SIZE],
            block_pos: 0,
            real_buffer,
            input_spectrum: vec![Complex32::default(); spectrum_len],
            output_spectrum: vec![Complex32::default(); spectrum_len],
            r2c_scratch,
            c2r_scratch,
        })
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.output_block[self.block_pos];
        self.input_block[self.block_pos] = input;
        self.block_pos += 1;

        if self.block_pos == CONVOLUTION_PARTITION_SIZE {
            self.process_block();
            self.block_pos = 0;
        }

        output
    }

    fn process_block(&mut self) {
        self.real_buffer[..CONVOLUTION_PARTITION_SIZE].copy_from_slice(&self.input_block);
        self.real_buffer[CONVOLUTION_PARTITION_SIZE..].fill(0.0);
        self.r2c
            .process_with_scratch(
                &mut self.real_buffer,
                &mut self.input_spectrum,
                &mut self.r2c_scratch,
            )
            .expect("preallocated FFT buffers have valid sizes");
        self.input_history[self.history_pos].copy_from_slice(&self.input_spectrum);
        self.output_spectrum.fill(Complex32::default());

        for (partition_idx, ir_spectrum) in self.ir_partitions.iter().enumerate() {
            let history_idx = (self.history_pos + self.input_history.len() - partition_idx)
                % self.input_history.len();
            for ((output, input), ir) in self
                .output_spectrum
                .iter_mut()
                .zip(&self.input_history[history_idx])
                .zip(ir_spectrum)
            {
                *output += *input * *ir;
            }
        }

        self.c2r
            .process_with_scratch(
                &mut self.output_spectrum,
                &mut self.real_buffer,
                &mut self.c2r_scratch,
            )
            .expect("preallocated FFT buffers have valid sizes");
        let normalization = 1.0 / FFT_SIZE as f32;
        for sample_idx in 0..CONVOLUTION_PARTITION_SIZE {
            self.output_block[sample_idx] =
                self.real_buffer[sample_idx] * normalization + self.overlap[sample_idx];
            self.overlap[sample_idx] =
                self.real_buffer[sample_idx + CONVOLUTION_PARTITION_SIZE] * normalization;
        }

        self.history_pos = (self.history_pos + 1) % self.input_history.len();
    }

    fn reset(&mut self) {
        for spectrum in &mut self.input_history {
            spectrum.fill(Complex32::default());
        }
        self.history_pos = 0;
        self.input_block.fill(0.0);
        self.output_block.fill(0.0);
        self.overlap.fill(0.0);
        self.block_pos = 0;
    }
}

fn load_wav_ir(path: &Path, sample_rate: u32) -> Result<Vec<f32>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("could not read reference speaker IR at {}", path.display()))?;
    decode_wav_ir(&bytes, sample_rate)
}

fn decode_wav_ir(bytes: &[u8], sample_rate: u32) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::new(Cursor::new(bytes)).context("could not decode speaker IR WAV")?;
    let spec = reader.spec();
    if spec.channels == 0 {
        bail!("reference speaker IR has an unexpected format");
    }

    let decoded = match spec.sample_format {
        hound::SampleFormat::Float => {
            if spec.bits_per_sample != 32 {
                bail!("reference speaker IR has an unsupported float format");
            }
            reader
                .samples::<f32>()
                .map(|sample| sample.context("could not decode speaker IR"))
                .collect::<Result<Vec<_>>>()?
        }
        hound::SampleFormat::Int => {
            if spec.bits_per_sample == 0 || spec.bits_per_sample > 32 {
                bail!("reference speaker IR has an unsupported integer format");
            }
            let scale = 2.0_f32.powi(spec.bits_per_sample as i32 - 1);
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| (value as f32 / scale).clamp(-1.0, 1.0))
                        .context("could not decode speaker IR")
                })
                .collect::<Result<Vec<_>>>()?
        }
    };

    let mono = downmix_to_mono(&decoded, spec.channels as usize);
    Ok(resample_linear(&mono, spec.sample_rate, sample_rate))
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
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

fn reference_speaker_ir(sample_rate: u32) -> Vec<f32> {
    let sample_rate = sample_rate.max(8_000) as f32;
    let len = (sample_rate * 0.045).round().clamp(512.0, 4096.0) as usize;
    let mut ir = Vec::with_capacity(len);
    let mut highpass = Biquad::highpass(sample_rate, 85.0, 0.72);
    let mut low_body = Biquad::peaking(sample_rate, 135.0, 0.85, 1.8);
    let mut cone_bite = Biquad::peaking(sample_rate, 1_850.0, 1.05, 2.6);
    let mut fizz_cut = Biquad::peaking(sample_rate, 3_900.0, 0.80, -4.5);
    let mut lowpass = Biquad::lowpass(sample_rate, 5_400.0, 0.66);
    let mut air_rolloff = Biquad::lowpass(sample_rate, 7_200.0, 0.72);

    for sample_idx in 0..len {
        let mut sample = if sample_idx == 0 { 1.0 } else { 0.0 };
        sample = highpass.process(sample);
        sample = low_body.process(sample);
        sample = cone_bite.process(sample);
        sample = fizz_cut.process(sample);
        sample = lowpass.process(sample);
        sample = air_rolloff.process(sample);
        let age_seconds = sample_idx as f32 / sample_rate;
        ir.push(sample * (-age_seconds / 0.018).exp());
    }

    add_reflection(&mut ir, sample_rate, 0.42, -0.18);
    add_reflection(&mut ir, sample_rate, 0.86, 0.13);
    add_reflection(&mut ir, sample_rate, 1.55, -0.09);
    normalize_ir_peak(&mut ir, 0.72);
    ir
}

fn add_reflection(ir: &mut [f32], sample_rate: f32, delay_ms: f32, gain: f32) {
    let delay = (delay_ms * 0.001 * sample_rate).round().max(1.0) as usize;
    if delay >= ir.len() {
        return;
    }

    for sample_idx in (delay..ir.len()).rev() {
        ir[sample_idx] += ir[sample_idx - delay] * gain;
    }
}

fn normalize_ir_peak(ir: &mut [f32], target_peak: f32) {
    let peak = ir
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak <= f32::EPSILON {
        return;
    }

    let scale = target_peak / peak;
    for sample in ir {
        *sample *= scale;
    }
}

struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn highpass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let omega = std::f32::consts::TAU * frequency.min(sample_rate * 0.45) / sample_rate;
        let sin = omega.sin();
        let cos = omega.cos();
        let alpha = sin / (2.0 * q.max(0.1));
        let b0 = (1.0 + cos) * 0.5;
        let b1 = -(1.0 + cos);
        let b2 = (1.0 + cos) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    fn lowpass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let omega = std::f32::consts::TAU * frequency.min(sample_rate * 0.45) / sample_rate;
        let sin = omega.sin();
        let cos = omega.cos();
        let alpha = sin / (2.0 * q.max(0.1));
        let b0 = (1.0 - cos) * 0.5;
        let b1 = 1.0 - cos;
        let b2 = (1.0 - cos) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    fn peaking(sample_rate: f32, frequency: f32, q: f32, gain_db: f32) -> Self {
        let omega = std::f32::consts::TAU * frequency.min(sample_rate * 0.45) / sample_rate;
        let sin = omega.sin();
        let cos = omega.cos();
        let alpha = sin / (2.0 * q.max(0.1));
        let gain = 10.0_f32.powf(gain_db / 40.0);
        let b0 = 1.0 + alpha * gain;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * gain;
        let a0 = 1.0 + alpha / gain;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / gain;
        Self::normalized(b0, b1, b2, a0, a1, a2)
    }

    fn normalized(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_starts_without_convolution_latency() {
        let mut stage = SpeakerStage::new(vec![1.0]).unwrap();
        let mut output = Vec::new();
        for sample_idx in 0..4 {
            output.push(stage.process((sample_idx == 0) as u8 as f32, true));
        }

        assert!((output[0] - 1.0).abs() < 1e-6);
        assert!(output[1..].iter().all(|sample| sample.abs() < 1e-6));
    }

    #[test]
    fn bypassed_ir_preserves_dry_path_without_latency() {
        let mut stage = SpeakerStage::new(vec![0.25]).unwrap();
        let input = [0.4, -0.2, 0.1, 0.0];
        let output: Vec<_> = input
            .iter()
            .map(|sample| stage.process(*sample, false))
            .collect();

        assert_eq!(output, input);
    }

    #[test]
    fn decodes_pcm24_mono_ir() {
        let mut bytes = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        {
            let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
            writer.write_sample(0_i32).unwrap();
            writer.write_sample(4_194_304_i32).unwrap();
            writer.finalize().unwrap();
        }

        let ir = decode_wav_ir(&bytes.into_inner(), 48_000).unwrap();

        assert_eq!(ir.len(), 2);
        assert!(ir[0].abs() < 1.0e-6);
        assert!((ir[1] - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn decodes_float32_mono_ir() {
        let mut bytes = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        {
            let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
            writer.write_sample(0.25_f32).unwrap();
            writer.write_sample(-0.5_f32).unwrap();
            writer.finalize().unwrap();
        }

        let ir = decode_wav_ir(&bytes.into_inner(), 44_100).unwrap();

        assert_eq!(ir, vec![0.25, -0.5]);
    }

    #[test]
    fn resamples_ir_to_runtime_rate() {
        let mut bytes = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 4,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        {
            let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
            writer.write_sample(0.0_f32).unwrap();
            writer.write_sample(1.0_f32).unwrap();
            writer.write_sample(0.0_f32).unwrap();
            writer.write_sample(-1.0_f32).unwrap();
            writer.finalize().unwrap();
        }

        let ir = decode_wav_ir(&bytes.into_inner(), 8).unwrap();

        assert_eq!(ir.len(), 8);
        assert!((ir[1] - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn convolution_preserves_taps_across_partitions() {
        let mut ir = vec![0.0; CONVOLUTION_PARTITION_SIZE + 2];
        ir[0] = 1.0;
        ir[CONVOLUTION_PARTITION_SIZE] = 0.5;
        ir[CONVOLUTION_PARTITION_SIZE + 1] = -0.25;
        let mut stage = SpeakerStage::new(ir.clone()).unwrap();
        let output: Vec<_> = (0..CONVOLUTION_PARTITION_SIZE + ir.len())
            .map(|sample_idx| stage.process((sample_idx == 0) as u8 as f32, true))
            .collect();

        for (tap_idx, tap) in ir.iter().enumerate() {
            assert!((output[tap_idx] - tap).abs() < 1e-5);
        }
    }

    #[test]
    fn embedded_reference_ir_is_available_at_supported_rates() {
        assert!(!reference_speaker_ir(48_000).is_empty());
        assert!(!reference_speaker_ir(44_100).is_empty());
    }

    #[test]
    fn embedded_reference_ir_changes_the_signal() {
        let mut wet_stage = SpeakerStage::from_embedded_ir(48_000).unwrap();
        let mut bypassed_stage = SpeakerStage::from_embedded_ir(48_000).unwrap();
        let mut difference = 0.0;

        for sample_idx in 0..2048 {
            let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin();
            let wet = wet_stage.process(input, true);
            let dry = bypassed_stage.process(input, false);
            difference += (wet - dry).abs();
        }

        assert!(difference > 1.0, "difference={difference}");
    }
}
