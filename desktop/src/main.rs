use greybound::{AmpControls, Greybound};
use greybound_ui::{GreyboundUi, Message, DESIGN_HEIGHT, DESIGN_WIDTH};
use iced::{Application, Command, Element, Settings, Subscription};

fn main() -> iced::Result {
    Desktop::run(Settings {
        window: iced::window::Settings {
            size: (DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
            min_size: Some(((DESIGN_WIDTH * 0.55) as u32, (DESIGN_HEIGHT * 0.55) as u32)),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct Desktop {
    ui: GreyboundUi,
    plugin: Greybound,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Desktop {
                ui: GreyboundUi::default(),
                plugin: Greybound::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Greybound Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.ui.update(message);
        {
            let device = &self.ui.amp;
            let controls = AmpControls {
                volume: device.gain,
                bass: device.bass,
                cut: device.cut,
                treble: device.treble,
                output: 0.58,
                drive: 0.0,
                presence: 0.0,
                sag: device.master,
            };
            self.plugin.set_ui_controls(controls);
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
