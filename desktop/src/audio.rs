use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::Stream;
use greybound::ir::SpeakerStage;
use greybound::{
    AmpControls, DeviceConfig, DeviceControls, DeviceSlotConfig, DeviceSlotControls,
    MinotaurControls, SignalChain, SignalChainConfig, SignalChainControls, SpringfieldControls,
};
use greybound_ui::{AppProfile, AudioInputSource, GreyboundUi, RuntimeDeviceSection};
use rtrb::{Consumer, RingBuffer};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio_devices::{
    device_name, select_config, selected_or_default_input, selected_or_default_output,
    stream_config,
};

const RMS_SCALE: f64 = 1_000_000_000.0;

pub(crate) struct LiveAudioEngine {
    _input_stream: Option<Stream>,
    _output_stream: Stream,
    _file_playback_worker: Option<FilePlaybackWorker>,
    _tuner_worker: TunerAnalysisWorker,
    controls: SharedRuntimeControls,
    meters: Arc<MeterStats>,
    tuner: Arc<TunerStats>,
    input_device: String,
    output_device: String,
    minotaur_device: String,
    fx_devices: String,
    amp_model: String,
    sample_rate: u32,
    period_size: u32,
}

impl LiveAudioEngine {
    pub(crate) fn start(ui: &GreyboundUi) -> Result<Self> {
        let host = cpal::default_host();
        let sample_rate = ui.audio_settings.sample_rate;
        let period_size = ui.audio_settings.period_size;
        let output_device =
            selected_or_default_output(&host, ui.audio_settings.selected_output.as_deref())?;
        let output_device_name = device_name(&output_device);
        let output_range = select_config(
            output_device.supported_output_configs()?,
            sample_rate,
            period_size,
            "output",
        )?;
        let output_config = stream_config(&output_range, sample_rate, period_size);
        let output_channels = output_config.channels as usize;
        let (mut producer, consumer) = RingBuffer::<f32>::new(period_size as usize * 16);
        let (mut tuner_producer, tuner_consumer) =
            RingBuffer::<f32>::new((sample_rate as usize / 2).max(period_size as usize * 16));
        let meters = Arc::new(MeterStats::default());
        let tuner = Arc::new(TunerStats::default());
        let controls = SharedRuntimeControls::new(ui);
        let tuner_worker = TunerAnalysisWorker::start(
            sample_rate as f32,
            tuner_consumer,
            controls.clone(),
            tuner.clone(),
        );

        let (input_stream, file_playback_worker, input_device_name) = match ui
            .audio_settings
            .input_source
        {
            AudioInputSource::LiveInput => {
                let input_device =
                    selected_or_default_input(&host, ui.audio_settings.selected_input.as_deref())?;
                let input_device_name = device_name(&input_device);
                let input_range = select_config(
                    input_device.supported_input_configs()?,
                    sample_rate,
                    period_size,
                    "input",
                )?;
                let input_config = stream_config(&input_range, sample_rate, period_size);
                let input_channels = input_config.channels as usize;
                let input_name = input_device_name.clone();
                let input_stream = input_device.build_input_stream(
                    &input_config,
                    move |data: &[f32], _| {
                        for frame in data.chunks_exact(input_channels) {
                            let sample = frame[0];
                            let _ = producer.push(sample);
                            let _ = tuner_producer.push(sample);
                        }
                    },
                    move |error| eprintln!("Greybound input stream error on {input_name}: {error}"),
                    None,
                )?;
                (Some(input_stream), None, input_device_name)
            }
            AudioInputSource::WavFile => {
                let path = ui
                    .audio_settings
                    .wav_path
                    .as_ref()
                    .context("choose a WAV file before switching to WAV source")?;
                let file = WavPlaybackBuffer::load(path, sample_rate)?;
                let label = file.label.clone();
                let worker = FilePlaybackWorker::start(
                    file,
                    producer,
                    tuner_producer,
                    sample_rate,
                    period_size,
                );
                (None, Some(worker), format!("WAV {label}"))
            }
        };

        let output_controls = controls.clone();
        let output_meters = meters.clone();
        let output_name = output_device_name.clone();
        let amp_model = ui.amp_model_id();
        let app_profile = ui.app_profile;
        let mut runtime = AudioRuntime::new(sample_rate as f32, consumer, amp_model, app_profile)?;
        let output_stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _| {
                for frame in data.chunks_exact_mut(output_channels) {
                    let (left, right) = runtime.process(&output_controls, &output_meters);
                    frame.fill(0.0);
                    frame[0] = left;
                    let metered_right = if output_channels > 1 { right } else { frame[0] };
                    if output_channels > 1 {
                        frame[1] = right;
                    }
                    output_meters.record_output(frame[0], metered_right);
                }
            },
            move |error| eprintln!("Greybound output stream error on {output_name}: {error}"),
            None,
        )?;

        if let Some(input_stream) = &input_stream {
            input_stream.play()?;
        }
        output_stream.play()?;

        Ok(Self {
            _input_stream: input_stream,
            _output_stream: output_stream,
            _file_playback_worker: file_playback_worker,
            _tuner_worker: tuner_worker,
            controls,
            meters,
            tuner,
            input_device: input_device_name,
            output_device: output_device_name,
            minotaur_device: pre_amp_device_summary(app_profile),
            fx_devices: post_amp_device_summary(app_profile),
            amp_model: amp_model.to_string(),
            sample_rate,
            period_size,
        })
    }

    pub(crate) fn meter_levels(&self) -> (f32, f32, f32) {
        self.meters.snapshot_levels()
    }

    pub(crate) fn tuner_reading(&self) -> TunerReading {
        self.tuner.snapshot()
    }

    pub(crate) fn status(&self) -> String {
        format!(
            "Running: {} -> {}, {} Hz / {} samples, pedal {}, fx {}, amp {}",
            self.input_device,
            self.output_device,
            self.sample_rate,
            self.period_size,
            self.minotaur_device,
            self.fx_devices,
            self.amp_model
        )
    }

    pub(crate) fn store_controls_from_ui(&self, ui: &GreyboundUi) {
        self.controls.store_from_ui(ui);
    }

    pub(crate) fn shutdown(self) {
        let _ = self._output_stream.pause();
        if let Some(input_stream) = &self._input_stream {
            let _ = input_stream.pause();
        }
        drop(self);
        thread::sleep(Duration::from_millis(50));
    }
}

struct WavPlaybackBuffer {
    label: String,
    samples: Vec<f32>,
}

impl WavPlaybackBuffer {
    fn load(path: &Path, target_sample_rate: u32) -> Result<Self> {
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

struct FilePlaybackWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FilePlaybackWorker {
    fn start(
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

struct AudioRuntime {
    input: Consumer<f32>,
    chain: SignalChain,
    speaker: SpeakerStage,
    device_controls: Vec<DeviceSlotControls>,
    eq: GraphicEqProcessor,
    metronome: MetronomeGenerator,
    doubler: DoublerProcessor,
}

impl AudioRuntime {
    fn new(
        sample_rate: f32,
        input: Consumer<f32>,
        amp_model: &str,
        app_profile: AppProfile,
    ) -> Result<Self> {
        let mut config = SignalChainConfig::amp_only(amp_model);
        for slot in app_profile.runtime_devices {
            let device = if slot.bypassed {
                DeviceSlotConfig::bypassed(slot.config)
            } else {
                DeviceSlotConfig::active(slot.config)
            };
            match slot.section {
                RuntimeDeviceSection::PreAmp => config.pre_amp.push(device),
                RuntimeDeviceSection::PostAmp => config.post_amp.push(device),
            }
        }

        Ok(Self {
            input,
            chain: SignalChain::new(sample_rate, config),
            speaker: reference_speaker_or_bypass(sample_rate as u32),
            device_controls: Vec::with_capacity(2),
            eq: GraphicEqProcessor::new(sample_rate),
            metronome: MetronomeGenerator::new(sample_rate),
            doubler: DoublerProcessor::new(sample_rate),
        })
    }

    fn process(&mut self, controls: &SharedRuntimeControls, meters: &MeterStats) -> (f32, f32) {
        let guitar = self.process_guitar_mono(controls, meters);
        if controls.tuner_muted() {
            return (0.0, 0.0);
        }
        let guitar = self.doubler.process(guitar, controls);
        let metronome = self.metronome.process(controls);
        mix_final_output(guitar, metronome)
    }

    fn process_guitar_mono(
        &mut self,
        controls: &SharedRuntimeControls,
        meters: &MeterStats,
    ) -> f32 {
        let input = self.input.pop().unwrap_or(0.0) * controls.input_gain();
        meters.record_input(input);
        controls.load_device_controls_into(&mut self.device_controls);
        let chain_output = self.chain.process_with_amp_enabled(
            input,
            SignalChainControls {
                amp: controls.load_amp_controls(),
                devices: &self.device_controls,
            },
            controls.amp_enabled(),
        );
        let cab_mix = controls.cab_mix();
        let wet = self.speaker.process(chain_output, cab_mix > 0.0);
        let cabbed = chain_output * (1.0 - cab_mix) + wet * cab_mix;
        self.eq.process(cabbed, controls) * controls.output_gain()
    }
}

fn reference_speaker_or_bypass(sample_rate: u32) -> SpeakerStage {
    SpeakerStage::from_embedded_ir(sample_rate).unwrap_or_else(|error| {
        #[cfg(debug_assertions)]
        eprintln!("Greybound speaker IR disabled: {error:#}");
        #[cfg(not(debug_assertions))]
        let _ = error;
        SpeakerStage::bypassed()
    })
}

fn mix_final_output(guitar: (f32, f32), metronome: (f32, f32)) -> (f32, f32) {
    (
        protect_dac(guitar.0 + metronome.0),
        protect_dac(guitar.1 + metronome.1),
    )
}

fn protect_dac(sample: f32) -> f32 {
    sample.clamp(-0.98, 0.98)
}

fn pre_amp_device_summary(app_profile: AppProfile) -> String {
    runtime_device_summary(app_profile, RuntimeDeviceSection::PreAmp)
}

fn post_amp_device_summary(app_profile: AppProfile) -> String {
    runtime_device_summary(app_profile, RuntimeDeviceSection::PostAmp)
}

fn runtime_device_summary(app_profile: AppProfile, section: RuntimeDeviceSection) -> String {
    let labels: Vec<String> = app_profile
        .runtime_devices
        .iter()
        .filter(|slot| slot.section == section)
        .map(|slot| {
            let label = slot.config.model_descriptor().label;
            if slot.bypassed {
                format!("{label} bypassed")
            } else {
                label.to_string()
            }
        })
        .collect();

    if labels.is_empty() {
        "none".to_string()
    } else {
        labels.join(" + ")
    }
}

#[derive(Clone)]
struct SharedRuntimeControls {
    input_gain: Arc<AtomicU32>,
    output_gain: Arc<AtomicU32>,
    amp_volume: Arc<AtomicU32>,
    amp_bass: Arc<AtomicU32>,
    amp_treble: Arc<AtomicU32>,
    amp_cut: Arc<AtomicU32>,
    amp_output: Arc<AtomicU32>,
    amp_drive: Arc<AtomicU32>,
    amp_presence: Arc<AtomicU32>,
    amp_sag: Arc<AtomicU32>,
    amp_bypassed: Arc<AtomicBool>,
    minotaur_bypassed: Arc<AtomicBool>,
    minotaur_gain: Arc<AtomicU32>,
    minotaur_treble: Arc<AtomicU32>,
    minotaur_output: Arc<AtomicU32>,
    springfield_bypassed: Arc<AtomicBool>,
    springfield_dwell: Arc<AtomicU32>,
    springfield_tone: Arc<AtomicU32>,
    springfield_mix: Arc<AtomicU32>,
    runtime_devices: &'static [greybound_ui::RuntimeDeviceSlot],
    cab_enabled: Arc<AtomicBool>,
    cab_mix: Arc<AtomicU32>,
    metronome_enabled: Arc<AtomicBool>,
    metronome_bpm: Arc<AtomicU32>,
    metronome_volume: Arc<AtomicU32>,
    metronome_pan: Arc<AtomicU32>,
    metronome_beats_per_bar: Arc<AtomicU32>,
    metronome_rhythm_division: Arc<AtomicU32>,
    eq_enabled: Arc<AtomicBool>,
    eq_hpf_hz: Arc<AtomicU32>,
    eq_lpf_hz: Arc<AtomicU32>,
    eq_band_gains_db: [Arc<AtomicU32>; greybound_ui::EQ_BAND_COUNT],
    doubler_enabled: Arc<AtomicBool>,
    doubler_delay_ms: Arc<AtomicU32>,
    tuner_live: Arc<AtomicBool>,
    tuner_muted: Arc<AtomicBool>,
    tuner_reference_hz: Arc<AtomicU32>,
}

impl SharedRuntimeControls {
    fn new(ui: &GreyboundUi) -> Self {
        let controls = Self {
            input_gain: atomic_f32(1.0),
            output_gain: atomic_f32(1.0),
            amp_volume: atomic_f32(0.0),
            amp_bass: atomic_f32(0.0),
            amp_treble: atomic_f32(0.0),
            amp_cut: atomic_f32(0.0),
            amp_output: atomic_f32(0.58),
            amp_drive: atomic_f32(0.0),
            amp_presence: atomic_f32(0.0),
            amp_sag: atomic_f32(0.0),
            amp_bypassed: Arc::new(AtomicBool::new(false)),
            minotaur_bypassed: Arc::new(AtomicBool::new(false)),
            minotaur_gain: atomic_f32(0.0),
            minotaur_treble: atomic_f32(0.0),
            minotaur_output: atomic_f32(0.0),
            springfield_bypassed: Arc::new(AtomicBool::new(true)),
            springfield_dwell: atomic_f32(0.48),
            springfield_tone: atomic_f32(0.58),
            springfield_mix: atomic_f32(0.26),
            runtime_devices: ui.app_profile.runtime_devices,
            cab_enabled: Arc::new(AtomicBool::new(true)),
            cab_mix: atomic_f32(1.0),
            metronome_enabled: Arc::new(AtomicBool::new(false)),
            metronome_bpm: atomic_f32(120.0),
            metronome_volume: atomic_f32(0.70),
            metronome_pan: atomic_f32(0.50),
            metronome_beats_per_bar: Arc::new(AtomicU32::new(4)),
            metronome_rhythm_division: Arc::new(AtomicU32::new(1)),
            eq_enabled: Arc::new(AtomicBool::new(true)),
            eq_hpf_hz: atomic_f32(0.0),
            eq_lpf_hz: atomic_f32(0.0),
            eq_band_gains_db: std::array::from_fn(|_| atomic_f32(0.0)),
            doubler_enabled: Arc::new(AtomicBool::new(false)),
            doubler_delay_ms: atomic_f32(7.15),
            tuner_live: Arc::new(AtomicBool::new(false)),
            tuner_muted: Arc::new(AtomicBool::new(false)),
            tuner_reference_hz: atomic_f32(440.0),
        };
        controls.store_from_ui(ui);
        controls
    }

    fn store_from_ui(&self, ui: &GreyboundUi) {
        store_f32(
            &self.input_gain,
            greybound_ui::normalized_gain(ui.input_gain, -24.0, 24.0),
        );
        store_f32(
            &self.output_gain,
            greybound_ui::normalized_gain(ui.output_gain, -24.0, 6.0),
        );
        store_f32(&self.amp_volume, ui.amp.gain);
        store_f32(&self.amp_bass, ui.amp.bass);
        store_f32(&self.amp_treble, ui.amp.treble);
        store_f32(&self.amp_cut, ui.amp.cut);
        self.amp_bypassed.store(ui.amp.bypassed, Ordering::Relaxed);
        store_f32(&self.amp_output, 0.58);
        store_f32(&self.amp_drive, ui.amp.drive);
        store_f32(&self.amp_presence, ui.amp.presence);
        store_f32(&self.amp_sag, ui.amp.sag);
        self.cab_enabled
            .store(!ui.cab.bypassed && ui.cab.master > 0.0, Ordering::Relaxed);
        store_f32(&self.cab_mix, ui.cab.master.clamp(0.0, 1.0));
        self.metronome_enabled
            .store(ui.metronome.enabled, Ordering::Relaxed);
        store_f32(&self.metronome_bpm, ui.metronome.bpm.clamp(30.0, 260.0));
        store_f32(&self.metronome_volume, ui.metronome.volume.clamp(0.0, 1.0));
        store_f32(&self.metronome_pan, ui.metronome.pan.clamp(0.0, 1.0));
        self.metronome_beats_per_bar.store(4, Ordering::Relaxed);
        self.metronome_rhythm_division.store(1, Ordering::Relaxed);
        self.eq_enabled.store(ui.eq.enabled, Ordering::Relaxed);
        store_f32(
            &self.eq_hpf_hz,
            greybound_ui::eq_hpf_frequency_hz(ui.eq.hpf).unwrap_or(0.0),
        );
        store_f32(
            &self.eq_lpf_hz,
            greybound_ui::eq_lpf_frequency_hz(ui.eq.lpf).unwrap_or(0.0),
        );
        for (target, value) in self.eq_band_gains_db.iter().zip(ui.eq.bands.iter()) {
            store_f32(target, greybound_ui::eq_band_gain_db(*value));
        }
        self.doubler_enabled
            .store(ui.doubler.enabled, Ordering::Relaxed);
        store_f32(&self.doubler_delay_ms, ui.doubler.delay_ms.clamp(0.0, 20.0));
        self.tuner_live
            .store(ui.tuner.open && ui.tuner.live, Ordering::Relaxed);
        self.tuner_muted.store(ui.tuner.muted, Ordering::Relaxed);
        store_f32(
            &self.tuner_reference_hz,
            ui.tuner.reference_hz.clamp(415.0, 466.0),
        );

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Minotaur)
        {
            self.minotaur_bypassed
                .store(device.bypassed, Ordering::Relaxed);
            store_f32(&self.minotaur_gain, device.gain);
            store_f32(&self.minotaur_treble, device.treble);
            store_f32(&self.minotaur_output, device.master);
        }

        if let Some(device) = ui
            .devices
            .iter()
            .find(|device| device.model == greybound_ui::DeviceModel::Springfield)
        {
            self.springfield_bypassed
                .store(device.bypassed, Ordering::Relaxed);
            store_f32(&self.springfield_dwell, device.gain);
            store_f32(&self.springfield_tone, device.treble);
            store_f32(&self.springfield_mix, device.master);
        }
    }

    fn load_amp_controls(&self) -> AmpControls {
        AmpControls {
            volume: load_f32(&self.amp_volume),
            bass: load_f32(&self.amp_bass),
            treble: load_f32(&self.amp_treble),
            cut: load_f32(&self.amp_cut),
            output: load_f32(&self.amp_output),
            drive: load_f32(&self.amp_drive),
            presence: load_f32(&self.amp_presence),
            sag: load_f32(&self.amp_sag),
        }
    }

    fn input_gain(&self) -> f32 {
        load_f32(&self.input_gain)
    }

    fn output_gain(&self) -> f32 {
        load_f32(&self.output_gain)
    }

    fn amp_enabled(&self) -> bool {
        !self.amp_bypassed.load(Ordering::Relaxed)
    }

    fn cab_mix(&self) -> f32 {
        if self.cab_enabled.load(Ordering::Relaxed) {
            load_f32(&self.cab_mix).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn load_device_controls_into(&self, target: &mut Vec<DeviceSlotControls>) {
        target.clear();
        for slot in self.runtime_devices {
            match slot.config {
                DeviceConfig::Minotaur => target.push(DeviceSlotControls {
                    bypassed: self.minotaur_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Minotaur(MinotaurControls {
                        gain: load_f32(&self.minotaur_gain),
                        treble: load_f32(&self.minotaur_treble),
                        output: load_f32(&self.minotaur_output),
                    }),
                }),
                DeviceConfig::Springfield => target.push(DeviceSlotControls {
                    bypassed: self.springfield_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Springfield(SpringfieldControls {
                        dwell: load_f32(&self.springfield_dwell),
                        tone: load_f32(&self.springfield_tone),
                        mix: load_f32(&self.springfield_mix),
                    }),
                }),
                _ => {}
            }
        }
    }

    fn metronome_enabled(&self) -> bool {
        self.metronome_enabled.load(Ordering::Relaxed)
    }

    fn metronome_bpm(&self) -> f32 {
        load_f32(&self.metronome_bpm).clamp(30.0, 260.0)
    }

    fn metronome_volume(&self) -> f32 {
        load_f32(&self.metronome_volume).clamp(0.0, 1.0)
    }

    fn metronome_pan(&self) -> f32 {
        load_f32(&self.metronome_pan).clamp(0.0, 1.0)
    }

    fn metronome_beats_per_bar(&self) -> u32 {
        self.metronome_beats_per_bar
            .load(Ordering::Relaxed)
            .clamp(1, 16)
    }

    fn metronome_rhythm_division(&self) -> u32 {
        self.metronome_rhythm_division
            .load(Ordering::Relaxed)
            .clamp(1, 16)
    }

    fn eq_enabled(&self) -> bool {
        self.eq_enabled.load(Ordering::Relaxed)
    }

    fn eq_band_gain_db(&self, index: usize) -> f32 {
        self.eq_band_gains_db
            .get(index)
            .map(|slot| load_f32(slot.as_ref()))
            .unwrap_or(0.0)
            .clamp(-greybound_ui::EQ_MAX_GAIN_DB, greybound_ui::EQ_MAX_GAIN_DB)
    }

    fn eq_hpf_hz(&self) -> Option<f32> {
        let frequency = load_f32(&self.eq_hpf_hz);
        (frequency > 0.0).then_some(frequency)
    }

    fn eq_lpf_hz(&self) -> Option<f32> {
        let frequency = load_f32(&self.eq_lpf_hz);
        (frequency > 0.0).then_some(frequency)
    }

    fn doubler_enabled(&self) -> bool {
        self.doubler_enabled.load(Ordering::Relaxed)
    }

    fn doubler_delay_ms(&self) -> f32 {
        load_f32(&self.doubler_delay_ms).clamp(0.0, 20.0)
    }

    fn tuner_live(&self) -> bool {
        self.tuner_live.load(Ordering::Relaxed)
    }

    fn tuner_muted(&self) -> bool {
        self.tuner_muted.load(Ordering::Relaxed)
    }

    fn tuner_reference_hz(&self) -> f32 {
        load_f32(&self.tuner_reference_hz).clamp(415.0, 466.0)
    }
}

const EQ_BAND_FREQUENCIES_HZ: [f32; greybound_ui::EQ_BAND_COUNT] = [
    65.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

struct GraphicEqProcessor {
    sample_rate: f32,
    highpass: PeakingBiquad,
    lowpass: PeakingBiquad,
    last_hpf_hz: f32,
    last_lpf_hz: f32,
    bands: [PeakingBiquad; greybound_ui::EQ_BAND_COUNT],
    last_gains_db: [f32; greybound_ui::EQ_BAND_COUNT],
}

impl GraphicEqProcessor {
    fn new(sample_rate: f32) -> Self {
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

    fn process(&mut self, input: f32, controls: &SharedRuntimeControls) -> f32 {
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

struct DoublerProcessor {
    sample_rate: f32,
    buffer: Vec<f32>,
    write_index: usize,
    modulation_phase: f32,
}

impl DoublerProcessor {
    fn new(sample_rate: f32) -> Self {
        let capacity = (sample_rate * 0.030).ceil() as usize + 4;
        Self {
            sample_rate,
            buffer: vec![0.0; capacity.max(8)],
            write_index: 0,
            modulation_phase: 0.0,
        }
    }

    fn process(&mut self, input: f32, controls: &SharedRuntimeControls) -> (f32, f32) {
        let delayed = self.read_delayed_sample(controls.doubler_delay_ms());
        self.buffer[self.write_index] = input;
        self.write_index = (self.write_index + 1) % self.buffer.len();
        self.advance_modulation();

        if !controls.doubler_enabled() {
            return (input, input);
        }

        let left = input * 0.92 + delayed * 0.08;
        let right = delayed * 0.92 + input * 0.08;
        (left, right)
    }

    fn read_delayed_sample(&self, delay_ms: f32) -> f32 {
        let modulation_depth_ms = (delay_ms * 0.04).min(0.35);
        let modulated_delay_ms = delay_ms + self.modulation_phase.sin() * modulation_depth_ms;
        let delay_samples = (modulated_delay_ms.max(0.0) * self.sample_rate / 1_000.0)
            .min((self.buffer.len() - 2) as f32);
        let read_position = self.write_index as f32 - delay_samples + self.buffer.len() as f32;
        let base_index = read_position.floor() as usize % self.buffer.len();
        let next_index = (base_index + 1) % self.buffer.len();
        let fraction = read_position.fract();
        self.buffer[base_index] * (1.0 - fraction) + self.buffer[next_index] * fraction
    }

    fn advance_modulation(&mut self) {
        let increment = std::f32::consts::TAU * 0.19 / self.sample_rate;
        self.modulation_phase = (self.modulation_phase + increment) % std::f32::consts::TAU;
    }
}

struct MetronomeGenerator {
    sample_rate: f32,
    samples_until_tick: f32,
    envelope: f32,
    phase: f32,
    frequency: f32,
    beat_index: u32,
    was_enabled: bool,
}

impl MetronomeGenerator {
    fn new(sample_rate: f32) -> Self {
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

    fn process(&mut self, controls: &SharedRuntimeControls) -> (f32, f32) {
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

struct TunerAnalysisWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TunerAnalysisWorker {
    fn start(
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
struct TunerStats {
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

    fn snapshot(&self) -> TunerReading {
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

#[derive(Default)]
struct MeterStats {
    input_sum_squares: AtomicU64,
    input_count: AtomicU64,
    output_left_sum_squares: AtomicU64,
    output_right_sum_squares: AtomicU64,
    output_left_count: AtomicU64,
    output_right_count: AtomicU64,
}

impl MeterStats {
    fn record_input(&self, sample: f32) {
        self.input_sum_squares
            .fetch_add(square(sample), Ordering::Relaxed);
        self.input_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_output(&self, left: f32, right: f32) {
        self.output_left_sum_squares
            .fetch_add(square(left), Ordering::Relaxed);
        self.output_right_sum_squares
            .fetch_add(square(right), Ordering::Relaxed);
        self.output_left_count.fetch_add(1, Ordering::Relaxed);
        self.output_right_count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot_levels(&self) -> (f32, f32, f32) {
        let input_level = meter_from_accumulators(&self.input_sum_squares, &self.input_count);
        let output_left_level =
            meter_from_accumulators(&self.output_left_sum_squares, &self.output_left_count);
        let output_right_level =
            meter_from_accumulators(&self.output_right_sum_squares, &self.output_right_count);
        (input_level, output_left_level, output_right_level)
    }
}

fn atomic_f32(value: f32) -> Arc<AtomicU32> {
    Arc::new(AtomicU32::new(value.to_bits()))
}

fn store_f32(slot: &AtomicU32, value: f32) {
    slot.store(value.to_bits(), Ordering::Relaxed);
}

fn load_f32(slot: &AtomicU32) -> f32 {
    f32::from_bits(slot.load(Ordering::Relaxed))
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

#[cfg(test)]
mod tests;
