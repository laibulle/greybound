use greybound_ui::{preload_render_assets, GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};
use wasm_bindgen::prelude::*;

const UI_FONT: &[u8] = include_bytes!("../assets/fonts/Geist-Regular.ttf");

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
        ui.update(Message::AudioStatusChanged(
            "Web alpha: shared iced UI running in the browser. Audio engine wiring is next."
                .to_string(),
        ));
        (
            Self { ui },
            iced::font::load(UI_FONT).map(|_| {
                Message::AudioStatusChanged(
                    "Web alpha: shared iced UI running in the browser.".to_string(),
                )
            }),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Web")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LoadWavRequested => {
                self.ui.update(Message::AudioStatusChanged(
                    "Browser WAV loading will be wired through Web Audio.".to_string(),
                ));
            }
            Message::ShutdownRequested => {}
            Message::WindowResized { width, height } => {
                self.ui.update(Message::WindowResized { width, height });
            }
            message => {
                self.ui.update(message);
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::subscription::events_with(|event, _status| match event {
            iced::Event::Window(iced::window::Event::Resized { width, height }) => {
                Some(Message::WindowResized { width, height })
            }
            _ => None,
        })
    }
}
