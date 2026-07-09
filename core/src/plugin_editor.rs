use crate::GreyboundParams;
use nih_plug::prelude::{Editor, GuiContext};
use nih_plug_iced::widgets as nih_widgets;
use nih_plug_iced::{
    alignment, assets, executor, Color, Column, Command, Container, Element, IcedEditor, Length,
    Row, Space, Subscription, Text, WindowQueue, WindowSubs,
};
use std::sync::Arc;

pub fn create(params: Arc<GreyboundParams>) -> Option<Box<dyn Editor>> {
    nih_plug_iced::create_iced_editor::<GreyboundPluginEditor>(
        params.editor_state.clone(),
        params,
    )
}

struct GreyboundPluginEditor {
    params: Arc<GreyboundParams>,
    context: Arc<dyn GuiContext>,
    gain: nih_widgets::param_slider::State,
    bass: nih_widgets::param_slider::State,
    cut: nih_widgets::param_slider::State,
    tone: nih_widgets::param_slider::State,
    master: nih_widgets::param_slider::State,
    speaker_ir: nih_widgets::param_slider::State,
    overdrive: nih_widgets::param_slider::State,
    overdrive_gain: nih_widgets::param_slider::State,
    overdrive_treble: nih_widgets::param_slider::State,
    overdrive_output: nih_widgets::param_slider::State,
    fuzz: nih_widgets::param_slider::State,
    fuzz_sustain: nih_widgets::param_slider::State,
    fuzz_tone: nih_widgets::param_slider::State,
    fuzz_level: nih_widgets::param_slider::State,
}

#[derive(Debug, Clone)]
enum Message {
    ParamUpdate(nih_widgets::ParamMessage),
}

impl IcedEditor for GreyboundPluginEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = Arc<GreyboundParams>;

    fn new(
        params: Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        (
            Self {
                params,
                context,
                gain: Default::default(),
                bass: Default::default(),
                cut: Default::default(),
                tone: Default::default(),
                master: Default::default(),
                speaker_ir: Default::default(),
                overdrive: Default::default(),
                overdrive_gain: Default::default(),
                overdrive_treble: Default::default(),
                overdrive_output: Default::default(),
                fuzz: Default::default(),
                fuzz_sustain: Default::default(),
                fuzz_tone: Default::default(),
                fuzz_level: Default::default(),
            },
            Command::none(),
        )
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::ParamUpdate(message) => self.handle_param_message(message),
        }
        Command::none()
    }

    fn subscription(
        &self,
        _window_subs: &mut WindowSubs<Self::Message>,
    ) -> Subscription<Self::Message> {
        Subscription::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let header = Column::new()
            .spacing(2)
            .push(
                Text::new("GREYBOUND")
                    .font(assets::NOTO_SANS_BOLD)
                    .size(34)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(
                Text::new("greybox circuit plugin")
                    .font(assets::NOTO_SANS_LIGHT)
                    .size(14)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
            );

        let amp = panel(
            "Amp circuit",
            Column::new()
                .spacing(10)
                .push(param_row("Volume", &mut self.gain, &self.params.gain))
                .push(param_row("Bass", &mut self.bass, &self.params.bass))
                .push(param_row("Cut", &mut self.cut, &self.params.cut))
                .push(param_row("Treble", &mut self.tone, &self.params.tone))
                .push(param_row("Output", &mut self.master, &self.params.master))
                .push(param_row(
                    "Speaker IR",
                    &mut self.speaker_ir,
                    &self.params.speaker_ir,
                )),
        );

        let overdrive = panel(
            "Minotaur",
            Column::new()
                .spacing(10)
                .push(param_row(
                    "Enabled",
                    &mut self.overdrive,
                    &self.params.overdrive,
                ))
                .push(param_row(
                    "Gain",
                    &mut self.overdrive_gain,
                    &self.params.overdrive_gain,
                ))
                .push(param_row(
                    "Treble",
                    &mut self.overdrive_treble,
                    &self.params.overdrive_treble,
                ))
                .push(param_row(
                    "Output",
                    &mut self.overdrive_output,
                    &self.params.overdrive_output,
                )),
        );

        let fuzz = panel(
            "Muffin",
            Column::new()
                .spacing(10)
                .push(param_row("Enabled", &mut self.fuzz, &self.params.fuzz))
                .push(param_row(
                    "Sustain",
                    &mut self.fuzz_sustain,
                    &self.params.fuzz_sustain,
                ))
                .push(param_row("Tone", &mut self.fuzz_tone, &self.params.fuzz_tone))
                .push(param_row("Level", &mut self.fuzz_level, &self.params.fuzz_level)),
        );

        Container::new(
            Column::new()
                .spacing(18)
                .padding(24)
                .push(header)
                .push(
                    Row::new()
                        .spacing(16)
                        .push(Container::new(amp).width(Length::FillPortion(5)))
                        .push(
                            Column::new()
                                .spacing(16)
                                .push(overdrive)
                                .push(fuzz)
                                .width(Length::FillPortion(4)),
                        ),
                )
                .push(Space::with_height(Length::Units(6)))
                .push(
                    Text::new("Iced plugin editor. Parameters are owned by the host for automation and preset recall.")
                        .size(12)
                        .width(Length::Fill)
                        .horizontal_alignment(alignment::Horizontal::Center),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn background_color(&self) -> Color {
        Color::from_rgb(0.065, 0.072, 0.088)
    }
}

fn panel<'a>(
    title: &'static str,
    content: Column<'a, Message>,
) -> Container<'a, Message> {
    Container::new(
        Column::new()
            .spacing(12)
            .padding(18)
            .push(
                Text::new(title)
                    .font(assets::NOTO_SANS_BOLD)
                    .size(20)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
            )
            .push(content),
    )
    .width(Length::Fill)
}

fn param_row<'a, P: nih_plug::prelude::Param>(
    label: &'static str,
    state: &'a mut nih_widgets::param_slider::State,
    param: &'a P,
) -> Row<'a, Message> {
    Row::new()
        .spacing(12)
        .push(
            Text::new(label)
                .size(13)
                .width(Length::Units(90))
                .vertical_alignment(alignment::Vertical::Center),
        )
        .push(
            nih_widgets::ParamSlider::new(state, param)
                .width(Length::Fill)
                .map(Message::ParamUpdate),
        )
}
