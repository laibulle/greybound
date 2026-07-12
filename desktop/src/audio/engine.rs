use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::Stream;
use greybound_ui::{AudioInputSource, GreyboundUi};
use rtrb::RingBuffer;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::audio_devices::{
    device_name, select_config, selected_or_default_input, selected_or_default_output,
    stream_config,
};

use super::controls::SharedRuntimeControls;
use super::meter::MeterStats;
use super::recording::RecordingWorker;
use super::runtime::{post_amp_device_summary, pre_amp_device_summary, AudioRuntime};
use super::tuner::{TunerAnalysisWorker, TunerReading, TunerStats};
use super::wav::{FilePlaybackWorker, WavPlaybackBuffer};
use std::path::PathBuf;

pub(crate) struct LiveAudioEngine {
    _input_stream: Option<Stream>,
    _output_stream: Stream,
    _file_playback_worker: Option<FilePlaybackWorker>,
    _tuner_worker: TunerAnalysisWorker,
    recording_worker: Arc<std::sync::Mutex<Option<RecordingWorker>>>,
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
        let recording_worker = Arc::new(std::sync::Mutex::new(None::<RecordingWorker>));
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
        let output_recorder = recording_worker.clone();
        let output_name = output_device_name.clone();
        let amp_model = ui.runtime_amp_model_id();
        let app_profile = ui.app_profile;
        let mut runtime = AudioRuntime::new(
            sample_rate as f32,
            consumer,
            amp_model.as_str(),
            app_profile,
        )?;
        let output_stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _| {
                let mut recorder = output_recorder.try_lock().ok();
                for frame in data.chunks_exact_mut(output_channels) {
                    let (left, right) = runtime.process(&output_controls, &output_meters);
                    frame.fill(0.0);
                    frame[0] = left;
                    let metered_right = if output_channels > 1 { right } else { frame[0] };
                    if output_channels > 1 {
                        frame[1] = right;
                    }
                    output_meters.record_output(frame[0], metered_right);
                    if let Some(recorder) = recorder.as_deref_mut().and_then(Option::as_mut) {
                        recorder.record(frame[0], metered_right);
                    }
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
            recording_worker,
            controls,
            meters,
            tuner,
            input_device: input_device_name,
            output_device: output_device_name,
            minotaur_device: pre_amp_device_summary(app_profile),
            fx_devices: post_amp_device_summary(app_profile),
            amp_model,
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

    pub(crate) fn start_recording(&self, path: PathBuf) -> Result<()> {
        let worker = RecordingWorker::start(path, self.sample_rate, self.period_size)?;
        let mut recording_worker = self
            .recording_worker
            .lock()
            .map_err(|_| anyhow::anyhow!("recording worker lock poisoned"))?;
        *recording_worker = Some(worker);
        Ok(())
    }

    pub(crate) fn stop_recording(&self) -> Option<PathBuf> {
        self.recording_worker
            .lock()
            .ok()
            .and_then(|mut worker| worker.take().map(|worker| worker.path().to_path_buf()))
    }

    pub(crate) fn shutdown(self) {
        let _ = self._output_stream.pause();
        if let Some(input_stream) = &self._input_stream {
            let _ = input_stream.pause();
        }
        let _ = self.stop_recording();
        drop(self);
        thread::sleep(Duration::from_millis(50));
    }
}
