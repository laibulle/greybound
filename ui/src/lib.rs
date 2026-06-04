use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, slider, text,
};
use iced::{Alignment, Background, Color, Element, Length, Vector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Amp,
    Pedal,
}

struct SkeuoButton;

impl button::StyleSheet for SkeuoButton {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.22, 0.20, 0.17))),
            border_radius: 16.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.80, 0.75, 0.62),
            shadow_offset: Vector::new(0.0, 3.0),
            text_color: Color::from_rgb(0.95, 0.92, 0.82),
            ..button::Appearance::default()
        }
    }
}

struct SkeuoContainer(Color);

impl container::StyleSheet for SkeuoContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(self.0)),
            border_radius: 18.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgb(0.70, 0.63, 0.51),
            ..container::Appearance::default()
        }
    }
}

fn skeuo_container(background: Color) -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(SkeuoContainer(background)))
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectDevice(usize),
    ToggleBypass(bool),
    GainChanged(f32),
    BassChanged(f32),
    TrebleChanged(f32),
    CutChanged(f32),
    MasterChanged(f32),
}

#[derive(Debug, Clone)]
pub struct DeviceState {
    pub name: String,
    pub kind: DeviceKind,
    pub bypassed: bool,
    pub gain: f32,
    pub bass: f32,
    pub treble: f32,
    pub cut: f32,
    pub master: f32,
}

impl DeviceState {
    pub fn new_amp(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: DeviceKind::Amp,
            bypassed: false,
            gain: 0.55,
            bass: 0.50,
            treble: 0.60,
            cut: 0.35,
            master: 0.50,
        }
    }

    pub fn new_pedal(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: DeviceKind::Pedal,
            bypassed: false,
            gain: 0.40,
            bass: 0.45,
            treble: 0.50,
            cut: 0.30,
            master: 0.70,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoxBoxUi {
    pub devices: Vec<DeviceState>,
    pub selected_index: usize,
}

impl Default for VoxBoxUi {
    fn default() -> Self {
        Self {
            devices: vec![
                DeviceState::new_amp("VoxBox Top Boost"),
                DeviceState::new_pedal("Crunch Pedal"),
                DeviceState::new_pedal("Reverb Pedal"),
            ],
            selected_index: 0,
        }
    }
}

impl VoxBoxUi {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectDevice(index) => {
                if index < self.devices.len() {
                    self.selected_index = index;
                }
            }
            Message::ToggleBypass(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.bypassed = value;
                }
            }
            Message::GainChanged(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.gain = value;
                }
            }
            Message::BassChanged(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.bass = value;
                }
            }
            Message::TrebleChanged(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.treble = value;
                }
            }
            Message::CutChanged(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.cut = value;
                }
            }
            Message::MasterChanged(value) => {
                if let Some(device) = self.devices.get_mut(self.selected_index) {
                    device.master = value;
                }
            }
        }
    }

    fn render_control(
        &self,
        label: &str,
        value: f32,
        on_change: impl Fn(f32) -> Message + 'static,
        unit: &str,
    ) -> iced::widget::Row<'_, Message> {
        row![
            text(label).size(16).width(Length::Fixed(100.0)),
            slider(0.0..=1.0, value, on_change)
                .step(0.001)
                .width(Length::Fill),
            text(format!("{:.0}{}", value * 100.0, unit)).width(Length::Fixed(60.0)),
        ]
        .spacing(12)
        .align_items(Alignment::Center)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = &self.devices[self.selected_index];

        let control_panel = container(
            column![
                text(&selected.name).size(26),
                text(match selected.kind {
                    DeviceKind::Amp => "Amp",
                    DeviceKind::Pedal => "Pedal",
                })
                .size(16),
                self.render_control("Gain", selected.gain, Message::GainChanged, "%"),
                self.render_control("Bass", selected.bass, Message::BassChanged, "%"),
                self.render_control("Treble", selected.treble, Message::TrebleChanged, "%"),
                self.render_control("Cut", selected.cut, Message::CutChanged, "%"),
                self.render_control("Output", selected.master, Message::MasterChanged, "%"),
                row![
                    checkbox("Bypass", selected.bypassed, Message::ToggleBypass),
                    text(if selected.bypassed {
                        "bypassed"
                    } else {
                        "active"
                    })
                    .size(16),
                ]
                .spacing(12)
                .align_items(Alignment::Center),
            ]
            .spacing(14)
            .padding(10),
        )
        .style(skeuo_container(Color::from_rgb(0.17, 0.15, 0.12)))
        .padding(20)
        .width(Length::FillPortion(3));

        let chain_buttons = self.devices.iter().enumerate().fold(
            column![text("Signal Chain").size(20)],
            |column, (index, device)| {
                column.push(
                    button(
                        text(&device.name)
                            .size(16)
                            .width(Length::Fill)
                            .horizontal_alignment(iced::alignment::Horizontal::Left),
                    )
                    .on_press(Message::SelectDevice(index))
                    .style(iced::theme::Button::custom(SkeuoButton))
                    .padding(12)
                    .width(Length::Fill),
                )
            },
        );

        let sidebar = container(scrollable(chain_buttons).width(Length::Fill))
            .padding(10)
            .style(skeuo_container(Color::from_rgb(0.15, 0.13, 0.10)))
            .width(Length::FillPortion(1));

        let meters = column![
            text("Level Meters").size(18),
            row![
                column![
                    text("Input"),
                    progress_bar(0.0..=1.0, selected.gain.clamp(0.0, 1.0)),
                ]
                .spacing(6),
                column![
                    text("Output"),
                    progress_bar(0.0..=1.0, selected.master.clamp(0.0, 1.0)),
                ]
                .spacing(6),
            ]
            .spacing(16)
            .width(Length::Fill),
        ]
        .spacing(10)
        .padding(10)
        .width(Length::FillPortion(1));

        let right_panel = column![control_panel, meters]
            .spacing(20)
            .width(Length::FillPortion(3));

        let layout = row![sidebar, right_panel].spacing(16).padding(20);

        container(layout)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
