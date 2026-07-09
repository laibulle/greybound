mod audio;

use greybound_ui::{preload_render_assets, GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use std::time::Duration;
use wasm_bindgen::prelude::*;

const UI_FONT: &[u8] = include_bytes!("../assets/fonts/Geist-Regular.ttf");
const METER_REFRESH_MS: u64 = 250;

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    let initial_size =
        browser_viewport_size().unwrap_or((DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32));
    let _ = WebApp::run(Settings {
        flags: initial_size,
        default_font: iced::Font::with_name("Geist"),
        window: iced::window::Settings {
            size: initial_size,
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            platform_specific: iced::window::PlatformSpecific {
                target: Some("greybound-web-root".to_string()),
            },
            ..iced::window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    });
}

fn browser_viewport_size() -> Option<(u32, u32)> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?.round().max(1.0) as u32;
    let height = window.inner_height().ok()?.as_f64()?.round().max(1.0) as u32;
    Some((width, height))
}

struct WebApp {
    ui: GreyboundUi,
}

impl Application for WebApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = (u32, u32);

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        preload_render_assets();
        let mut ui = GreyboundUi::default();
        ui.update(Message::WindowResized {
            width: flags.0,
            height: flags.1,
        });
        ui.update(Message::AudioDevicesChanged {
            inputs: vec!["Browser live input".to_string()],
            outputs: vec!["Browser output".to_string()],
            selected_input: Some("Browser live input".to_string()),
            selected_output: Some("Browser output".to_string()),
            status: "Requesting browser audio permission".to_string(),
        });
        ui.update(Message::AudioStatusChanged(
            "Requesting browser audio permission".to_string(),
        ));
        let snapshot = audio::WebAudioSnapshot::from_ui(&ui);
        (
            Self { ui },
            Command::batch([
                iced::font::load(UI_FONT).map(|_| {
                    Message::AudioStatusChanged("Starting WebAudio live input".to_string())
                }),
                start_audio_command(snapshot),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Web")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Message::MeterProbeTick(_) = message {
            let (input, output_left, output_right) = audio::meter_levels();
            self.ui.update(Message::MeterLevelsChanged {
                input,
                output_left,
                output_right,
            });
            return Command::none();
        }

        let restart_audio = should_restart_audio(&message);
        let update_audio_controls = should_update_audio_controls(&message);
        match message {
            Message::LoadWavRequested => {
                self.ui.update(Message::AudioStatusChanged(
                    "Browser WAV playback is not wired yet; live input is active.".to_string(),
                ));
            }
            Message::AudioInputSourceSelected(greybound_ui::AudioInputSource::WavFile) => {
                self.ui.update(Message::AudioInputSourceSelected(
                    greybound_ui::AudioInputSource::WavFile,
                ));
                audio::stop();
                self.ui.update(Message::AudioStatusChanged(
                    "Browser WAV playback is not wired yet; live input stopped.".to_string(),
                ));
            }
            Message::ShutdownRequested => {
                audio::stop();
            }
            Message::WindowResized { width, height } => {
                self.ui.update(Message::WindowResized { width, height });
            }
            message => {
                self.ui.update(message);
            }
        }

        if restart_audio {
            self.ui.update(Message::AudioStatusChanged(
                "Restarting WebAudio live input".to_string(),
            ));
            return start_audio_command(audio::WebAudioSnapshot::from_ui(&self.ui));
        }

        if update_audio_controls {
            audio::store_controls_from_ui(&self.ui);
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
                _ => None,
            }),
            iced::time::every(Duration::from_millis(METER_REFRESH_MS))
                .map(|_| Message::MeterProbeTick(std::time::Instant::now())),
        ])
    }
}

fn start_audio_command(snapshot: audio::WebAudioSnapshot) -> Command<Message> {
    Command::perform(audio::start(snapshot), |result| {
        Message::AudioStatusChanged(match result {
            Ok(status) => status,
            Err(error) => format!("WebAudio unavailable: {error}"),
        })
    })
}

fn should_restart_audio(message: &Message) -> bool {
    match message {
        Message::AudioInputSelected(_)
        | Message::AudioOutputSelected(_)
        | Message::AudioSampleRateSelected(_)
        | Message::AudioBufferSizeSelected(_)
        | Message::SelectAmpModel(_) => true,
        Message::AudioInputSourceSelected(source) => {
            *source == greybound_ui::AudioInputSource::LiveInput
        }
        _ => false,
    }
}

fn should_update_audio_controls(message: &Message) -> bool {
    matches!(
        message,
        Message::ToggleDoubler
            | Message::ToggleEq
            | Message::SetEqHpf(_)
            | Message::SetEqLpf(_)
            | Message::SetEqBand { .. }
            | Message::ToggleDeviceBypass(_)
            | Message::ToggleBypass(_)
            | Message::SetDeviceControl { .. }
            | Message::SetGlobalControl { .. }
            | Message::GainChanged(_)
            | Message::DriveChanged(_)
            | Message::BassChanged(_)
            | Message::TrebleChanged(_)
            | Message::CutChanged(_)
            | Message::PresenceChanged(_)
            | Message::SagChanged(_)
            | Message::MasterChanged(_)
    )
}
