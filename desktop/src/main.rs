use iced::{Application, Command, Element, Settings, Subscription};
use voxbox_ui::{Message, VoxBoxUi};

fn main() -> iced::Result {
    Desktop::run(Settings::default())
}

struct Desktop {
    ui: VoxBoxUi,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Desktop {
                ui: VoxBoxUi::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("VoxBox Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.ui.update(message);
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
