mod audio;
mod audio_devices;
mod mcp_sidecar;
mod windowing;

use audio::LiveAudioEngine;
use audio_devices::refresh_audio_devices;
use greybound_ui::{preload_render_assets, GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use mcp_sidecar::AudioLabMcpSidecar;
use std::io::Cursor;
use std::time::{Duration, Instant};
use windowing::aspect_corrected_size;

const TUNER_REFRESH_MS: u64 = 33;
const METER_REFRESH_MS: u64 = 250;
const AUDIO_RESTART_RETRY_MS: u64 = 2_000;
const METER_LEVEL_EPSILON: f32 = 0.015;

fn main() -> iced::Result {
    Desktop::run(Settings {
        window: iced::window::Settings {
            size: (DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            icon: app_icon(),
            ..iced::window::Settings::default()
        },
        antialiasing: true,
        exit_on_close_request: false,
        ..Settings::default()
    })
}

fn app_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/husky-app-icon.png");
    let decoder = png::Decoder::new(Cursor::new(bytes.as_slice()));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;

    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }

    let pixels = &buffer[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => pixels
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };

    iced::window::icon::from_rgba(rgba, info.width, info.height).ok()
}

struct Desktop {
    ui: GreyboundUi,
    audio: Option<LiveAudioEngine>,
    audio_error: Option<String>,
    _audio_lab_mcp: Option<AudioLabMcpSidecar>,
    correcting_window_size: bool,
    shutting_down: bool,
    last_audio_restart_attempt: Option<Instant>,
    last_meter_levels: (f32, f32, f32),
}

impl Desktop {
    fn stop_audio(&mut self) {
        if let Some(audio) = self.audio.take() {
            audio.shutdown();
        }
    }

    fn should_retry_audio_start(&self, now: Instant) -> bool {
        self.last_audio_restart_attempt
            .map(|last| now.duration_since(last) >= Duration::from_millis(AUDIO_RESTART_RETRY_MS))
            .unwrap_or(true)
    }

    fn should_publish_meter_levels(&self, levels: (f32, f32, f32)) -> bool {
        (levels.0 - self.last_meter_levels.0).abs() >= METER_LEVEL_EPSILON
            || (levels.1 - self.last_meter_levels.1).abs() >= METER_LEVEL_EPSILON
            || (levels.2 - self.last_meter_levels.2).abs() >= METER_LEVEL_EPSILON
    }
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        preload_render_assets();
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
                last_audio_restart_attempt: None,
                last_meter_levels: (0.0, 0.0, 0.0),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Free")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if matches!(message, Message::ShutdownRequested) {
            self.shutting_down = true;
            self.stop_audio();
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

        if let Message::MeterProbeTick(now) = message {
            if self.shutting_down {
                return Command::none();
            }
            if self.audio.is_none() && self.should_retry_audio_start(now) {
                self.last_audio_restart_attempt = Some(now);
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
                let levels = audio.meter_levels();
                if self.should_publish_meter_levels(levels) {
                    self.last_meter_levels = levels;
                    self.ui.update(Message::MeterLevelsChanged {
                        input: levels.0,
                        output_left: levels.1,
                        output_right: levels.2,
                    });
                }
            }
            return Command::none();
        }

        if let Message::TunerProbeTick(_) = message {
            if self.shutting_down {
                return Command::none();
            }

            if let Some(audio) = &self.audio {
                let tuner = audio.tuner_reading();
                self.ui.update(Message::TunerAnalysisChanged {
                    frequency_hz: tuner.frequency_hz,
                    cents: tuner.cents,
                    confidence: tuner.confidence,
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
                | Message::SelectAmpModel(_)
        );
        self.ui.update(message);
        if restart_audio {
            self.stop_audio();
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
            audio.store_controls_from_ui(&self.ui);
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
            iced::time::every(Duration::from_millis(METER_REFRESH_MS)).map(Message::MeterProbeTick),
            tuner_subscription(self.ui.tuner.open),
        ])
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        self.stop_audio();
        self._audio_lab_mcp = None;
    }
}

fn tuner_subscription(open: bool) -> Subscription<Message> {
    if open {
        iced::time::every(Duration::from_millis(TUNER_REFRESH_MS)).map(Message::TunerProbeTick)
    } else {
        Subscription::none()
    }
}
