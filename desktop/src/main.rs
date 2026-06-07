use boutique59::{AmpControls, Boutique59};
use boutique59_ui::{Boutique59Ui, Message};
use iced::{Application, Command, Element, Settings, Subscription};

fn main() -> iced::Result {
    Desktop::run(Settings::default())
}

struct Desktop {
    ui: Boutique59Ui,
    plugin: Boutique59,
}

impl Application for Desktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Desktop {
                ui: Boutique59Ui::default(),
                plugin: Boutique59::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Boutique59 Desktop")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.ui.update(message);
        // Map current selected device's UI state to plugin controls and push to plugin
        if let Some(device) = self.ui.devices.get(self.ui.selected_index) {
            let controls = AmpControls {
                volume: device.gain,
                bass: device.bass,
                cut: device.cut,
                treble: device.treble,
                output: device.master,
                drive: 0.0,
                presence: 0.0,
                sag: 0.0,
            };
            self.plugin.set_ui_controls(controls);
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.ui.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
