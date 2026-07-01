use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, row, slider, text};
use iced::{mouse, Alignment, Background, Color, Element, Length, Point, Rectangle, Size, Vector};

const INK: Color = Color::from_rgb(0.09, 0.12, 0.24);
const PANEL: Color = Color::from_rgb(0.72, 0.78, 0.91);
const PEDAL_CREAM: Color = Color::from_rgb(0.84, 0.80, 0.72);
const PEDAL_PEACH: Color = Color::from_rgb(0.77, 0.56, 0.45);
const PEDAL_SAGE: Color = Color::from_rgb(0.67, 0.62, 0.49);
const TEAL: Color = Color::from_rgb(0.35, 0.56, 0.57);
const GOLD: Color = Color::from_rgb(0.76, 0.61, 0.35);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Amp,
    Pedal,
    Cab,
}

struct ChromeButton;

impl button::StyleSheet for ChromeButton {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.90, 0.93, 1.0, 0.18))),
            border_radius: 10.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.14, 0.18, 0.32, 0.22),
            shadow_offset: Vector::new(0.0, 2.0),
            text_color: INK,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.32))),
            ..self.active(style)
        }
    }
}

struct GhostContainer(Color);

impl container::StyleSheet for GhostContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(INK),
            background: Some(Background::Color(self.0)),
            border_radius: 14.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.12, 0.16, 0.28, 0.12),
            ..container::Appearance::default()
        }
    }
}

fn ghost_container(background: Color) -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(GhostContainer(background)))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceModel {
    Minotaur,
    Nox30,
    Springfield,
    CabIr,
}

impl DeviceModel {
    fn title(self) -> &'static str {
        match self {
            Self::Minotaur => "Minotaur",
            Self::Nox30 => "Nox30",
            Self::Springfield => "Springfield",
            Self::CabIr => "Celestion IR",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::Minotaur => "Klon-style greybox overdrive",
            Self::Nox30 => "JMI-era Top Boost greybox amp",
            Self::Springfield => "Spring tank greybox/IR hybrid",
            Self::CabIr => "External cabinet impulse response",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceState {
    pub name: String,
    pub kind: DeviceKind,
    pub model: DeviceModel,
    pub bypassed: bool,
    pub gain: f32,
    pub bass: f32,
    pub treble: f32,
    pub cut: f32,
    pub master: f32,
}

impl DeviceState {
    pub fn minotaur() -> Self {
        Self {
            name: "MINOTAUR".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Minotaur,
            bypassed: false,
            gain: 0.42,
            bass: 0.0,
            treble: 0.70,
            cut: 0.0,
            master: 0.42,
        }
    }

    pub fn nox30() -> Self {
        Self {
            name: "NOX30".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::Nox30,
            bypassed: false,
            gain: 0.58,
            bass: 0.54,
            treble: 0.59,
            cut: 0.43,
            master: 0.45,
        }
    }

    pub fn springfield() -> Self {
        Self {
            name: "SPRING".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Springfield,
            bypassed: false,
            gain: 0.48,
            bass: 0.0,
            treble: 0.58,
            cut: 0.0,
            master: 0.26,
        }
    }

    pub fn cab_ir() -> Self {
        Self {
            name: "CAB IR".to_string(),
            kind: DeviceKind::Cab,
            model: DeviceModel::CabIr,
            bypassed: false,
            gain: 0.0,
            bass: 0.0,
            treble: 0.0,
            cut: 0.0,
            master: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GreyboundUi {
    pub devices: Vec<DeviceState>,
    pub selected_index: usize,
}

impl Default for GreyboundUi {
    fn default() -> Self {
        Self {
            devices: vec![
                DeviceState::minotaur(),
                DeviceState::nox30(),
                DeviceState::springfield(),
                DeviceState::cab_ir(),
            ],
            selected_index: 0,
        }
    }
}

impl GreyboundUi {
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

    pub fn view(&self) -> Element<'_, Message> {
        let selected = &self.devices[self.selected_index];

        let top = container(
            row![
                self.global_knob("INPUT", 0.50, "0.0 dB"),
                self.global_knob("CABLE", 0.47, "470 pF"),
                self.global_knob("IR MIX", 1.0, "100%"),
                self.preset_strip(selected),
                self.global_knob("SPRING", self.springfield_mix(), "mix"),
                self.global_knob("OUTPUT", 0.58, "-3.9 dB"),
            ]
            .spacing(20)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([22, 34])
        .style(ghost_container(Color::from_rgba(0.78, 0.83, 0.95, 0.84)));

        let board = Canvas::new(BoardArt {
            devices: self.devices.clone(),
            selected_index: self.selected_index,
        })
        .width(Length::Fill)
        .height(Length::Fixed(560.0));

        let controls = self.selected_controls(selected);

        let bottom = container(
            row![
                text("TUNER").size(14),
                text("MIDI").size(14),
                text("TAP").size(14),
                text("120.0 BPM").size(14),
                text("METRONOME").size(14),
                text("SETTINGS").size(14),
                text("DEVELOPED BY GREYBOUND DSP")
                    .size(14)
                    .width(Length::Fill)
                    .horizontal_alignment(Horizontal::Right),
            ]
            .spacing(24)
            .align_items(Alignment::Center),
        )
        .padding([10, 18])
        .width(Length::Fill)
        .style(ghost_container(Color::from_rgb(0.02, 0.025, 0.03)));

        container(column![top, board, controls, bottom].spacing(0))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ghost_container(PANEL))
            .into()
    }

    fn preset_strip(&self, selected: &DeviceState) -> Element<'_, Message> {
        container(
            column![
                row![
                    text("GREYBOUND").size(18),
                    text("GREY-NOX").size(13),
                    text("GREYBOX ONLY").size(13),
                    text("RIG: grey-nox").size(13),
                ]
                .spacing(22)
                .align_items(Alignment::Center),
                row![
                    button(text("<").size(18))
                        .style(iced::theme::Button::custom(ChromeButton))
                        .padding([8, 12]),
                    container(
                        text(format!(
                            "{} / {}",
                            selected.model.title(),
                            selected.model.subtitle()
                        ))
                        .size(18)
                        .horizontal_alignment(Horizontal::Left)
                    )
                    .padding([14, 20])
                    .width(Length::Fixed(430.0))
                    .style(ghost_container(Color::from_rgba(0.94, 0.96, 1.0, 0.72))),
                    button(text(">").size(18))
                        .style(iced::theme::Button::custom(ChromeButton))
                        .padding([8, 12]),
                ]
                .spacing(10)
                .align_items(Alignment::Center),
            ]
            .spacing(12)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .into()
    }

    fn selected_controls(&self, selected: &DeviceState) -> Element<'_, Message> {
        let bypass_label = if selected.bypassed {
            "BYPASSED"
        } else {
            "ACTIVE"
        };

        let mut controls = row![button(text(bypass_label).size(13))
            .on_press(Message::ToggleBypass(!selected.bypassed))
            .style(iced::theme::Button::custom(ChromeButton))
            .padding([10, 16]),]
        .spacing(16)
        .align_items(Alignment::Center);

        controls = match selected.model {
            DeviceModel::Minotaur => controls
                .push(self.control("GAIN", selected.gain, Message::GainChanged))
                .push(self.control("TREBLE", selected.treble, Message::TrebleChanged))
                .push(self.control("OUTPUT", selected.master, Message::MasterChanged)),
            DeviceModel::Nox30 => controls
                .push(self.control("VOLUME", selected.gain, Message::GainChanged))
                .push(self.control("BASS", selected.bass, Message::BassChanged))
                .push(self.control("TREBLE", selected.treble, Message::TrebleChanged))
                .push(self.control("CUT", selected.cut, Message::CutChanged))
                .push(self.control("SAG", selected.master, Message::MasterChanged)),
            DeviceModel::Springfield => controls
                .push(self.control("DWELL", selected.gain, Message::GainChanged))
                .push(self.control("TONE", selected.treble, Message::TrebleChanged))
                .push(self.control("MIX", selected.master, Message::MasterChanged)),
            DeviceModel::CabIr => controls.push(
                container(text("IR: lab/references/tone3000-irs/celestion.wav").size(13))
                    .padding([10, 14])
                    .style(ghost_container(Color::from_rgba(0.94, 0.96, 1.0, 0.24))),
            ),
        };

        container(controls)
            .padding([12, 28])
            .width(Length::Fill)
            .style(ghost_container(Color::from_rgba(0.62, 0.69, 0.84, 0.62)))
            .into()
    }

    fn control(
        &self,
        label: &'static str,
        value: f32,
        on_change: fn(f32) -> Message,
    ) -> Element<'_, Message> {
        container(
            column![
                text(format!("{label}  {:02}", (value * 99.0).round() as i32)).size(12),
                slider(0.0..=1.0, value, on_change).width(Length::Fixed(116.0)),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .style(ghost_container(Color::from_rgba(0.94, 0.96, 1.0, 0.24)))
        .into()
    }

    fn global_knob(
        &self,
        label: &'static str,
        value: f32,
        readout: &'static str,
    ) -> Element<'_, Message> {
        container(
            column![
                text(label)
                    .size(14)
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fixed(104.0)),
                Canvas::new(KnobArt { value, label: "" })
                    .width(Length::Fixed(92.0))
                    .height(Length::Fixed(92.0)),
                text(readout)
                    .size(14)
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fixed(104.0)),
            ]
            .align_items(Alignment::Center)
            .spacing(4),
        )
        .into()
    }

    fn springfield_mix(&self) -> f32 {
        self.devices
            .iter()
            .find(|device| device.model == DeviceModel::Springfield)
            .map(|device| device.master)
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
struct BoardArt {
    devices: Vec<DeviceState>,
    selected_index: usize,
}

impl canvas::Program<Message> for BoardArt {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return (canvas::event::Status::Ignored, None);
        };

        let Some(position) = cursor.position_in(bounds) else {
            return (canvas::event::Status::Ignored, None);
        };

        if let Some(index) = hit_test_pedal(self.devices.len(), bounds.size(), position) {
            return (
                canvas::event::Status::Captured,
                Some(Message::SelectDevice(index)),
            );
        }

        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        draw_stage_background(&mut frame, bounds.size());

        let layout = board_layout(self.devices.len(), bounds.size());
        let y = 70.0;

        for (index, device) in self.devices.iter().enumerate() {
            let x = layout.start_x + index as f32 * (layout.pedal_w + layout.gap);
            let palette = match device.model {
                DeviceModel::Minotaur => Color::from_rgb(0.73, 0.65, 0.47),
                DeviceModel::Nox30 => PEDAL_CREAM,
                DeviceModel::Springfield => PEDAL_PEACH,
                DeviceModel::CabIr => PEDAL_SAGE,
            };
            draw_pedal(
                &mut frame,
                Point::new(x, y),
                Size::new(layout.pedal_w, layout.pedal_h),
                device,
                palette,
                index == self.selected_index,
            );
        }

        draw_chain_legend(&mut frame, bounds.size());
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }
}

#[derive(Debug, Clone, Copy)]
struct BoardLayout {
    start_x: f32,
    gap: f32,
    pedal_w: f32,
    pedal_h: f32,
}

fn board_layout(device_count: usize, size: Size) -> BoardLayout {
    let count = device_count.max(1) as f32;
    let gap = 34.0;
    let pedal_w = ((size.width - 86.0 - gap * (count - 1.0)) / count).clamp(190.0, 275.0);
    let pedal_h = 420.0;
    let total = pedal_w * count + gap * (count - 1.0);

    BoardLayout {
        start_x: (size.width - total) * 0.5,
        gap,
        pedal_w,
        pedal_h,
    }
}

fn hit_test_pedal(device_count: usize, size: Size, position: Point) -> Option<usize> {
    let layout = board_layout(device_count, size);
    let y = 70.0;

    (0..device_count).find(|index| {
        let x = layout.start_x + *index as f32 * (layout.pedal_w + layout.gap);
        position.x >= x
            && position.x <= x + layout.pedal_w
            && position.y >= y
            && position.y <= y + layout.pedal_h
    })
}

#[derive(Debug, Clone, Copy)]
struct KnobArt {
    value: f32,
    label: &'static str,
}

impl canvas::Program<Message> for KnobArt {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let radius = bounds.width.min(bounds.height) * 0.34;
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.47);
        draw_knob(&mut frame, center, radius, self.value, self.label);
        vec![frame.into_geometry()]
    }
}

fn draw_stage_background(frame: &mut Frame, size: Size) {
    frame.fill_rectangle(Point::ORIGIN, size, PANEL);

    for i in 0..18 {
        let y = size.height - 135.0 + i as f32 * 8.0;
        let alpha = 0.055 - i as f32 * 0.002;
        let line = Path::line(Point::new(0.0, y), Point::new(size.width, y + 26.0));
        frame.stroke(
            &line,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, alpha.max(0.0)))
                .with_width(7.0),
        );
    }

    let haze = rounded_rect(
        Point::new(20.0, 34.0),
        Size::new(size.width - 40.0, size.height - 64.0),
        28.0,
    );
    frame.stroke(
        &haze,
        Stroke::default()
            .with_color(Color::from_rgba(0.97, 0.99, 1.0, 0.18))
            .with_width(2.0),
    );
}

fn draw_pedal(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    device: &DeviceState,
    color: Color,
    selected: bool,
) {
    let shadow = rounded_rect(
        Point::new(origin.x + 12.0, origin.y + 20.0),
        Size::new(size.width, size.height),
        16.0,
    );
    frame.fill(&shadow, Color::from_rgba(0.04, 0.05, 0.08, 0.34));

    let body = rounded_rect(origin, size, 18.0);
    frame.fill(&body, darken(color, 0.04));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(if selected {
                Color::from_rgb(0.98, 0.96, 0.78)
            } else {
                Color::from_rgba(0.28, 0.22, 0.15, 0.32)
            })
            .with_width(if selected { 3.0 } else { 1.3 }),
    );

    let inner = rounded_rect(
        Point::new(origin.x + 8.0, origin.y + 8.0),
        Size::new(size.width - 16.0, size.height - 20.0),
        14.0,
    );
    frame.fill(&inner, lighten(color, 0.10));
    frame.stroke(
        &inner,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.34))
            .with_width(1.2),
    );

    draw_side_jack(
        frame,
        Point::new(origin.x - 13.0, origin.y + size.height * 0.53),
        true,
    );
    draw_side_jack(
        frame,
        Point::new(origin.x + size.width - 5.0, origin.y + size.height * 0.53),
        false,
    );

    let knob_y = origin.y + 76.0;
    match device.model {
        DeviceModel::Minotaur => {
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.30, knob_y),
                31.0,
                device.gain,
                "Gain",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.70, knob_y),
                31.0,
                device.treble,
                "Treble",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
                31.0,
                device.master,
                "Output",
            );
        }
        DeviceModel::Nox30 => {
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y),
                31.0,
                device.gain,
                "Volume",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y),
                31.0,
                device.treble,
                "Treble",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y + 88.0),
                31.0,
                device.bass,
                "Bass",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y + 88.0),
                31.0,
                device.cut,
                "Cut",
            );
        }
        DeviceModel::Springfield => {
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.30, knob_y),
                31.0,
                device.gain,
                "Dwell",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.70, knob_y),
                31.0,
                device.treble,
                "Tone",
            );
            draw_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
                31.0,
                device.master,
                "Mix",
            );
        }
        DeviceModel::CabIr => {
            draw_text(
                frame,
                "celestion.wav",
                Point::new(origin.x + size.width * 0.50, knob_y + 18.0),
                20.0,
                Color::from_rgb(0.03, 0.03, 0.035),
                Horizontal::Center,
            );
            draw_text(
                frame,
                "optional post-amp convolution",
                Point::new(origin.x + size.width * 0.50, knob_y + 54.0),
                13.0,
                Color::from_rgba(0.03, 0.03, 0.035, 0.72),
                Horizontal::Center,
            );
        }
    }

    draw_text(
        frame,
        device.model.subtitle(),
        Point::new(origin.x + size.width * 0.50, origin.y + 30.0),
        13.0,
        Color::from_rgb(0.02, 0.025, 0.03),
        Horizontal::Center,
    );

    let title_y = origin.y + size.height * 0.56;
    draw_text(
        frame,
        ">  >>>>",
        Point::new(origin.x + 27.0, title_y),
        22.0,
        Color::from_rgb(0.11, 0.12, 0.12),
        Horizontal::Left,
    );
    draw_text(
        frame,
        &device.name,
        Point::new(origin.x + size.width * 0.50, title_y + 1.0),
        27.0,
        Color::from_rgb(0.02, 0.025, 0.03),
        Horizontal::Center,
    );
    draw_text(
        frame,
        ">>>>  >",
        Point::new(origin.x + size.width - 28.0, title_y),
        22.0,
        Color::from_rgb(0.11, 0.12, 0.12),
        Horizontal::Right,
    );

    let plate_origin = Point::new(origin.x + 18.0, origin.y + size.height * 0.64);
    let plate_size = Size::new(size.width - 36.0, size.height * 0.27);
    draw_texture_plate(frame, plate_origin, plate_size, device.name.as_str());

    let led = Path::circle(
        Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.69),
        10.0,
    );
    frame.fill(
        &led,
        if device.bypassed {
            Color::from_rgb(0.09, 0.25, 0.25)
        } else {
            Color::from_rgb(0.0, 0.75, 0.78)
        },
    );
    frame.stroke(
        &led,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.48))
            .with_width(2.0),
    );

    draw_footswitch(
        frame,
        Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.81),
    );
}

fn draw_knob(frame: &mut Frame, center: Point, radius: f32, value: f32, label: &str) {
    for tick in 0..25 {
        let t = tick as f32 / 24.0;
        let angle = (-135.0 + t * 270.0).to_radians();
        let inner = Point::new(
            center.x + angle.cos() * (radius + 8.0),
            center.y + angle.sin() * (radius + 8.0),
        );
        let outer = Point::new(
            center.x + angle.cos() * (radius + 15.0),
            center.y + angle.sin() * (radius + 15.0),
        );
        frame.stroke(
            &Path::line(inner, outer),
            Stroke::default()
                .with_color(Color::from_rgba(0.08, 0.09, 0.10, 0.54))
                .with_width(if tick % 4 == 0 { 1.8 } else { 1.0 }),
        );
    }

    frame.fill(
        &Path::circle(Point::new(center.x + 4.0, center.y + 8.0), radius + 4.0),
        Color::from_rgba(0.06, 0.05, 0.04, 0.22),
    );
    frame.fill(&Path::circle(center, radius), darken(TEAL, 0.10));

    for ring in 0..9 {
        let r = radius - ring as f32 * 2.0;
        frame.stroke(
            &Path::circle(center, r),
            Stroke::default()
                .with_color(Color::from_rgba(0.80, 0.93, 0.90, 0.06))
                .with_width(1.0),
        );
    }

    for tooth in 0..28 {
        let angle = tooth as f32 / 28.0 * std::f32::consts::TAU;
        let a = Point::new(
            center.x + angle.cos() * (radius - 2.0),
            center.y + angle.sin() * (radius - 2.0),
        );
        let b = Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        );
        frame.stroke(
            &Path::line(a, b),
            Stroke::default()
                .with_color(Color::from_rgba(0.03, 0.08, 0.08, 0.36))
                .with_width(2.0),
        );
    }

    let cap = Path::circle(
        Point::new(center.x - radius * 0.22, center.y - radius * 0.22),
        radius * 0.42,
    );
    frame.fill(&cap, Color::from_rgba(0.86, 0.98, 0.94, 0.13));

    let angle = (-130.0 + value.clamp(0.0, 1.0) * 260.0).to_radians();
    let pointer = Path::line(
        Point::new(
            center.x + angle.cos() * radius * 0.20,
            center.y + angle.sin() * radius * 0.20,
        ),
        Point::new(
            center.x + angle.cos() * radius * 0.74,
            center.y + angle.sin() * radius * 0.74,
        ),
    );
    frame.stroke(
        &pointer,
        Stroke::default()
            .with_color(Color::from_rgb(0.92, 0.88, 0.80))
            .with_width(5.0),
    );

    if !label.is_empty() {
        draw_text(
            frame,
            label,
            Point::new(center.x, center.y + radius + 22.0),
            14.0,
            Color::from_rgb(0.03, 0.03, 0.035),
            Horizontal::Center,
        );
    }
}

fn draw_texture_plate(frame: &mut Frame, origin: Point, size: Size, name: &str) {
    let plate = rounded_rect(origin, size, 5.0);
    frame.fill(&plate, Color::from_rgb(0.43, 0.27, 0.22));
    frame.stroke(
        &plate,
        Stroke::default()
            .with_color(Color::from_rgb(0.16, 0.08, 0.07))
            .with_width(2.0),
    );

    match name {
        "CAB IR" => {
            let center = Point::new(origin.x + size.width * 0.5, origin.y + size.height * 0.5);
            for i in 0..18 {
                frame.stroke(
                    &Path::circle(center, 14.0 + i as f32 * 8.0),
                    Stroke::default()
                        .with_color(Color::from_rgba(0.78, 0.62, 0.49, 0.54))
                        .with_width(4.0),
                );
            }
        }
        "SPRING" => {
            for y in 0..6 {
                for x in 0..9 {
                    let cx = origin.x + 16.0 + x as f32 * (size.width - 32.0) / 8.0;
                    let cy = origin.y + 14.0 + y as f32 * (size.height - 28.0) / 5.0;
                    let leaf =
                        rounded_rect(Point::new(cx - 8.0, cy - 5.0), Size::new(16.0, 10.0), 6.0);
                    frame.stroke(
                        &leaf,
                        Stroke::default()
                            .with_color(Color::from_rgba(0.72, 0.50, 0.39, 0.50))
                            .with_width(1.5),
                    );
                }
            }
        }
        _ => {
            for i in 0..20 {
                let x = origin.x + 8.0 + i as f32 * (size.width - 16.0) / 20.0;
                frame.stroke(
                    &Path::line(
                        Point::new(x, origin.y + 4.0),
                        Point::new(x + 8.0, origin.y + size.height - 4.0),
                    ),
                    Stroke::default()
                        .with_color(Color::from_rgba(0.78, 0.45, 0.32, 0.42))
                        .with_width(3.0),
                );
            }
        }
    }
}

fn draw_footswitch(frame: &mut Frame, center: Point) {
    frame.fill(
        &Path::circle(Point::new(center.x + 4.0, center.y + 8.0), 43.0),
        Color::from_rgba(0.02, 0.015, 0.01, 0.30),
    );
    frame.fill(
        &Path::circle(center, 43.0),
        Color::from_rgb(0.49, 0.34, 0.18),
    );
    frame.fill(&Path::circle(center, 34.0), GOLD);
    frame.stroke(
        &Path::circle(center, 34.0),
        Stroke::default()
            .with_color(Color::from_rgb(0.94, 0.82, 0.57))
            .with_width(3.0),
    );
    frame.fill(
        &Path::circle(center, 23.0),
        Color::from_rgb(0.11, 0.07, 0.05),
    );
    frame.fill(
        &Path::circle(Point::new(center.x, center.y - 4.0), 18.0),
        TEAL,
    );
    frame.stroke(
        &Path::circle(Point::new(center.x, center.y - 4.0), 18.0),
        Stroke::default()
            .with_color(Color::from_rgb(0.92, 0.86, 0.70))
            .with_width(2.0),
    );
}

fn draw_side_jack(frame: &mut Frame, origin: Point, left: bool) {
    let body = rounded_rect(origin, Size::new(23.0, 52.0), 5.0);
    frame.fill(&body, Color::from_rgb(0.43, 0.28, 0.12));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgb(0.88, 0.72, 0.42))
            .with_width(2.0),
    );
    let sign = if left { -1.0 } else { 1.0 };
    frame.stroke(
        &Path::line(
            Point::new(origin.x + 12.0, origin.y + 8.0),
            Point::new(origin.x + 12.0 + sign * 12.0, origin.y + 44.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.85, 0.55, 0.52))
            .with_width(2.0),
    );
}

fn draw_chain_legend(frame: &mut Frame, size: Size) {
    let y = size.height - 54.0;
    let center = size.width * 0.5;
    frame.stroke(
        &Path::line(
            Point::new(center - 72.0, y + 26.0),
            Point::new(center + 72.0, y + 26.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.72))
            .with_width(4.0),
    );
    for i in 0..3 {
        let x = center - 62.0 + i as f32 * 62.0;
        frame.stroke(
            &rounded_rect(Point::new(x, y), Size::new(48.0, 42.0), 4.0),
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.90))
                .with_width(3.0),
        );
        frame.fill(
            &Path::circle(Point::new(x + 15.0, y + 25.0), 10.0),
            Color::from_rgba(1.0, 1.0, 1.0, 0.72),
        );
        frame.fill(
            &Path::circle(Point::new(x + 34.0, y + 25.0), 10.0),
            Color::from_rgba(1.0, 1.0, 1.0, 0.72),
        );
    }
}

fn draw_text(
    frame: &mut Frame,
    content: &str,
    position: Point,
    size: f32,
    color: Color,
    align: Horizontal,
) {
    frame.fill_text(Text {
        content: content.to_string(),
        position,
        color,
        size,
        horizontal_alignment: align,
        vertical_alignment: Vertical::Center,
        ..Text::default()
    });
}

fn rounded_rect(origin: Point, size: Size, radius: f32) -> Path {
    let r = radius.min(size.width * 0.5).min(size.height * 0.5);
    Path::new(|p| {
        p.move_to(Point::new(origin.x + r, origin.y));
        p.line_to(Point::new(origin.x + size.width - r, origin.y));
        p.quadratic_curve_to(
            Point::new(origin.x + size.width, origin.y),
            Point::new(origin.x + size.width, origin.y + r),
        );
        p.line_to(Point::new(
            origin.x + size.width,
            origin.y + size.height - r,
        ));
        p.quadratic_curve_to(
            Point::new(origin.x + size.width, origin.y + size.height),
            Point::new(origin.x + size.width - r, origin.y + size.height),
        );
        p.line_to(Point::new(origin.x + r, origin.y + size.height));
        p.quadratic_curve_to(
            Point::new(origin.x, origin.y + size.height),
            Point::new(origin.x, origin.y + size.height - r),
        );
        p.line_to(Point::new(origin.x, origin.y + r));
        p.quadratic_curve_to(
            Point::new(origin.x, origin.y),
            Point::new(origin.x + r, origin.y),
        );
        p.close();
    })
}

fn lighten(color: Color, amount: f32) -> Color {
    Color::from_rgb(
        (color.r + amount).min(1.0),
        (color.g + amount).min(1.0),
        (color.b + amount).min(1.0),
    )
}

fn darken(color: Color, amount: f32) -> Color {
    Color::from_rgb(
        (color.r - amount).max(0.0),
        (color.g - amount).max(0.0),
        (color.b - amount).max(0.0),
    )
}
