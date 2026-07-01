use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, Stream, StreamConfig, SupportedStreamConfigRange,
};
use greybound::ir::SpeakerStage;
use greybound::{
    AmpControls, DeviceConfig, DeviceControls, DeviceSlotConfig, DeviceSlotControls,
    MinotaurControls, SignalChain, SignalChainConfig, SignalChainControls, SpringfieldControls,
};
use greybound_ui::{GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use rtrb::{Consumer, RingBuffer};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

const RMS_SCALE: f64 = 1_000_000_000.0;
const ASPECT_RATIO: f32 = DESIGN_WIDTH / DESIGN_HEIGHT;
const RESIZE_TOLERANCE_PX: u32 = 2;

fn main() -> iced::Result {
    Desktop::run(Settings {
        window: iced::window::Settings {
            size: (DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            ..iced::window::Settings::default()
        },
        exit_on_close_request: false,
        ..Settings::default()
    })
}

struct Desktop {
    ui: GreyboundUi,
    audio: Option<LiveAudioEngine>,
    audio_error: Option<String>,
    _audio_lab_mcp: Option<AudioLabMcpSidecar>,
    correcting_window_size: bool,
    shutting_down: bool,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut ui = GreyboundUi::default();
        refresh_audio_devices(&mut ui);
        let audio = LiveAudioEngine::start(&ui);
        let (audio, audio_error) = match audio {
            Ok(engine) => {
                ui.update(Message::AudioStatusChanged(engine.status()));
                (Some(engine), None)
            }
            Err(error) => {
                let error = error.to_string();
                ui.update(Message::AudioStatusChanged(error.clone()));
                (None, Some(error))
            }
        };
        (
            Desktop {
                ui,
                audio,
                audio_error,
                _audio_lab_mcp: AudioLabMcpSidecar::start().ok(),
                correcting_window_size: false,
                shutting_down: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if matches!(message, Message::ShutdownRequested) {
            self.shutting_down = true;
            self.audio = None;
            self._audio_lab_mcp = None;
            return iced::window::close();
        }

        if let Message::WindowResized { width, height } = message {
            self.ui.update(Message::WindowResized { width, height });
            if self.correcting_window_size {
                self.correcting_window_size = false;
                return Command::none();
            }

            if let Some((target_width, target_height)) = aspect_corrected_size(width, height) {
                self.correcting_window_size = true;
                return iced::window::resize(iced::Size::new(target_width, target_height));
            }

            return Command::none();
        }

        if let Message::MeterProbeTick(_) = message {
            if self.shutting_down {
                return Command::none();
            }
            if self.audio.is_none() {
                match LiveAudioEngine::start(&self.ui) {
                    Ok(engine) => {
                        self.ui.update(Message::AudioStatusChanged(engine.status()));
                        self.audio = Some(engine);
                        self.audio_error = None;
                    }
                    Err(error) => {
                        let error = error.to_string();
                        self.ui.update(Message::AudioStatusChanged(error.clone()));
                        self.audio_error = Some(error);
                    }
                }
            }

            if let Some(audio) = &self.audio {
                let (input, output_left, output_right) = audio.meter_levels();
                self.ui.update(Message::MeterLevelsChanged {
                    input,
                    output_left,
                    output_right,
                });
            }
            return Command::none();
        }

        let restart_audio = matches!(
            &message,
            Message::AudioInputSelected(_)
                | Message::AudioOutputSelected(_)
                | Message::AudioSampleRateSelected(_)
                | Message::AudioBufferSizeSelected(_)
        );
        self.ui.update(message);
        if restart_audio {
            self.audio = None;
            match LiveAudioEngine::start(&self.ui) {
                Ok(engine) => {
                    self.ui.update(Message::AudioStatusChanged(engine.status()));
                    self.audio = Some(engine);
                    self.audio_error = None;
                }
                Err(error) => {
                    let error = error.to_string();
                    self.ui.update(Message::AudioStatusChanged(error.clone()));
                    self.audio_error = Some(error);
                }
            }
        }
        if let Some(audio) = &self.audio {
            audio.controls.store_from_ui(&self.ui);
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::subscription::events_with(|event, _status| match event {
                iced::Event::Window(iced::window::Event::Resized { width, height }) => {
                    Some(Message::WindowResized { width, height })
                }
                iced::Event::Window(iced::window::Event::CloseRequested) => {
                    Some(Message::ShutdownRequested)
                }
                _ => None,
            }),
            iced::time::every(Duration::from_millis(33)).map(Message::MeterProbeTick),
        ])
    }
}

struct AudioLabMcpSidecar {
    child: Child,
    command: PathBuf,
    stderr_log: PathBuf,
}

impl AudioLabMcpSidecar {
    fn start() -> Result<Self> {
        let root = workspace_root()?;
        let command = find_audio_lab_mcp_binary()?;
        let log_dir = root.join("target/greybound-mcp");
        fs::create_dir_all(&log_dir)?;
        let stderr_log = log_dir.join("audio-lab-mcp.stderr.log");
        let stderr = File::create(&stderr_log)?;
        let child = ProcessCommand::new(&command)
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .spawn()
            .with_context(|| format!("could not start MCP sidecar '{}'", command.display()))?;
        eprintln!(
            "Greybound Audio Lab MCP started: {} (stderr {})",
            command.display(),
            stderr_log.display()
        );
        Ok(Self {
            child,
            command,
            stderr_log,
        })
    }
}

impl Drop for AudioLabMcpSidecar {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill() {
            eprintln!(
                "Greybound Audio Lab MCP shutdown warning for {}: {error}",
                self.command.display()
            );
        }
        let _ = self.child.wait();
        eprintln!(
            "Greybound Audio Lab MCP stopped; stderr log was {}",
            self.stderr_log.display()
        );
    }
}

struct LiveAudioEngine {
    _input_stream: Stream,
    _output_stream: Stream,
    controls: SharedRuntimeControls,
    meters: Arc<MeterStats>,
    input_device: String,
    output_device: String,
    sample_rate: u32,
    period_size: u32,
}

impl LiveAudioEngine {
    fn start(ui: &GreyboundUi) -> Result<Self> {
        let host = cpal::default_host();
        let sample_rate = ui.audio_settings.sample_rate;
        let period_size = ui.audio_settings.period_size;
        let input_device =
            selected_or_default_input(&host, ui.audio_settings.selected_input.as_deref())?;
        let output_device =
            selected_or_default_output(&host, ui.audio_settings.selected_output.as_deref())?;
        let input_device_name = device_name(&input_device);
        let output_device_name = device_name(&output_device);
        let output_range = select_config(
            output_device.supported_output_configs()?,
            sample_rate,
            period_size,
            "output",
        )?;
        let input_range = select_config(
            input_device.supported_input_configs()?,
            sample_rate,
            period_size,
            "input",
        )?;
        let output_config = stream_config(&output_range, sample_rate, period_size);
        let input_config = stream_config(&input_range, sample_rate, period_size);
        let input_channels = input_config.channels as usize;
        let output_channels = output_config.channels as usize;
        let (mut producer, consumer) = RingBuffer::<f32>::new(period_size as usize * 16);
        let meters = Arc::new(MeterStats::default());

        let input_name = input_device_name.clone();
        let input_stream = input_device.build_input_stream(
            &input_config,
            move |data: &[f32], _| {
                for frame in data.chunks_exact(input_channels) {
                    let sample = frame[0];
                    let _ = producer.push(sample);
                }
            },
            move |error| eprintln!("Greybound input stream error on {input_name}: {error}"),
            None,
        )?;

        let controls = SharedRuntimeControls::new(ui);
        let output_controls = controls.clone();
        let output_meters = meters.clone();
        let output_name = output_device_name.clone();
        let mut runtime = AudioRuntime::new(sample_rate as f32, consumer)?;
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

        input_stream.play()?;
        output_stream.play()?;

        Ok(Self {
            _input_stream: input_stream,
            _output_stream: output_stream,
            controls,
            meters,
            input_device: input_device_name,
            output_device: output_device_name,
            sample_rate,
            period_size,
        })
    }

    fn meter_levels(&self) -> (f32, f32, f32) {
        self.meters.snapshot_levels()
    }

    fn status(&self) -> String {
        format!(
            "Running: {} -> {}, {} Hz / {} samples",
            self.input_device, self.output_device, self.sample_rate, self.period_size
        )
    }
}

struct AudioRuntime {
    input: Consumer<f32>,
    chain: SignalChain,
    speaker: SpeakerStage,
    device_controls: Vec<DeviceSlotControls>,
    metronome: MetronomeGenerator,
}

impl AudioRuntime {
    fn new(sample_rate: f32, input: Consumer<f32>) -> Result<Self> {
        let mut config = SignalChainConfig::amp_only("nox30");
        config
            .pre_amp
            .push(DeviceSlotConfig::active(DeviceConfig::Minotaur));
        config
            .post_amp
            .push(DeviceSlotConfig::active(DeviceConfig::Springfield));

        Ok(Self {
            input,
            chain: SignalChain::new(sample_rate, config),
            speaker: SpeakerStage::from_embedded_ir(sample_rate as u32)?,
            device_controls: Vec::with_capacity(2),
            metronome: MetronomeGenerator::new(sample_rate),
        })
    }

    fn process(&mut self, controls: &SharedRuntimeControls, meters: &MeterStats) -> (f32, f32) {
        let guitar = self.process_guitar_mono(controls, meters);
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
        let chain_output = self.chain.process(
            input,
            SignalChainControls {
                amp: controls.load_amp_controls(),
                devices: &self.device_controls,
            },
        );
        let cab_mix = controls.cab_mix();
        let wet = self.speaker.process(chain_output, cab_mix > 0.0);
        (chain_output * (1.0 - cab_mix) + wet * cab_mix) * controls.output_gain()
    }
}

fn mix_final_output(guitar_mono: f32, metronome: (f32, f32)) -> (f32, f32) {
    (
        protect_dac(guitar_mono + metronome.0),
        protect_dac(guitar_mono + metronome.1),
    )
}

fn protect_dac(sample: f32) -> f32 {
    sample.clamp(-0.98, 0.98)
}

#[derive(Clone)]
struct SharedRuntimeControls {
    input_gain: Arc<AtomicU32>,
    output_gain: Arc<AtomicU32>,
    amp_volume: Arc<AtomicU32>,
    amp_bass: Arc<AtomicU32>,
    amp_treble: Arc<AtomicU32>,
    amp_cut: Arc<AtomicU32>,
    amp_sag: Arc<AtomicU32>,
    minotaur_bypassed: Arc<AtomicBool>,
    minotaur_gain: Arc<AtomicU32>,
    minotaur_treble: Arc<AtomicU32>,
    minotaur_output: Arc<AtomicU32>,
    springfield_bypassed: Arc<AtomicBool>,
    springfield_dwell: Arc<AtomicU32>,
    springfield_tone: Arc<AtomicU32>,
    springfield_mix: Arc<AtomicU32>,
    cab_enabled: Arc<AtomicBool>,
    cab_mix: Arc<AtomicU32>,
    metronome_enabled: Arc<AtomicBool>,
    metronome_bpm: Arc<AtomicU32>,
    metronome_volume: Arc<AtomicU32>,
    metronome_pan: Arc<AtomicU32>,
    metronome_beats_per_bar: Arc<AtomicU32>,
    metronome_rhythm_division: Arc<AtomicU32>,
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
            amp_sag: atomic_f32(0.0),
            minotaur_bypassed: Arc::new(AtomicBool::new(false)),
            minotaur_gain: atomic_f32(0.0),
            minotaur_treble: atomic_f32(0.0),
            minotaur_output: atomic_f32(0.0),
            springfield_bypassed: Arc::new(AtomicBool::new(false)),
            springfield_dwell: atomic_f32(0.0),
            springfield_tone: atomic_f32(0.0),
            springfield_mix: atomic_f32(0.0),
            cab_enabled: Arc::new(AtomicBool::new(true)),
            cab_mix: atomic_f32(1.0),
            metronome_enabled: Arc::new(AtomicBool::new(false)),
            metronome_bpm: atomic_f32(120.0),
            metronome_volume: atomic_f32(0.70),
            metronome_pan: atomic_f32(0.50),
            metronome_beats_per_bar: Arc::new(AtomicU32::new(4)),
            metronome_rhythm_division: Arc::new(AtomicU32::new(1)),
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
        store_f32(&self.amp_sag, ui.amp.master);
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
            output: 0.58,
            drive: 0.0,
            presence: 0.0,
            sag: load_f32(&self.amp_sag),
        }
    }

    fn input_gain(&self) -> f32 {
        load_f32(&self.input_gain)
    }

    fn output_gain(&self) -> f32 {
        load_f32(&self.output_gain)
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
        target.push(DeviceSlotControls {
            bypassed: self.minotaur_bypassed.load(Ordering::Relaxed),
            controls: DeviceControls::Minotaur(MinotaurControls {
                gain: load_f32(&self.minotaur_gain),
                treble: load_f32(&self.minotaur_treble),
                output: load_f32(&self.minotaur_output),
            }),
        });
        target.push(DeviceSlotControls {
            bypassed: self.springfield_bypassed.load(Ordering::Relaxed),
            controls: DeviceControls::Springfield(SpringfieldControls {
                dwell: load_f32(&self.springfield_dwell),
                tone: load_f32(&self.springfield_tone),
                mix: load_f32(&self.springfield_mix),
            }),
        });
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
        let transient = self.phase.sin().signum() * 0.55 + self.phase.sin() * 0.45;
        let sample = transient * self.envelope * controls.metronome_volume() * 0.24;
        let decay = (-1.0 / (self.sample_rate * 0.004)).exp();
        self.envelope *= decay;

        let pan = controls.metronome_pan();
        let left_gain = (pan * std::f32::consts::FRAC_PI_2).cos();
        let right_gain = (pan * std::f32::consts::FRAC_PI_2).sin();
        (
            (sample * left_gain).clamp(-0.24, 0.24),
            (sample * right_gain).clamp(-0.24, 0.24),
        )
    }

    fn trigger(&mut self, beats_per_bar: u32) {
        let accent = self.beat_index == 0;
        self.frequency = if accent { 1_700.0 } else { 1_100.0 };
        self.envelope = 1.0;
        self.phase = 0.0;
        self.beat_index = (self.beat_index + 1) % beats_per_bar.max(1);
    }

    fn samples_per_tick(&self, controls: &SharedRuntimeControls) -> f32 {
        let beats_per_second = controls.metronome_bpm() / 60.0;
        let ticks_per_beat = controls.metronome_rhythm_division() as f32;
        (self.sample_rate / (beats_per_second * ticks_per_beat)).max(1.0)
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

fn select_config(
    configs: impl Iterator<Item = SupportedStreamConfigRange>,
    sample_rate: u32,
    period_size: u32,
    direction: &str,
) -> Result<SupportedStreamConfigRange> {
    let rate = SampleRate(sample_rate);
    configs
        .filter(|config| config.sample_format() == SampleFormat::F32)
        .find(|config| {
            (config.min_sample_rate()..=config.max_sample_rate()).contains(&rate)
                && match config.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => {
                        (*min..=*max).contains(&period_size)
                    }
                    cpal::SupportedBufferSize::Unknown => true,
                }
        })
        .with_context(|| {
            format!(
                "no f32 {direction} configuration supports {sample_rate} Hz / {period_size} samples"
            )
        })
}

fn stream_config(
    range: &SupportedStreamConfigRange,
    sample_rate: u32,
    period_size: u32,
) -> StreamConfig {
    StreamConfig {
        channels: range.channels(),
        sample_rate: SampleRate(sample_rate),
        buffer_size: BufferSize::Fixed(period_size),
    }
}

fn device_name(device: &Device) -> String {
    device
        .name()
        .unwrap_or_else(|_| "unknown device".to_string())
}

fn refresh_audio_devices(ui: &mut GreyboundUi) {
    let host = cpal::default_host();
    let inputs = device_names(host.input_devices());
    let outputs = device_names(host.output_devices());
    let sample_rate = ui.audio_settings.sample_rate;
    let period_size = ui.audio_settings.period_size;
    let selected_input = ui
        .audio_settings
        .selected_input
        .clone()
        .filter(|selected| inputs.contains(selected))
        .or_else(|| preferred_input_name(&host, sample_rate, period_size));
    let selected_output = ui
        .audio_settings
        .selected_output
        .clone()
        .filter(|selected| outputs.contains(selected))
        .or_else(|| preferred_output_name(&host, sample_rate, period_size));

    ui.update(Message::AudioDevicesChanged {
        inputs,
        outputs,
        selected_input,
        selected_output,
        status: "Audio devices loaded".to_string(),
    });
}

fn device_names<I>(devices: std::result::Result<I, cpal::DevicesError>) -> Vec<String>
where
    I: Iterator<Item = Device>,
{
    let mut names = devices
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_else(|_| Vec::new());
    names.sort();
    names.dedup();
    names
}

fn preferred_input_name(host: &cpal::Host, sample_rate: u32, period_size: u32) -> Option<String> {
    let default_name = host
        .default_input_device()
        .map(|device| device_name(&device));
    preferred_device_name(
        host.input_devices().ok()?,
        default_name.as_deref(),
        sample_rate,
        period_size,
        AudioDirection::Input,
    )
    .or(default_name)
}

fn preferred_output_name(host: &cpal::Host, sample_rate: u32, period_size: u32) -> Option<String> {
    let default_name = host
        .default_output_device()
        .map(|device| device_name(&device));
    preferred_device_name(
        host.output_devices().ok()?,
        default_name.as_deref(),
        sample_rate,
        period_size,
        AudioDirection::Output,
    )
    .or(default_name)
}

fn preferred_device_name<I>(
    devices: I,
    default_name: Option<&str>,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> Option<String>
where
    I: Iterator<Item = Device>,
{
    devices
        .filter_map(|device| {
            let name = device.name().ok()?;
            let score = low_latency_device_score(
                &device,
                &name,
                default_name,
                sample_rate,
                period_size,
                direction,
            );
            (score > 0).then_some((score, name))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, name)| name)
}

fn low_latency_device_score(
    device: &Device,
    name: &str,
    default_name: Option<&str>,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> i32 {
    let Some(support) = low_latency_support(device, sample_rate, period_size, direction) else {
        return 0;
    };

    let mut score = match support {
        LowLatencySupport::Explicit => 120,
        LowLatencySupport::Unknown => 70,
    };

    if is_pro_audio_device_name(name) {
        score += 90;
    }
    if is_integrated_audio_name(name) {
        score -= 80;
    }
    if default_name.is_some_and(|default| default == name) {
        score += 5;
    }

    score
}

#[derive(Clone, Copy)]
enum LowLatencySupport {
    Explicit,
    Unknown,
}

fn low_latency_support(
    device: &Device,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> Option<LowLatencySupport> {
    match direction {
        AudioDirection::Input => {
            configs_low_latency_support(device.supported_input_configs(), sample_rate, period_size)
        }
        AudioDirection::Output => {
            configs_low_latency_support(device.supported_output_configs(), sample_rate, period_size)
        }
    }
}

fn configs_low_latency_support<I>(
    configs: std::result::Result<I, cpal::SupportedStreamConfigsError>,
    sample_rate: u32,
    period_size: u32,
) -> Option<LowLatencySupport>
where
    I: Iterator<Item = SupportedStreamConfigRange>,
{
    let rate = SampleRate(sample_rate);
    let mut has_unknown = false;

    for config in configs.ok()? {
        if config.sample_format() != SampleFormat::F32
            || !(config.min_sample_rate()..=config.max_sample_rate()).contains(&rate)
        {
            continue;
        }

        match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                if (*min..=*max).contains(&period_size) {
                    return Some(LowLatencySupport::Explicit);
                }
            }
            cpal::SupportedBufferSize::Unknown => {
                has_unknown = true;
            }
        }
    }

    has_unknown.then_some(LowLatencySupport::Unknown)
}

fn is_pro_audio_device_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "scarlett",
        "focusrite",
        "clarett",
        "apollo",
        "universal audio",
        "rme",
        "babyface",
        "fireface",
        "motu",
        "audient",
        "ssl",
        "antelope",
        "presonus",
        "quantum",
        "steinberg",
        "ur22",
        "ur44",
        "komplete",
        "axe-fx",
        "helix",
        "quad cortex",
        "volt",
        "behringer",
        "umc",
        "zoom",
        "tascam",
        "m-audio",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

fn is_integrated_audio_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "macbook",
        "built-in",
        "built in",
        "internal",
        "airpods",
        "display audio",
        "microphone",
        "speakers",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

#[derive(Clone, Copy)]
enum AudioDirection {
    Input,
    Output,
}

fn selected_or_default_input(host: &cpal::Host, selected: Option<&str>) -> Result<Device> {
    selected_device(host.input_devices()?, selected)
        .or_else(|| host.default_input_device())
        .context("missing default input device")
}

fn selected_or_default_output(host: &cpal::Host, selected: Option<&str>) -> Result<Device> {
    selected_device(host.output_devices()?, selected)
        .or_else(|| host.default_output_device())
        .context("missing default output device")
}

fn selected_device<I>(devices: I, selected: Option<&str>) -> Option<Device>
where
    I: Iterator<Item = Device>,
{
    let selected = selected?;
    devices
        .filter_map(|device| device.name().ok().map(|name| (device, name)))
        .find(|(_, name)| name == selected)
        .map(|(device, _)| device)
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

fn aspect_corrected_size(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let current_ratio = width as f32 / height as f32;
    if (current_ratio - ASPECT_RATIO).abs() < 0.003 {
        return None;
    }

    let width_from_height = (height as f32 * ASPECT_RATIO).round() as u32;
    let height_from_width = (width as f32 / ASPECT_RATIO).round() as u32;

    let width_delta = width.abs_diff(width_from_height);
    let height_delta = height.abs_diff(height_from_width);
    let (target_width, target_height) = if width_delta <= height_delta {
        (width_from_height, height)
    } else {
        (width, height_from_width)
    };

    if width.abs_diff(target_width) <= RESIZE_TOLERANCE_PX
        && height.abs_diff(target_height) <= RESIZE_TOLERANCE_PX
    {
        None
    } else {
        Some((target_width.max(1), target_height.max(1)))
    }
}

fn workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("knowledge").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().context("could not resolve workspace root")
}

fn find_audio_lab_mcp_binary() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?;
    let root = workspace_root()?;
    let candidates = [
        current_dir.join("greybound-audio-lab-mcp"),
        root.join("target/release/greybound-audio-lab-mcp"),
        root.join("target/debug/greybound-audio-lab-mcp"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "greybound-audio-lab-mcp binary not found; build it with cargo build --release -p greybound-audio-lab-mcp"
            )
        })
}
