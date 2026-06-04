use iced::widget::{checkbox, column, container, progress_bar, row, slider, text};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone)]
pub enum Message {
    GainChanged(f32),
    BassChanged(f32),
    TrebleChanged(f32),
    CutChanged(f32),
    MasterChanged(f32),
    SpeakerIrToggled(bool),
}

#[derive(Debug, Clone)]
pub struct VoxBoxUi {
    pub gain: f32,
    pub bass: f32,
    pub treble: f32,
    pub cut: f32,
    pub master: f32,
    pub speaker_ir: bool,
}

impl Default for VoxBoxUi {
    fn default() -> Self {
        Self {
            gain: 0.55,
            bass: 0.50,
            treble: 0.60,
            cut: 0.35,
            master: 0.50,
            speaker_ir: false,
        }
    }
}

impl VoxBoxUi {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::GainChanged(value) => self.gain = value,
            Message::BassChanged(value) => self.bass = value,
            Message::TrebleChanged(value) => self.treble = value,
            Message::CutChanged(value) => self.cut = value,
            Message::MasterChanged(value) => self.master = value,
            Message::SpeakerIrToggled(value) => self.speaker_ir = value,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let control = |label: &str, value: f32, on_change: fn(f32) -> Message, unit: &str| {
            row![
                text(label).size(16).width(Length::Fixed(100.0)),
                slider(0.0..=1.0, value, on_change)
                    .step(0.001)
                    .width(Length::Fill),
                text(format!("{:.0}{}", value * 100.0, unit)).width(Length::Fixed(60.0)),
            ]
            .spacing(12)
            .align_items(Alignment::Center)
        };

        let input_level = (self.gain * 0.8 + self.bass * 0.1 + self.cut * 0.1).clamp(0.0, 1.0);
        let output_level = (self.master * 0.9 + self.treble * 0.1).clamp(0.0, 1.0);

        let meters = column![
            text("Level Meters").size(18),
            row![
                column![text("Input"), progress_bar(0.0..=1.0, input_level)].spacing(6),
                column![text("Output"), progress_bar(0.0..=1.0, output_level)].spacing(6),
            ]
            .spacing(16)
            .width(Length::Fill),
        ]
        .spacing(10)
        .padding(10);

        let layout = column![
            text("VoxBox Controls").size(28),
            control("Top Boost", self.gain, Message::GainChanged, "%"),
            control("Bass", self.bass, Message::BassChanged, "%"),
            control("Treble", self.treble, Message::TrebleChanged, "%"),
            control("Cut", self.cut, Message::CutChanged, "%"),
            control("Output", self.master, Message::MasterChanged, "%"),
            row![
                checkbox("Speaker IR", self.speaker_ir, Message::SpeakerIrToggled),
                text(if self.speaker_ir {
                    "enabled"
                } else {
                    "bypassed"
                })
                .size(16),
            ]
            .spacing(12)
            .align_items(Alignment::Center),
            meters,
        ]
        .spacing(18)
        .padding(20)
        .max_width(640);

        container(layout)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
