pub mod components;

use components::{KnobSkin, KnobSpec};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, pick_list, row, text};
use iced::{mouse, Alignment, Background, Color, Element, Length, Point, Rectangle, Size, Vector};
use std::rc::Rc;

const INK: Color = Color::from_rgb(0.09, 0.12, 0.24);
const PANEL: Color = Color::from_rgb(0.72, 0.78, 0.91);
const PEDAL_CREAM: Color = Color::from_rgb(0.84, 0.80, 0.72);
const PEDAL_PEACH: Color = Color::from_rgb(0.77, 0.56, 0.45);
const PEDAL_SAGE: Color = Color::from_rgb(0.67, 0.62, 0.49);
const TEAL: Color = Color::from_rgb(0.35, 0.56, 0.57);
const GOLD: Color = Color::from_rgb(0.76, 0.61, 0.35);
pub const DESIGN_WIDTH: f32 = 1600.0;
pub const DESIGN_HEIGHT: f32 = 900.0;

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

struct FooterButton {
    selected: bool,
}

impl button::StyleSheet for FooterButton {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: self
                .selected
                .then_some(Background::Color(Color::from_rgba(0.42, 0.47, 0.58, 0.80))),
            border_radius: 9.0.into(),
            border_width: if self.selected { 1.0 } else { 0.0 },
            border_color: Color::from_rgba(0.72, 0.76, 0.86, 0.40),
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: Color::from_rgb(0.88, 0.90, 0.95),
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.42, 0.47, 0.58, 0.34))),
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

struct DarkContainer;

impl container::StyleSheet for DarkContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.84, 0.84, 0.84)),
            background: Some(Background::Color(Color::from_rgb(0.17, 0.17, 0.17))),
            border_radius: 24.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(0.0, 0.0, 0.0, 0.65),
            ..container::Appearance::default()
        }
    }
}

fn dark_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(DarkContainer))
}

struct ModalTitleBarContainer;

impl container::StyleSheet for ModalTitleBarContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgb(0.095, 0.095, 0.095))),
            border_radius: 24.0.into(),
            ..container::Appearance::default()
        }
    }
}

fn modal_title_bar_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(ModalTitleBarContainer))
}

struct ModalBodyContainer;

impl container::StyleSheet for ModalBodyContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.86, 0.86, 0.86)),
            background: None,
            ..container::Appearance::default()
        }
    }
}

fn modal_body_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(ModalBodyContainer))
}

struct ModalRuleContainer;

impl container::StyleSheet for ModalRuleContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.02, 0.02, 0.02))),
            ..container::Appearance::default()
        }
    }
}

fn modal_rule_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(ModalRuleContainer))
}

struct DarkFieldContainer;

impl container::StyleSheet for DarkFieldContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgb(0.055, 0.055, 0.055))),
            border_radius: 18.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
            ..container::Appearance::default()
        }
    }
}

fn dark_field_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(DarkFieldContainer))
}

struct DarkPickList;

impl pick_list::StyleSheet for DarkPickList {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> pick_list::Appearance {
        pick_list::Appearance {
            text_color: Color::WHITE,
            placeholder_color: Color::from_rgb(0.72, 0.72, 0.72),
            handle_color: Color::WHITE,
            background: Background::Color(Color::from_rgb(0.055, 0.055, 0.055)),
            border_radius: 12.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        }
    }

    fn hovered(&self, _style: &Self::Style) -> pick_list::Appearance {
        pick_list::Appearance {
            border_color: Color::from_rgb(0.54, 0.64, 0.92),
            ..self.active(_style)
        }
    }
}

struct DarkMenu;

impl iced::overlay::menu::StyleSheet for DarkMenu {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::overlay::menu::Appearance {
        iced::overlay::menu::Appearance {
            text_color: Color::WHITE,
            background: Background::Color(Color::from_rgb(0.08, 0.08, 0.08)),
            border_width: 1.0,
            border_radius: 8.0.into(),
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            selected_text_color: Color::WHITE,
            selected_background: Background::Color(Color::from_rgb(0.20, 0.24, 0.35)),
        }
    }
}

fn dark_pick_list() -> iced::theme::PickList {
    iced::theme::PickList::Custom(Rc::new(DarkPickList), Rc::new(DarkMenu))
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectDevice(usize),
    SelectView(ViewMode),
    ToggleAudioSettings,
    CloseAudioSettings,
    AudioInputSelected(String),
    AudioOutputSelected(String),
    AudioSampleRateSelected(String),
    AudioBufferSizeSelected(String),
    AudioDevicesChanged {
        inputs: Vec<String>,
        outputs: Vec<String>,
        selected_input: Option<String>,
        selected_output: Option<String>,
        status: String,
    },
    AudioStatusChanged(String),
    MeterProbeTick(std::time::Instant),
    MeterLevelsChanged {
        input: f32,
        output: f32,
    },
    WindowResized {
        width: u32,
        height: u32,
    },
    ToggleBypass(bool),
    SetDeviceControl {
        index: usize,
        control: ControlKind,
        value: f32,
    },
    GainChanged(f32),
    BassChanged(f32),
    TrebleChanged(f32),
    CutChanged(f32),
    MasterChanged(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Gain,
    Bass,
    Treble,
    Cut,
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Pedals,
    Amp,
    Cab,
}

impl ViewMode {
    fn label(self) -> &'static str {
        match self {
            Self::Pedals => "PEDALS",
            Self::Amp => "AMP",
            Self::Cab => "CAB",
        }
    }
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

    fn control_value(&self, control: ControlKind) -> f32 {
        match control {
            ControlKind::Gain => self.gain,
            ControlKind::Bass => self.bass,
            ControlKind::Treble => self.treble,
            ControlKind::Cut => self.cut,
            ControlKind::Master => self.master,
        }
    }

    fn set_control(&mut self, control: ControlKind, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match control {
            ControlKind::Gain => self.gain = value,
            ControlKind::Bass => self.bass = value,
            ControlKind::Treble => self.treble = value,
            ControlKind::Cut => self.cut = value,
            ControlKind::Master => self.master = value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GreyboundUi {
    pub devices: Vec<DeviceState>,
    pub amp: DeviceState,
    pub cab: DeviceState,
    pub meters: MeterLevels,
    pub audio_settings: AudioSettingsState,
    pub selected_index: usize,
    pub view_mode: ViewMode,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct AudioSettingsState {
    pub open: bool,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub selected_input: Option<String>,
    pub selected_output: Option<String>,
    pub status: String,
    pub sample_rate: u32,
    pub period_size: u32,
    pub sample_rates: Vec<String>,
    pub period_sizes: Vec<String>,
}

impl Default for AudioSettingsState {
    fn default() -> Self {
        Self {
            open: false,
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected_input: None,
            selected_output: None,
            status: "Audio engine starting".to_string(),
            sample_rate: 48_000,
            period_size: 32,
            sample_rates: vec![
                "44100 Hz".to_string(),
                "48000 Hz".to_string(),
                "88200 Hz".to_string(),
                "96000 Hz".to_string(),
            ],
            period_sizes: vec![
                "32 samples".to_string(),
                "64 samples".to_string(),
                "128 samples".to_string(),
                "256 samples".to_string(),
                "512 samples".to_string(),
                "1024 samples".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MeterLevels {
    pub input: f32,
    pub output: f32,
}

impl Default for MeterLevels {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
        }
    }
}

impl Default for GreyboundUi {
    fn default() -> Self {
        Self {
            devices: vec![DeviceState::minotaur(), DeviceState::springfield()],
            amp: DeviceState::nox30(),
            cab: DeviceState::cab_ir(),
            meters: MeterLevels::default(),
            audio_settings: AudioSettingsState::default(),
            selected_index: 0,
            view_mode: ViewMode::Pedals,
            scale: 1.0,
        }
    }
}

impl GreyboundUi {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectDevice(index) => {
                if index < self.devices.len() {
                    self.selected_index = index;
                    self.view_mode = ViewMode::Pedals;
                }
            }
            Message::SelectView(view_mode) => {
                self.view_mode = view_mode;
            }
            Message::ToggleAudioSettings => {
                self.audio_settings.open = !self.audio_settings.open;
            }
            Message::CloseAudioSettings => {
                self.audio_settings.open = false;
            }
            Message::AudioInputSelected(device) => {
                self.audio_settings.selected_input = Some(device);
                self.audio_settings.status = "Restarting audio engine".to_string();
            }
            Message::AudioOutputSelected(device) => {
                self.audio_settings.selected_output = Some(device);
                self.audio_settings.status = "Restarting audio engine".to_string();
            }
            Message::AudioSampleRateSelected(value) => {
                if let Some(sample_rate) = parse_prefixed_u32(&value) {
                    self.audio_settings.sample_rate = sample_rate;
                    self.audio_settings.status = "Restarting audio engine".to_string();
                }
            }
            Message::AudioBufferSizeSelected(value) => {
                if let Some(period_size) = parse_prefixed_u32(&value) {
                    self.audio_settings.period_size = period_size;
                    self.audio_settings.status = "Restarting audio engine".to_string();
                }
            }
            Message::AudioDevicesChanged {
                inputs,
                outputs,
                selected_input,
                selected_output,
                status,
            } => {
                self.audio_settings.inputs = inputs;
                self.audio_settings.outputs = outputs;
                self.audio_settings.selected_input = selected_input;
                self.audio_settings.selected_output = selected_output;
                self.audio_settings.status = status;
            }
            Message::AudioStatusChanged(status) => {
                self.audio_settings.status = status;
            }
            Message::MeterProbeTick(_) => {}
            Message::MeterLevelsChanged { input, output } => {
                self.meters.input = input.clamp(0.0, 1.0);
                self.meters.output = output.clamp(0.0, 1.0);
            }
            Message::WindowResized { width, height } => {
                self.scale = uniform_scale(width as f32, height as f32);
            }
            Message::ToggleBypass(value) => {
                self.active_device_mut().bypassed = value;
            }
            Message::SetDeviceControl {
                index,
                control,
                value,
            } => {
                if let Some(device) = self.devices.get_mut(index) {
                    device.set_control(control, value);
                    self.selected_index = index;
                    self.view_mode = ViewMode::Pedals;
                }
            }
            Message::GainChanged(value) => {
                self.active_device_mut().gain = value;
            }
            Message::BassChanged(value) => {
                self.active_device_mut().bass = value;
            }
            Message::TrebleChanged(value) => {
                self.active_device_mut().treble = value;
            }
            Message::CutChanged(value) => {
                self.active_device_mut().cut = value;
            }
            Message::MasterChanged(value) => {
                self.active_device_mut().master = value;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.active_device();
        let scale = self.scale;

        let top = container(
            row![
                self.metered_global_knob("INPUT", 0.50, "0.0 dB", self.meters.input),
                self.global_knob("CABLE", 0.47, "470 pF"),
                self.global_knob("IR MIX", 1.0, "100%"),
                self.preset_strip(selected),
                self.global_knob("SPRING", self.springfield_mix(), "mix"),
                self.metered_global_knob("OUTPUT", 0.58, "-3.9 dB", self.meters.output),
            ]
            .spacing(self.s(20.0))
            .align_items(Alignment::Center),
        )
        .width(Length::Fixed(self.s(DESIGN_WIDTH)))
        .height(Length::Fixed(self.s(190.0)))
        .padding([self.s(22.0), self.s(34.0)])
        .style(ghost_container(Color::from_rgba(0.78, 0.83, 0.95, 0.84)));

        let main_view: Element<'_, Message> = if self.audio_settings.open {
            self.audio_settings_panel()
        } else {
            match self.view_mode {
                ViewMode::Pedals => Canvas::new(BoardArt {
                    devices: self.devices.clone(),
                    selected_index: self.selected_index,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(666.0)))
                .into(),
                ViewMode::Amp => Canvas::new(AmpArt {
                    amp: self.amp.clone(),
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(666.0)))
                .into(),
                ViewMode::Cab => Canvas::new(CabArt {
                    cab: self.cab.clone(),
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(666.0)))
                .into(),
            }
        };

        let bottom_text = Color::from_rgb(0.80, 0.82, 0.88);
        let bottom = container(
            row![
                text("TUNER").size(self.font(14.0)).style(bottom_text),
                text("MIDI").size(self.font(14.0)).style(bottom_text),
                text("TAP").size(self.font(14.0)).style(bottom_text),
                text("120.0 BPM").size(self.font(14.0)).style(bottom_text),
                text("METRONOME").size(self.font(14.0)).style(bottom_text),
                button(text("SETTINGS").size(self.font(14.0)).style(Color::WHITE))
                    .on_press(Message::ToggleAudioSettings)
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: self.audio_settings.open
                    }))
                    .padding([self.s(4.0), self.s(10.0)]),
                text("DEVELOPED BY GREYBOUND")
                    .size(self.font(14.0))
                    .style(bottom_text)
                    .width(Length::Fill)
                    .horizontal_alignment(Horizontal::Right),
            ]
            .spacing(self.s(24.0))
            .align_items(Alignment::Center),
        )
        .padding([self.s(10.0), self.s(18.0)])
        .width(Length::Fixed(self.s(DESIGN_WIDTH)))
        .height(Length::Fixed(self.s(44.0)))
        .style(ghost_container(Color::from_rgb(0.02, 0.025, 0.03)));

        let panel = container(column![top, main_view, bottom].spacing(0))
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(DESIGN_HEIGHT)))
            .style(ghost_container(PANEL));

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn preset_strip(&self, selected: &DeviceState) -> Element<'_, Message> {
        container(
            column![
                row![
                    text("GREYBOUND").size(self.font(18.0)),
                    text("GREY-NOX").size(self.font(13.0)),
                    text("GREYBOX ONLY").size(self.font(13.0)),
                    text("RIG: grey-nox").size(self.font(13.0)),
                ]
                .spacing(self.s(22.0))
                .align_items(Alignment::Center),
                row![
                    self.view_button(ViewMode::Pedals),
                    self.view_button(ViewMode::Amp),
                    self.view_button(ViewMode::Cab),
                ]
                .spacing(self.s(10.0))
                .align_items(Alignment::Center),
                row![
                    button(text("<").size(self.font(18.0)))
                        .style(iced::theme::Button::custom(ChromeButton))
                        .padding([self.s(8.0), self.s(12.0)]),
                    container(
                        text(format!(
                            "{} / {}",
                            selected.model.title(),
                            selected.model.subtitle()
                        ))
                        .size(self.font(18.0))
                        .horizontal_alignment(Horizontal::Left)
                    )
                    .padding([self.s(14.0), self.s(20.0)])
                    .width(Length::Fixed(self.s(430.0)))
                    .style(ghost_container(Color::from_rgba(0.94, 0.96, 1.0, 0.72))),
                    button(text(">").size(self.font(18.0)))
                        .style(iced::theme::Button::custom(ChromeButton))
                        .padding([self.s(8.0), self.s(12.0)]),
                ]
                .spacing(self.s(10.0))
                .align_items(Alignment::Center),
            ]
            .spacing(self.s(12.0))
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .into()
    }

    fn view_button(&self, view_mode: ViewMode) -> Element<'_, Message> {
        let label = if self.view_mode == view_mode {
            format!("* {}", view_mode.label())
        } else {
            view_mode.label().to_string()
        };

        button(text(label).size(self.font(12.0)))
            .on_press(Message::SelectView(view_mode))
            .style(iced::theme::Button::custom(ChromeButton))
            .padding([self.s(8.0), self.s(12.0)])
            .into()
    }

    fn audio_settings_panel(&self) -> Element<'_, Message> {
        let settings = &self.audio_settings;
        let input = pick_list(
            settings.inputs.clone(),
            settings.selected_input.clone(),
            Message::AudioInputSelected,
        )
        .style(dark_pick_list())
        .text_size(self.font(17.0) as f32)
        .padding([self.s(12.0), self.s(14.0)])
        .width(Length::Fixed(self.s(390.0)));
        let output = pick_list(
            settings.outputs.clone(),
            settings.selected_output.clone(),
            Message::AudioOutputSelected,
        )
        .style(dark_pick_list())
        .text_size(self.font(17.0) as f32)
        .padding([self.s(12.0), self.s(14.0)])
        .width(Length::Fixed(self.s(390.0)));
        let sample_rate = pick_list(
            settings.sample_rates.clone(),
            Some(format!("{} Hz", settings.sample_rate)),
            Message::AudioSampleRateSelected,
        )
        .style(dark_pick_list())
        .text_size(self.font(17.0) as f32)
        .padding([self.s(12.0), self.s(14.0)])
        .width(Length::Fixed(self.s(185.0)));
        let period_size = pick_list(
            settings.period_sizes.clone(),
            Some(format!("{} samples", settings.period_size)),
            Message::AudioBufferSizeSelected,
        )
        .style(dark_pick_list())
        .text_size(self.font(17.0) as f32)
        .padding([self.s(12.0), self.s(14.0)])
        .width(Length::Fixed(self.s(185.0)));

        let content = column![
            row![
                self.settings_field(
                    "Audio Device Type",
                    text("CoreAudio")
                        .size(self.font(18.0))
                        .style(Color::WHITE)
                        .into()
                ),
                self.settings_field(
                    "Status",
                    text(settings.status.as_str())
                        .size(self.font(15.0))
                        .style(Color::WHITE)
                        .into()
                ),
            ]
            .spacing(self.s(44.0)),
            self.settings_separator(),
            row![
                self.settings_select_field("Audio Input Device", input.into(), 390.0),
                self.settings_select_field("Audio Output Device", output.into(), 390.0),
            ]
            .spacing(self.s(44.0)),
            row![
                self.settings_box("Audio Input Channels", "1"),
                self.settings_box("Audio Output Channels", "1 + 2"),
                self.settings_select_field("Sample Rate", sample_rate.into(), 185.0),
                self.settings_select_field("Audio Buffer Size", period_size.into(), 185.0),
            ]
            .spacing(self.s(28.0)),
        ]
        .spacing(self.s(28.0));

        let modal = self.modal_frame("Audio Settings", content.into());

        container(modal)
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(666.0)))
            .center_x()
            .center_y()
            .style(ghost_container(Color::from_rgba(0.04, 0.05, 0.08, 0.58)))
            .into()
    }

    fn modal_frame<'a>(
        &self,
        title: &'static str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let close = button(text("X").size(self.font(26.0)).style(Color::WHITE))
            .on_press(Message::CloseAudioSettings)
            .style(iced::theme::Button::custom(FooterButton {
                selected: false,
            }))
            .padding([self.s(2.0), self.s(8.0)]);

        container(
            column![
                container(
                    row![
                        text(title).size(self.font(24.0)).style(Color::WHITE),
                        container(close)
                            .width(Length::Fill)
                            .align_x(Horizontal::Right),
                    ]
                    .align_items(Alignment::Center)
                )
                .height(Length::Fixed(self.s(58.0)))
                .width(Length::Fill)
                .padding([0.0, self.s(36.0)])
                .style(modal_title_bar_container()),
                container("")
                    .height(Length::Fixed(self.s(1.0)))
                    .width(Length::Fill)
                    .style(modal_rule_container()),
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([self.s(34.0), self.s(36.0)])
                    .style(modal_body_container()),
            ]
            .spacing(0),
        )
        .width(Length::Fixed(self.s(980.0)))
        .height(Length::Fixed(self.s(590.0)))
        .style(dark_container())
        .into()
    }

    fn settings_field<'a>(
        &self,
        label: &'static str,
        control: Element<'a, Message>,
    ) -> Element<'a, Message> {
        self.settings_field_width(label, control, 410.0)
    }

    fn settings_field_width<'a>(
        &self,
        label: &'static str,
        control: Element<'a, Message>,
        width: f32,
    ) -> Element<'a, Message> {
        container(
            column![
                text(label).size(self.font(16.0)).style(Color::WHITE),
                container(control)
                    .padding([self.s(12.0), self.s(14.0)])
                    .width(Length::Fixed(self.s(width)))
                    .style(dark_field_container()),
            ]
            .spacing(self.s(8.0)),
        )
        .into()
    }

    fn settings_select_field<'a>(
        &self,
        label: &'static str,
        control: Element<'a, Message>,
        width: f32,
    ) -> Element<'a, Message> {
        container(
            column![
                text(label).size(self.font(16.0)).style(Color::WHITE),
                control,
            ]
            .spacing(self.s(8.0)),
        )
        .width(Length::Fixed(self.s(width)))
        .into()
    }

    fn settings_separator(&self) -> Element<'_, Message> {
        container("")
            .height(Length::Fixed(self.s(1.0)))
            .width(Length::Fill)
            .style(modal_rule_container())
            .into()
    }

    fn settings_box(&self, label: &'static str, value: &str) -> Element<'_, Message> {
        container(
            column![
                text(label).size(self.font(15.0)).style(Color::WHITE),
                container(
                    text(value.to_string())
                        .size(self.font(18.0))
                        .style(Color::WHITE)
                )
                .padding([self.s(14.0), self.s(16.0)])
                .width(Length::Fixed(self.s(185.0)))
                .height(Length::Fixed(self.s(58.0)))
                .style(dark_field_container()),
            ]
            .spacing(self.s(8.0)),
        )
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
                    .size(self.font(14.0))
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fixed(self.s(104.0))),
                Canvas::new(KnobArt { value, label: "" })
                    .width(Length::Fixed(self.s(92.0)))
                    .height(Length::Fixed(self.s(92.0))),
                text(readout)
                    .size(self.font(14.0))
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fixed(self.s(104.0))),
            ]
            .align_items(Alignment::Center)
            .spacing(self.s(4.0)),
        )
        .into()
    }

    fn metered_global_knob(
        &self,
        label: &'static str,
        value: f32,
        readout: &'static str,
        meter_level: f32,
    ) -> Element<'_, Message> {
        row![
            Canvas::new(MeterArt { level: meter_level })
                .width(Length::Fixed(self.s(18.0)))
                .height(Length::Fixed(self.s(132.0))),
            self.global_knob(label, value, readout),
        ]
        .spacing(self.s(12.0))
        .align_items(Alignment::Center)
        .into()
    }

    fn springfield_mix(&self) -> f32 {
        self.devices
            .iter()
            .find(|device| device.model == DeviceModel::Springfield)
            .map(|device| device.master)
            .unwrap_or(0.0)
    }

    fn active_device(&self) -> &DeviceState {
        match self.view_mode {
            ViewMode::Pedals => &self.devices[self.selected_index],
            ViewMode::Amp => &self.amp,
            ViewMode::Cab => &self.cab,
        }
    }

    fn active_device_mut(&mut self) -> &mut DeviceState {
        match self.view_mode {
            ViewMode::Pedals => &mut self.devices[self.selected_index],
            ViewMode::Amp => &mut self.amp,
            ViewMode::Cab => &mut self.cab,
        }
    }

    fn s(&self, value: f32) -> f32 {
        value * self.scale
    }

    fn font(&self, value: f32) -> u16 {
        (value * self.scale).round().max(1.0) as u16
    }
}

fn uniform_scale(width: f32, height: f32) -> f32 {
    (width / DESIGN_WIDTH)
        .min(height / DESIGN_HEIGHT)
        .clamp(0.45, 1.60)
}

fn parse_prefixed_u32(value: &str) -> Option<u32> {
    value.split_whitespace().next()?.parse().ok()
}

#[derive(Debug, Clone)]
struct BoardArt {
    devices: Vec<DeviceState>,
    selected_index: usize,
    scale: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct DragState {
    gesture: Option<DragGesture>,
}

#[derive(Debug, Clone, Copy)]
struct DragGesture {
    index: Option<usize>,
    control: ControlKind,
    start_position: Point,
    start_value: f32,
}

impl canvas::Program<Message> for BoardArt {
    type State = DragState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor
                    .position_in(bounds)
                    .map(|p| unscale_point(p, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };

                if let Some((index, control)) = hit_test_pedal_knob(
                    &self.devices,
                    unscale_size(bounds.size(), self.scale),
                    position,
                ) {
                    let start_value = self.devices[index].control_value(control);
                    state.gesture = Some(DragGesture {
                        index: Some(index),
                        control,
                        start_position: position,
                        start_value,
                    });
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SetDeviceControl {
                            index,
                            control,
                            value: start_value,
                        }),
                    );
                }

                if let Some(index) = hit_test_pedal(
                    self.devices.len(),
                    unscale_size(bounds.size(), self.scale),
                    position,
                ) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SelectDevice(index)),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(gesture) = state.gesture else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(position) = cursor
                    .position_in(bounds)
                    .map(|p| unscale_point(p, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(index) = gesture.index else {
                    return (canvas::event::Status::Ignored, None);
                };

                return (
                    canvas::event::Status::Captured,
                    Some(Message::SetDeviceControl {
                        index,
                        control: gesture.control,
                        value: dragged_value(gesture, position),
                    }),
                );
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.gesture = None;
                return (canvas::event::Status::Captured, None);
            }
            _ => {}
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
        frame.scale(self.scale);
        let logical_size = unscale_size(bounds.size(), self.scale);
        draw_stage_background(&mut frame, logical_size);

        let layout = board_layout(self.devices.len(), logical_size);
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

#[derive(Debug, Clone)]
struct AmpArt {
    amp: DeviceState,
    scale: f32,
}

impl canvas::Program<Message> for AmpArt {
    type State = DragState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor
                    .position_in(bounds)
                    .map(|p| unscale_point(p, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };

                if let Some(control) =
                    hit_test_amp_knob(unscale_size(bounds.size(), self.scale), position)
                {
                    let start_value = self.amp.control_value(control);
                    state.gesture = Some(DragGesture {
                        index: None,
                        control,
                        start_position: position,
                        start_value,
                    });
                    return (
                        canvas::event::Status::Captured,
                        Some(control_message(control, start_value)),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(gesture) = state.gesture else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(position) = cursor
                    .position_in(bounds)
                    .map(|p| unscale_point(p, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };

                return (
                    canvas::event::Status::Captured,
                    Some(control_message(
                        gesture.control,
                        dragged_value(gesture, position),
                    )),
                );
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.gesture = None;
                return (canvas::event::Status::Captured, None);
            }
            _ => {}
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
        frame.scale(self.scale);
        let logical_size = unscale_size(bounds.size(), self.scale);
        draw_stage_background(&mut frame, logical_size);
        draw_amp_head(&mut frame, logical_size, &self.amp);
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone)]
struct CabArt {
    cab: DeviceState,
    scale: f32,
}

impl canvas::Program<Message> for CabArt {
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
        frame.scale(self.scale);
        let logical_size = unscale_size(bounds.size(), self.scale);
        draw_stage_background(&mut frame, logical_size);

        let w = logical_size.width.min(760.0);
        let h = 360.0;
        let origin = Point::new((logical_size.width - w) * 0.5, 78.0);
        let body = rounded_rect(origin, Size::new(w, h), 24.0);
        frame.fill(&body, Color::from_rgb(0.50, 0.45, 0.36));
        frame.stroke(
            &body,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 0.95, 0.78, 0.55))
                .with_width(3.0),
        );

        let grille = rounded_rect(
            Point::new(origin.x + 52.0, origin.y + 62.0),
            Size::new(w - 104.0, h - 132.0),
            12.0,
        );
        frame.fill(&grille, Color::from_rgb(0.70, 0.64, 0.53));
        for y in 0..18 {
            let yy = origin.y + 78.0 + y as f32 * 10.0;
            frame.stroke(
                &Path::line(
                    Point::new(origin.x + 70.0, yy),
                    Point::new(origin.x + w - 70.0, yy),
                ),
                Stroke::default()
                    .with_color(Color::from_rgba(0.32, 0.25, 0.20, 0.28))
                    .with_width(2.0),
            );
        }
        draw_text(
            &mut frame,
            self.cab.model.title(),
            Point::new(origin.x + w * 0.5, origin.y + 34.0),
            30.0,
            Color::from_rgb(0.05, 0.04, 0.035),
            Horizontal::Center,
        );
        draw_text(
            &mut frame,
            "lab/references/tone3000-irs/celestion.wav",
            Point::new(origin.x + w * 0.5, origin.y + h - 42.0),
            18.0,
            Color::from_rgba(0.05, 0.04, 0.035, 0.72),
            Horizontal::Center,
        );
        vec![frame.into_geometry()]
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

fn unscale_point(point: Point, scale: f32) -> Point {
    let scale = scale.max(0.001);
    Point::new(point.x / scale, point.y / scale)
}

fn unscale_size(size: Size, scale: f32) -> Size {
    let scale = scale.max(0.001);
    Size::new(size.width / scale, size.height / scale)
}

fn hit_test_pedal_knob(
    devices: &[DeviceState],
    size: Size,
    position: Point,
) -> Option<(usize, ControlKind)> {
    let layout = board_layout(devices.len(), size);
    let y = 70.0;

    devices.iter().enumerate().find_map(|(index, device)| {
        let origin = Point::new(
            layout.start_x + index as f32 * (layout.pedal_w + layout.gap),
            y,
        );
        let size = Size::new(layout.pedal_w, layout.pedal_h);
        pedal_knob_centers(device, origin, size)
            .into_iter()
            .find(|(_, center)| distance(*center, position) <= 48.0)
            .map(|(control, _)| (index, control))
    })
}

fn pedal_knob_centers(
    device: &DeviceState,
    origin: Point,
    size: Size,
) -> Vec<(ControlKind, Point)> {
    let knob_y = origin.y + 76.0;

    match device.model {
        DeviceModel::Minotaur => vec![
            (
                ControlKind::Gain,
                Point::new(origin.x + size.width * 0.30, knob_y),
            ),
            (
                ControlKind::Treble,
                Point::new(origin.x + size.width * 0.70, knob_y),
            ),
            (
                ControlKind::Master,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
            ),
        ],
        DeviceModel::Springfield => vec![
            (
                ControlKind::Gain,
                Point::new(origin.x + size.width * 0.30, knob_y),
            ),
            (
                ControlKind::Treble,
                Point::new(origin.x + size.width * 0.70, knob_y),
            ),
            (
                ControlKind::Master,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
            ),
        ],
        _ => Vec::new(),
    }
}

fn hit_test_amp_knob(size: Size, position: Point) -> Option<ControlKind> {
    let amp_w = size.width.min(1080.0);
    let origin = Point::new((size.width - amp_w) * 0.5, 74.0);
    let knob_y = origin.y + 96.0;
    let first_knob_x = origin.x + amp_w * 0.38;
    let spacing = 120.0;
    let knobs = [
        (ControlKind::Gain, Point::new(first_knob_x, knob_y)),
        (
            ControlKind::Bass,
            Point::new(first_knob_x + spacing, knob_y),
        ),
        (
            ControlKind::Cut,
            Point::new(first_knob_x + spacing * 2.0, knob_y),
        ),
        (
            ControlKind::Master,
            Point::new(first_knob_x + spacing * 3.0, knob_y),
        ),
        (
            ControlKind::Treble,
            Point::new(first_knob_x + spacing * 4.0, knob_y),
        ),
    ];

    knobs
        .into_iter()
        .find(|(_, center)| distance(*center, position) <= 48.0)
        .map(|(control, _)| control)
}

fn dragged_value(gesture: DragGesture, position: Point) -> f32 {
    let horizontal = position.x - gesture.start_position.x;
    let vertical = gesture.start_position.y - position.y;
    (gesture.start_value + (horizontal + vertical) / 240.0).clamp(0.0, 1.0)
}

fn control_message(control: ControlKind, value: f32) -> Message {
    match control {
        ControlKind::Gain => Message::GainChanged(value),
        ControlKind::Bass => Message::BassChanged(value),
        ControlKind::Treble => Message::TrebleChanged(value),
        ControlKind::Cut => Message::CutChanged(value),
        ControlKind::Master => Message::MasterChanged(value),
    }
}

fn distance(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
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
        components::draw_knob(
            &mut frame,
            center,
            radius,
            KnobSpec {
                skin: KnobSkin::HeaderDial,
                ..KnobSpec::normalized(self.label, self.value)
            },
        );
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy)]
struct MeterArt {
    level: f32,
}

impl canvas::Program<Message> for MeterArt {
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
        let scale = (bounds.height / 132.0).max(0.001);
        frame.scale(scale);
        components::draw_vertical_meter(&mut frame, Point::new(3.0, 4.0), 124.0, self.level);
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

fn draw_amp_head(frame: &mut Frame, size: Size, amp: &DeviceState) {
    let amp_w = size.width.min(1080.0);
    let amp_h = 390.0;
    let origin = Point::new((size.width - amp_w) * 0.5, 74.0);

    let shadow = rounded_rect(
        Point::new(origin.x + 12.0, origin.y + 22.0),
        Size::new(amp_w, amp_h),
        38.0,
    );
    frame.fill(&shadow, Color::from_rgba(0.04, 0.04, 0.06, 0.28));

    let handle = Path::new(|p| {
        p.move_to(Point::new(origin.x + amp_w * 0.25, origin.y - 22.0));
        p.quadratic_curve_to(
            Point::new(origin.x + amp_w * 0.50, origin.y - 80.0),
            Point::new(origin.x + amp_w * 0.75, origin.y - 22.0),
        );
    });
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgb(0.07, 0.05, 0.08))
            .with_width(12.0),
    );
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgba(0.96, 0.70, 0.34, 0.52))
            .with_width(2.0),
    );

    let body = rounded_rect(origin, Size::new(amp_w, amp_h), 42.0);
    frame.fill(&body, Color::from_rgb(0.88, 0.67, 0.68));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.94, 0.98, 0.76))
            .with_width(5.0),
    );

    let panel = rounded_rect(
        Point::new(origin.x + 36.0, origin.y + 34.0),
        Size::new(amp_w - 72.0, 118.0),
        20.0,
    );
    frame.fill(&panel, Color::from_rgb(0.76, 0.53, 0.50));
    frame.stroke(
        &panel,
        Stroke::default()
            .with_color(Color::from_rgba(0.33, 0.16, 0.14, 0.30))
            .with_width(2.0),
    );

    let badge = rounded_rect(
        Point::new(origin.x + 70.0, origin.y + 58.0),
        Size::new(82.0, 72.0),
        10.0,
    );
    frame.fill(&badge, Color::from_rgba(1.0, 0.88, 0.90, 0.80));
    draw_text(
        frame,
        "GB",
        Point::new(origin.x + 111.0, origin.y + 96.0),
        26.0,
        Color::from_rgb(0.02, 0.02, 0.025),
        Horizontal::Center,
    );

    draw_text(
        frame,
        "INPUT",
        Point::new(origin.x + 235.0, origin.y + 62.0),
        13.0,
        Color::from_rgb(0.08, 0.06, 0.06),
        Horizontal::Center,
    );
    frame.fill(
        &Path::circle(Point::new(origin.x + 235.0, origin.y + 104.0), 18.0),
        GOLD,
    );
    frame.fill(
        &Path::circle(Point::new(origin.x + 235.0, origin.y + 104.0), 10.0),
        Color::from_rgb(0.04, 0.035, 0.05),
    );

    let knob_y = origin.y + 96.0;
    let first_knob_x = origin.x + amp_w * 0.38;
    let spacing = 120.0;
    draw_component_knob(
        frame,
        Point::new(first_knob_x, knob_y),
        30.0,
        "Volume",
        amp.gain,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing, knob_y),
        30.0,
        "Bass",
        amp.bass,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 2.0, knob_y),
        30.0,
        "Cut",
        amp.cut,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 3.0, knob_y),
        30.0,
        "Sag",
        amp.master,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 4.0, knob_y),
        30.0,
        "Treble",
        amp.treble,
        KnobSkin::AsatoBlack,
    );

    draw_text(
        frame,
        "POWER",
        Point::new(origin.x + amp_w - 112.0, origin.y + 58.0),
        14.0,
        Color::from_rgb(0.08, 0.06, 0.06),
        Horizontal::Center,
    );
    frame.fill(
        &Path::circle(Point::new(origin.x + amp_w - 112.0, origin.y + 104.0), 16.0),
        GOLD,
    );
    frame.stroke(
        &Path::line(
            Point::new(origin.x + amp_w - 112.0, origin.y + 88.0),
            Point::new(origin.x + amp_w - 112.0, origin.y + 114.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgb(0.05, 0.04, 0.05))
            .with_width(4.0),
    );
    frame.fill(
        &Path::circle(Point::new(origin.x + amp_w - 112.0, origin.y + 136.0), 11.0),
        if amp.bypassed {
            Color::from_rgb(0.09, 0.25, 0.25)
        } else {
            Color::from_rgb(0.0, 0.75, 0.78)
        },
    );

    let grille_origin = Point::new(origin.x + 92.0, origin.y + 188.0);
    let grille_size = Size::new(amp_w - 184.0, 150.0);
    let grille = rounded_rect(grille_origin, grille_size, 12.0);
    frame.fill(&grille, Color::from_rgb(0.91, 0.77, 0.77));
    frame.stroke(
        &grille,
        Stroke::default()
            .with_color(Color::from_rgba(0.32, 0.16, 0.16, 0.32))
            .with_width(2.0),
    );

    for row in 0..2 {
        for col in 0..6 {
            let cell_w = grille_size.width / 6.0;
            let cell_h = grille_size.height / 2.0;
            let x = grille_origin.x + col as f32 * cell_w + 12.0;
            let y = grille_origin.y + row as f32 * cell_h + 12.0;
            let motif = rounded_rect(
                Point::new(x, y),
                Size::new(cell_w - 24.0, cell_h - 24.0),
                4.0,
            );
            frame.stroke(
                &motif,
                Stroke::default()
                    .with_color(Color::from_rgba(0.58, 0.35, 0.34, 0.50))
                    .with_width(4.0),
            );
            frame.stroke(
                &Path::line(
                    Point::new(x + 14.0, y + cell_h * 0.42),
                    Point::new(x + cell_w - 38.0, y + cell_h * 0.42),
                ),
                Stroke::default()
                    .with_color(Color::from_rgba(0.58, 0.35, 0.34, 0.42))
                    .with_width(4.0),
            );
        }
    }

    draw_text(
        frame,
        "NOX30",
        Point::new(origin.x + amp_w - 112.0, origin.y + amp_h - 52.0),
        32.0,
        Color::from_rgb(0.04, 0.025, 0.03),
        Horizontal::Center,
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
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.30, knob_y),
                31.0,
                "Gain",
                device.gain,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.70, knob_y),
                31.0,
                "Treble",
                device.treble,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
                31.0,
                "Output",
                device.master,
                KnobSkin::Teal,
            );
        }
        DeviceModel::Nox30 => {
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y),
                31.0,
                "Volume",
                device.gain,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y),
                31.0,
                "Treble",
                device.treble,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y + 88.0),
                31.0,
                "Bass",
                device.bass,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y + 88.0),
                31.0,
                "Cut",
                device.cut,
                KnobSkin::AsatoBlack,
            );
        }
        DeviceModel::Springfield => {
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.30, knob_y),
                31.0,
                "Dwell",
                device.gain,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.70, knob_y),
                31.0,
                "Tone",
                device.treble,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + 92.0),
                31.0,
                "Mix",
                device.master,
                KnobSkin::Teal,
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

fn draw_component_knob(
    frame: &mut Frame,
    center: Point,
    radius: f32,
    label: &'static str,
    value: f32,
    skin: KnobSkin,
) {
    components::draw_knob(
        frame,
        center,
        radius,
        KnobSpec {
            label,
            value,
            min: 0.0,
            max: 1.0,
            step: 0.01,
            skin,
        },
    );
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
