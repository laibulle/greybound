pub mod components;

use components::{KnobSkin, KnobSpec};
use greybound::{
    device_circuit_descriptor, CircuitConfidence, CircuitDescriptor, CircuitDescriptorKind,
    CircuitNodeDescriptor, CircuitNodeKind, ComponentBoundary, DeviceConfig as CoreDeviceConfig,
    NOX30_COMPONENT_BOUNDARIES,
};
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
const PEDAL_STANDARD_WIDTH: f32 = 300.0;
const PEDAL_STANDARD_HEIGHT: f32 = 565.0;
const PEDAL_KNOB_RADIUS: f32 = 33.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Amp,
    Pedal,
    Cab,
}

struct TopIconButton;

impl button::StyleSheet for TopIconButton {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: INK,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            border_radius: 8.0.into(),
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

struct FooterContainer;

impl container::StyleSheet for FooterContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.80, 0.82, 0.88)),
            background: Some(Background::Color(Color::from_rgb(0.02, 0.025, 0.03))),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..container::Appearance::default()
        }
    }
}

fn footer_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(FooterContainer))
}

struct AppPanelContainer;

impl container::StyleSheet for AppPanelContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(INK),
            background: Some(Background::Color(PANEL)),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..container::Appearance::default()
        }
    }
}

fn app_panel_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(AppPanelContainer))
}

struct ControlBarContainer;

impl container::StyleSheet for ControlBarContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(INK),
            background: Some(Background::Color(Color::from_rgba(0.78, 0.83, 0.95, 0.84))),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..container::Appearance::default()
        }
    }
}

fn control_bar_container() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(ControlBarContainer))
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
            border_radius: [24.0, 24.0, 0.0, 0.0].into(),
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
    ToggleCircuitView,
    ToggleAmpImplementation,
    ToggleTuner,
    CloseTuner,
    ToggleTunerLive,
    ToggleTunerMute,
    TunerDisplayModeSelected(TunerDisplayMode),
    TunerReferenceStep(f32),
    TunerAnalysisChanged {
        frequency_hz: f32,
        cents: f32,
        confidence: f32,
    },
    ToggleAudioSettings,
    CloseAudioSettings,
    ToggleMetronome,
    CloseMetronome,
    ToggleMetronomePlayback,
    MetronomeBpmStep(f32),
    MetronomeVolumeChanged(f32),
    MetronomePanChanged(f32),
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
    TunerProbeTick(std::time::Instant),
    MeterLevelsChanged {
        input: f32,
        output_left: f32,
        output_right: f32,
    },
    ShutdownRequested,
    WindowResized {
        width: u32,
        height: u32,
    },
    ToggleDeviceBypass(usize),
    ToggleBypass(bool),
    SetDeviceControl {
        index: usize,
        control: ControlKind,
        value: f32,
    },
    SetGlobalControl {
        control: GlobalControl,
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
pub enum GlobalControl {
    Input,
    IrMix,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetronomeControl {
    Volume,
    Pan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunerDisplayMode {
    Cents,
    Hz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Pedals,
    Amp,
    Cab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmpImplementation {
    Stable,
    Experimental,
}

impl AmpImplementation {
    pub fn model_id(self) -> &'static str {
        match self {
            Self::Stable => "nox30",
            Self::Experimental => "nox30-experimental",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Experimental => "EXPERIMENT",
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
    pub input_gain: f32,
    pub output_gain: f32,
    pub meters: MeterLevels,
    pub audio_settings: AudioSettingsState,
    pub metronome: MetronomeState,
    pub tuner: TunerState,
    pub selected_index: usize,
    pub view_mode: ViewMode,
    pub circuit_view: bool,
    pub amp_implementation: AmpImplementation,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct TunerState {
    pub open: bool,
    pub live: bool,
    pub muted: bool,
    pub display_mode: TunerDisplayMode,
    pub reference_hz: f32,
    pub detected_hz: Option<f32>,
    pub note_name: Option<String>,
    pub cents: f32,
    pub confidence: f32,
}

impl Default for TunerState {
    fn default() -> Self {
        Self {
            open: false,
            live: true,
            muted: false,
            display_mode: TunerDisplayMode::Cents,
            reference_hz: 440.0,
            detected_hz: None,
            note_name: None,
            cents: 0.0,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetronomeState {
    pub open: bool,
    pub enabled: bool,
    pub bpm: f32,
    pub volume: f32,
    pub pan: f32,
}

impl Default for MetronomeState {
    fn default() -> Self {
        Self {
            open: false,
            enabled: false,
            bpm: 120.0,
            volume: 0.70,
            pan: 0.50,
        }
    }
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
    pub output_left: f32,
    pub output_right: f32,
}

impl Default for MeterLevels {
    fn default() -> Self {
        Self {
            input: 0.0,
            output_left: 0.0,
            output_right: 0.0,
        }
    }
}

impl Default for GreyboundUi {
    fn default() -> Self {
        Self {
            devices: vec![DeviceState::minotaur(), DeviceState::springfield()],
            amp: DeviceState::nox30(),
            cab: DeviceState::cab_ir(),
            input_gain: 0.50,
            output_gain: 0.58,
            meters: MeterLevels::default(),
            audio_settings: AudioSettingsState::default(),
            metronome: MetronomeState::default(),
            tuner: TunerState::default(),
            selected_index: 0,
            view_mode: ViewMode::Pedals,
            circuit_view: false,
            amp_implementation: AmpImplementation::Stable,
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
            Message::ToggleCircuitView => {
                self.circuit_view = !self.circuit_view;
            }
            Message::ToggleAmpImplementation => {
                self.amp_implementation = match self.amp_implementation {
                    AmpImplementation::Stable => AmpImplementation::Experimental,
                    AmpImplementation::Experimental => AmpImplementation::Stable,
                };
                self.audio_settings.status = "Restarting audio engine".to_string();
            }
            Message::ToggleTuner => {
                self.tuner.open = !self.tuner.open;
                if self.tuner.open {
                    self.audio_settings.open = false;
                    self.metronome.open = false;
                }
            }
            Message::CloseTuner => {
                self.tuner.open = false;
            }
            Message::ToggleTunerLive => {
                self.tuner.live = !self.tuner.live;
            }
            Message::ToggleTunerMute => {
                self.tuner.muted = !self.tuner.muted;
            }
            Message::TunerDisplayModeSelected(mode) => {
                self.tuner.display_mode = mode;
            }
            Message::TunerReferenceStep(delta) => {
                self.tuner.reference_hz = (self.tuner.reference_hz + delta).clamp(415.0, 466.0);
            }
            Message::TunerAnalysisChanged {
                frequency_hz,
                cents,
                confidence,
            } => {
                let confidence = confidence.clamp(0.0, 1.0);
                self.tuner.confidence = confidence;
                if frequency_hz > 0.0 && confidence > 0.0 {
                    self.tuner.detected_hz = Some(frequency_hz);
                    self.tuner.cents = cents.clamp(-50.0, 50.0);
                    self.tuner.note_name = Some(note_name(frequency_hz, self.tuner.reference_hz));
                } else if self.tuner.live {
                    self.tuner.detected_hz = None;
                    self.tuner.note_name = None;
                    self.tuner.cents = 0.0;
                }
            }
            Message::ToggleAudioSettings => {
                self.audio_settings.open = !self.audio_settings.open;
                if self.audio_settings.open {
                    self.metronome.open = false;
                    self.tuner.open = false;
                }
            }
            Message::CloseAudioSettings => {
                self.audio_settings.open = false;
            }
            Message::ToggleMetronome => {
                self.metronome.open = !self.metronome.open;
                if self.metronome.open {
                    self.audio_settings.open = false;
                    self.tuner.open = false;
                }
            }
            Message::CloseMetronome => {
                self.metronome.open = false;
            }
            Message::ToggleMetronomePlayback => {
                self.metronome.enabled = !self.metronome.enabled;
            }
            Message::MetronomeBpmStep(delta) => {
                self.metronome.bpm = (self.metronome.bpm + delta).clamp(30.0, 260.0);
            }
            Message::MetronomeVolumeChanged(value) => {
                self.metronome.volume = value.clamp(0.0, 1.0);
            }
            Message::MetronomePanChanged(value) => {
                self.metronome.pan = value.clamp(0.0, 1.0);
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
            Message::TunerProbeTick(_) => {}
            Message::MeterLevelsChanged {
                input,
                output_left,
                output_right,
            } => {
                self.meters.input = input.clamp(0.0, 1.0);
                self.meters.output_left = output_left.clamp(0.0, 1.0);
                self.meters.output_right = output_right.clamp(0.0, 1.0);
            }
            Message::ShutdownRequested => {}
            Message::WindowResized { width, height } => {
                self.scale = uniform_scale(width as f32, height as f32);
            }
            Message::ToggleBypass(value) => {
                self.active_device_mut().bypassed = value;
            }
            Message::ToggleDeviceBypass(index) => {
                if let Some(device) = self.devices.get_mut(index) {
                    device.bypassed = !device.bypassed;
                    self.selected_index = index;
                    self.view_mode = ViewMode::Pedals;
                }
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
            Message::SetGlobalControl { control, value } => {
                self.set_global_control(control, value);
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
        let scale = self.scale;

        let mode_tabs = container(
            row![
                self.view_button(ViewMode::Pedals),
                self.view_button(ViewMode::Amp),
                self.view_button(ViewMode::Cab),
            ]
            .spacing(self.s(10.0))
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x();

        let top = container(
            column![
                mode_tabs,
                row![
                    self.metered_global_knob(
                        "INPUT",
                        GlobalControl::Input,
                        self.input_gain,
                        normalized_db_readout(self.input_gain, -24.0, 24.0),
                        self.meters.input
                    ),
                    self.global_knob(
                        "IR MIX",
                        GlobalControl::IrMix,
                        self.cab.master,
                        percent_readout(self.cab.master)
                    ),
                    container(self.amp_implementation_switch())
                        .width(Length::Fill)
                        .center_x(),
                    self.output_metered_global_knob(
                        "OUTPUT",
                        GlobalControl::Output,
                        self.output_gain,
                        normalized_db_readout(self.output_gain, -24.0, 6.0),
                        self.meters.output_left,
                        self.meters.output_right
                    ),
                ]
                .spacing(self.s(20.0))
                .align_items(Alignment::Center),
            ]
            .spacing(self.s(2.0))
            .align_items(Alignment::Center),
        )
        .width(Length::Fixed(self.s(DESIGN_WIDTH)))
        .height(Length::Fixed(self.s(190.0)))
        .padding([self.s(4.0), self.s(34.0)])
        .style(control_bar_container());

        let main_view: Element<'_, Message> = if self.audio_settings.open {
            self.audio_settings_panel()
        } else if self.metronome.open {
            self.metronome_panel()
        } else if self.tuner.open {
            self.tuner_panel()
        } else {
            match self.view_mode {
                ViewMode::Pedals => Canvas::new(BoardArt {
                    devices: self.devices.clone(),
                    selected_index: self.selected_index,
                    circuit_view: self.circuit_view,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(666.0)))
                .into(),
                ViewMode::Amp => Canvas::new(AmpArt {
                    amp: self.amp.clone(),
                    circuit_view: self.circuit_view,
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
                button(text("TUNER").size(self.font(14.0)).style(Color::WHITE))
                    .on_press(Message::ToggleTuner)
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: self.tuner.open || self.tuner.muted
                    }))
                    .padding([self.s(4.0), self.s(10.0)]),
                text("MIDI").size(self.font(14.0)).style(bottom_text),
                text("TAP").size(self.font(14.0)).style(bottom_text),
                text(format!("{:.1} BPM", self.metronome.bpm))
                    .size(self.font(14.0))
                    .style(bottom_text),
                button(text("METRONOME").size(self.font(14.0)).style(Color::WHITE))
                    .on_press(Message::ToggleMetronome)
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: self.metronome.open || self.metronome.enabled
                    }))
                    .padding([self.s(4.0), self.s(10.0)]),
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
        .style(footer_container());

        let panel = container(column![top, main_view, bottom].spacing(0))
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(DESIGN_HEIGHT)))
            .style(app_panel_container());

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn view_button(&self, view_mode: ViewMode) -> Element<'_, Message> {
        button(
            Canvas::new(ViewIconArt {
                view_mode,
                selected: self.view_mode == view_mode,
                scale: self.scale,
            })
            .width(Length::Fixed(self.s(54.0)))
            .height(Length::Fixed(self.s(48.0))),
        )
        .on_press(Message::SelectView(view_mode))
        .style(iced::theme::Button::custom(TopIconButton))
        .padding(0)
        .into()
    }

    pub fn amp_model_id(&self) -> &'static str {
        self.amp_implementation.model_id()
    }

    fn amp_implementation_switch(&self) -> Element<'_, Message> {
        let label = format!("AMP {}", self.amp_implementation.label());
        button(text(label).size(self.font(13.0)).style(Color::WHITE))
            .on_press(Message::ToggleAmpImplementation)
            .style(iced::theme::Button::custom(FooterButton {
                selected: self.amp_implementation == AmpImplementation::Experimental,
            }))
            .padding([self.s(8.0), self.s(16.0)])
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

        let modal = self.modal_frame(
            "Audio Settings",
            Message::CloseAudioSettings,
            content.into(),
        );

        container(modal)
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(666.0)))
            .center_x()
            .center_y()
            .style(ghost_container(Color::from_rgba(0.04, 0.05, 0.08, 0.58)))
            .into()
    }

    fn metronome_panel(&self) -> Element<'_, Message> {
        let metro = &self.metronome;
        let secondary = Color::from_rgb(0.72, 0.72, 0.72);
        let play_label = if metro.enabled { "STOP" } else { "PLAY" };

        let content = column![
            row![
                self.metronome_knob(
                    "VOLUME",
                    MetronomeControl::Volume,
                    metro.volume,
                    metronome_volume_readout(metro.volume)
                ),
                self.settings_select_field(
                    "TIME SIGNATURE",
                    container(text("4/4").size(self.font(18.0)).style(Color::WHITE))
                        .padding([self.s(14.0), self.s(16.0)])
                        .width(Length::Fixed(self.s(250.0)))
                        .height(Length::Fixed(self.s(58.0)))
                        .style(dark_field_container())
                        .into(),
                    250.0
                ),
                self.settings_select_field(
                    "SOUND",
                    container(text("Blip").size(self.font(18.0)).style(Color::WHITE))
                        .padding([self.s(14.0), self.s(16.0)])
                        .width(Length::Fixed(self.s(250.0)))
                        .height(Length::Fixed(self.s(58.0)))
                        .style(dark_field_container())
                        .into(),
                    250.0
                ),
                self.metronome_knob(
                    "PAN",
                    MetronomeControl::Pan,
                    metro.pan,
                    metronome_pan_readout(metro.pan)
                ),
            ]
            .spacing(self.s(54.0))
            .align_items(Alignment::Center),
            self.settings_separator(),
            row![
                column![
                    text("BPM").size(self.font(14.0)).style(secondary),
                    row![
                        column![
                            button(text("+").size(self.font(18.0)).style(Color::WHITE))
                                .on_press(Message::MetronomeBpmStep(1.0))
                                .style(iced::theme::Button::custom(FooterButton {
                                    selected: false
                                }))
                                .padding([self.s(0.0), self.s(8.0)]),
                            button(text("-").size(self.font(18.0)).style(Color::WHITE))
                                .on_press(Message::MetronomeBpmStep(-1.0))
                                .style(iced::theme::Button::custom(FooterButton {
                                    selected: false
                                }))
                                .padding([self.s(0.0), self.s(8.0)]),
                        ]
                        .spacing(self.s(4.0)),
                        text(format!("{:.1}", metro.bpm))
                            .size(self.font(58.0))
                            .style(Color::WHITE),
                    ]
                    .spacing(self.s(18.0))
                    .align_items(Alignment::Center),
                ]
                .spacing(self.s(10.0)),
                button(text("TAP").size(self.font(18.0)).style(Color::WHITE))
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: false
                    }))
                    .padding([self.s(26.0), self.s(26.0)]),
                self.settings_select_field(
                    "RHYTHM",
                    container(text("1/4").size(self.font(18.0)).style(Color::WHITE))
                        .padding([self.s(14.0), self.s(16.0)])
                        .width(Length::Fixed(self.s(250.0)))
                        .height(Length::Fixed(self.s(58.0)))
                        .style(dark_field_container())
                        .into(),
                    250.0
                ),
                button(text(play_label).size(self.font(22.0)).style(Color::WHITE))
                    .on_press(Message::ToggleMetronomePlayback)
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: metro.enabled
                    }))
                    .padding([self.s(28.0), self.s(34.0)]),
            ]
            .spacing(self.s(70.0))
            .align_items(Alignment::Center),
        ]
        .spacing(self.s(38.0));

        let modal = self.modal_frame("Metronome", Message::CloseMetronome, content.into());

        container(modal)
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(666.0)))
            .center_x()
            .center_y()
            .style(ghost_container(Color::from_rgba(0.04, 0.05, 0.08, 0.58)))
            .into()
    }

    fn tuner_panel(&self) -> Element<'_, Message> {
        let tuner = &self.tuner;
        let mode_cents = tuner.display_mode == TunerDisplayMode::Cents;
        let reading = match (
            tuner.display_mode,
            tuner.detected_hz,
            tuner.note_name.as_deref(),
        ) {
            (TunerDisplayMode::Cents, Some(_), Some(note)) => {
                format!("{note} {:+.1} c", tuner.cents)
            }
            (TunerDisplayMode::Hz, Some(frequency), Some(note)) => {
                format!("{note} {:.1} Hz", frequency)
            }
            _ => "--".to_string(),
        };

        let content = column![
            Canvas::new(TunerArt {
                cents: tuner.cents,
                confidence: tuner.confidence,
                reading,
                scale: self.scale,
            })
            .width(Length::Fixed(self.s(1_060.0)))
            .height(Length::Fixed(self.s(330.0))),
            row![
                row![
                    button(text("Cents").size(self.font(16.0)).style(Color::WHITE))
                        .on_press(Message::TunerDisplayModeSelected(TunerDisplayMode::Cents))
                        .style(iced::theme::Button::custom(FooterButton {
                            selected: mode_cents
                        }))
                        .padding([self.s(14.0), self.s(18.0)]),
                    button(text("Hz").size(self.font(16.0)).style(Color::WHITE))
                        .on_press(Message::TunerDisplayModeSelected(TunerDisplayMode::Hz))
                        .style(iced::theme::Button::custom(FooterButton {
                            selected: !mode_cents
                        }))
                        .padding([self.s(14.0), self.s(18.0)]),
                ]
                .spacing(0),
                container("").width(Length::Fill),
                button(
                    text(if tuner.live { "LIVE" } else { "HOLD" })
                        .size(self.font(15.0))
                        .style(Color::WHITE)
                )
                .on_press(Message::ToggleTunerLive)
                .style(iced::theme::Button::custom(FooterButton {
                    selected: tuner.live
                }))
                .padding([self.s(12.0), self.s(24.0)]),
                text("Live Tuner")
                    .size(self.font(16.0))
                    .style(Color::from_rgb(0.82, 0.82, 0.82)),
                container("").width(Length::Fill),
                row![
                    text(format!("{:.1}", tuner.reference_hz))
                        .size(self.font(16.0))
                        .style(Color::WHITE),
                    column![
                        button(text("+").size(self.font(12.0)).style(Color::WHITE))
                            .on_press(Message::TunerReferenceStep(0.5))
                            .style(iced::theme::Button::custom(FooterButton {
                                selected: false
                            }))
                            .padding([self.s(0.0), self.s(8.0)]),
                        button(text("-").size(self.font(12.0)).style(Color::WHITE))
                            .on_press(Message::TunerReferenceStep(-0.5))
                            .style(iced::theme::Button::custom(FooterButton {
                                selected: false
                            }))
                            .padding([self.s(0.0), self.s(8.0)]),
                    ]
                    .spacing(self.s(2.0)),
                ]
                .spacing(self.s(8.0))
                .align_items(Alignment::Center),
            ]
            .spacing(self.s(22.0))
            .align_items(Alignment::Center),
        ]
        .spacing(self.s(28.0));

        let modal = self.tuner_modal_frame(content.into());

        container(modal)
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(666.0)))
            .center_x()
            .center_y()
            .style(ghost_container(Color::from_rgba(0.04, 0.05, 0.08, 0.58)))
            .into()
    }

    fn tuner_modal_frame<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        let mute_label = if self.tuner.muted { "MUTED" } else { "OUT" };
        let mute = button(text(mute_label).size(self.font(14.0)).style(Color::WHITE))
            .on_press(Message::ToggleTunerMute)
            .style(iced::theme::Button::custom(FooterButton {
                selected: self.tuner.muted,
            }))
            .padding([self.s(6.0), self.s(10.0)]);
        let close = button(text("X").size(self.font(26.0)).style(Color::WHITE))
            .on_press(Message::CloseTuner)
            .style(iced::theme::Button::custom(FooterButton {
                selected: false,
            }))
            .padding([self.s(2.0), self.s(8.0)]);

        container(
            column![
                container(
                    row![
                        container("").width(Length::Fixed(self.s(36.0))),
                        container(text("Tuner").size(self.font(24.0)).style(Color::WHITE))
                            .height(Length::Fill)
                            .center_y(),
                        container(row![mute, close].spacing(self.s(16.0)))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .align_x(Horizontal::Right)
                            .center_y(),
                        container("").width(Length::Fixed(self.s(16.0))),
                    ]
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                )
                .height(Length::Fixed(self.s(58.0)))
                .width(Length::Fill)
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
        .width(Length::Fixed(self.s(1_080.0)))
        .height(Length::Fixed(self.s(590.0)))
        .style(dark_container())
        .into()
    }

    fn modal_frame<'a>(
        &self,
        title: &'static str,
        close_message: Message,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let close = button(text("X").size(self.font(26.0)).style(Color::WHITE))
            .on_press(close_message)
            .style(iced::theme::Button::custom(FooterButton {
                selected: false,
            }))
            .padding([self.s(2.0), self.s(8.0)]);

        container(
            column![
                container(
                    row![
                        container("").width(Length::Fixed(self.s(36.0))),
                        container(text(title).size(self.font(24.0)).style(Color::WHITE))
                            .height(Length::Fill)
                            .center_y(),
                        container(close)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .align_x(Horizontal::Right)
                            .center_y(),
                        container("").width(Length::Fixed(self.s(16.0))),
                    ]
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                )
                .height(Length::Fixed(self.s(58.0)))
                .width(Length::Fill)
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

    fn metronome_knob(
        &self,
        label: &'static str,
        control: MetronomeControl,
        value: f32,
        readout: String,
    ) -> Element<'_, Message> {
        Canvas::new(MetronomeKnobArt {
            control,
            value,
            scale: self.scale,
            label,
            readout,
        })
        .width(Length::Fixed(self.s(136.0)))
        .height(Length::Fixed(self.s(154.0)))
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
        control: GlobalControl,
        value: f32,
        readout: String,
    ) -> Element<'_, Message> {
        container(
            Canvas::new(GlobalKnobArt {
                control,
                value,
                scale: self.scale,
                label,
                readout,
            })
            .width(Length::Fixed(self.s(128.0)))
            .height(Length::Fixed(self.s(132.0))),
        )
        .into()
    }

    fn metered_global_knob(
        &self,
        label: &'static str,
        control: GlobalControl,
        value: f32,
        readout: String,
        meter_level: f32,
    ) -> Element<'_, Message> {
        row![
            Canvas::new(MeterArt { level: meter_level })
                .width(Length::Fixed(self.s(18.0)))
                .height(Length::Fixed(self.s(132.0))),
            self.global_knob(label, control, value, readout),
        ]
        .spacing(self.s(12.0))
        .align_items(Alignment::Center)
        .into()
    }

    fn output_metered_global_knob(
        &self,
        label: &'static str,
        control: GlobalControl,
        value: f32,
        readout: String,
        left_level: f32,
        right_level: f32,
    ) -> Element<'_, Message> {
        row![
            self.global_knob(label, control, value, readout),
            row![
                Canvas::new(MeterArt { level: left_level })
                    .width(Length::Fixed(self.s(18.0)))
                    .height(Length::Fixed(self.s(132.0))),
                Canvas::new(MeterArt { level: right_level })
                    .width(Length::Fixed(self.s(18.0)))
                    .height(Length::Fixed(self.s(132.0))),
            ]
            .spacing(self.s(4.0))
            .align_items(Alignment::Center),
        ]
        .spacing(self.s(12.0))
        .align_items(Alignment::Center)
        .into()
    }

    fn set_global_control(&mut self, control: GlobalControl, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match control {
            GlobalControl::Input => self.input_gain = value,
            GlobalControl::IrMix => self.cab.master = value,
            GlobalControl::Output => self.output_gain = value,
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

fn percent_readout(value: f32) -> String {
    format!("{:.0}%", value.clamp(0.0, 1.0) * 100.0)
}

fn normalized_db(value: f32, min_db: f32, max_db: f32) -> f32 {
    min_db + value.clamp(0.0, 1.0) * (max_db - min_db)
}

pub fn normalized_gain(value: f32, min_db: f32, max_db: f32) -> f32 {
    10.0_f32.powf(normalized_db(value, min_db, max_db) / 20.0)
}

fn normalized_db_readout(value: f32, min_db: f32, max_db: f32) -> String {
    format!("{:.1} dB", normalized_db(value, min_db, max_db))
}

fn metronome_volume_readout(value: f32) -> String {
    if value <= 0.0 {
        "-inf dB".to_string()
    } else {
        format!("{:.1} dB", 20.0 * value.clamp(0.000_001, 1.0).log10())
    }
}

fn metronome_pan_readout(value: f32) -> String {
    let value = value.clamp(0.0, 1.0);
    if (value - 0.5).abs() < 0.04 {
        "C".to_string()
    } else if value < 0.5 {
        format!("L {:.0}", (0.5 - value) * 200.0)
    } else {
        format!("R {:.0}", (value - 0.5) * 200.0)
    }
}

fn note_name(frequency_hz: f32, reference_hz: f32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let midi = (69.0 + 12.0 * (frequency_hz / reference_hz).log2()).round() as i32;
    let name = NAMES[midi.rem_euclid(12) as usize];
    let octave = midi / 12 - 1;
    format!("{name}{octave}")
}

#[derive(Debug, Clone)]
struct BoardArt {
    devices: Vec<DeviceState>,
    selected_index: usize,
    circuit_view: bool,
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

                if hit_test_stage_circuit_toggle(unscale_size(bounds.size(), self.scale), position)
                {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::ToggleCircuitView),
                    );
                }

                if !self.circuit_view {
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

                    if let Some(index) = hit_test_pedal_footswitch(
                        self.devices.len(),
                        unscale_size(bounds.size(), self.scale),
                        position,
                    ) {
                        return (
                            canvas::event::Status::Captured,
                            Some(Message::ToggleDeviceBypass(index)),
                        );
                    }
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
        let y = pedal_board_y(logical_size, layout.pedal_h);

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
                self.circuit_view,
            );
        }

        draw_stage_circuit_toggle(&mut frame, logical_size, self.circuit_view);

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
    circuit_view: bool,
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

                if hit_test_stage_circuit_toggle(unscale_size(bounds.size(), self.scale), position)
                {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::ToggleCircuitView),
                    );
                }

                if self.circuit_view {
                    return (canvas::event::Status::Ignored, None);
                }

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
        if self.circuit_view {
            draw_amp_circuit(&mut frame, logical_size);
        } else {
            draw_amp_head(&mut frame, logical_size, &self.amp);
        }
        draw_stage_circuit_toggle(&mut frame, logical_size, self.circuit_view);
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
    let gap = 74.0;
    let available_width = size.width - 86.0 - gap * (count - 1.0);
    let pedal_w = (available_width / count)
        .min(PEDAL_STANDARD_WIDTH)
        .max(220.0);
    let pedal_h = (pedal_w * (PEDAL_STANDARD_HEIGHT / PEDAL_STANDARD_WIDTH))
        .min(size.height - 44.0)
        .max(420.0);
    let total = pedal_w * count + gap * (count - 1.0);

    BoardLayout {
        start_x: (size.width - total) * 0.5,
        gap,
        pedal_w,
        pedal_h,
    }
}

fn pedal_board_y(size: Size, pedal_h: f32) -> f32 {
    ((size.height - pedal_h) * 0.5 + 10.0).max(22.0)
}

fn hit_test_pedal(device_count: usize, size: Size, position: Point) -> Option<usize> {
    let layout = board_layout(device_count, size);
    let y = pedal_board_y(size, layout.pedal_h);

    (0..device_count).find(|index| {
        let x = layout.start_x + *index as f32 * (layout.pedal_w + layout.gap);
        position.x >= x
            && position.x <= x + layout.pedal_w
            && position.y >= y
            && position.y <= y + layout.pedal_h
    })
}

fn stage_circuit_toggle_center(size: Size) -> Point {
    Point::new(size.width - 54.0, 32.0)
}

fn hit_test_stage_circuit_toggle(size: Size, position: Point) -> bool {
    let center = stage_circuit_toggle_center(size);
    position.x >= center.x - 30.0
        && position.x <= center.x + 30.0
        && position.y >= center.y - 26.0
        && position.y <= center.y + 32.0
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
    let y = pedal_board_y(size, layout.pedal_h);

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

fn hit_test_pedal_footswitch(device_count: usize, size: Size, position: Point) -> Option<usize> {
    let layout = board_layout(device_count, size);
    let y = pedal_board_y(size, layout.pedal_h);

    (0..device_count).find(|index| {
        let origin = Point::new(
            layout.start_x + *index as f32 * (layout.pedal_w + layout.gap),
            y,
        );
        let center = Point::new(
            origin.x + layout.pedal_w * 0.50,
            origin.y + layout.pedal_h * 0.82,
        );
        distance(center, position) <= 50.0
    })
}

fn pedal_knob_centers(
    device: &DeviceState,
    origin: Point,
    size: Size,
) -> Vec<(ControlKind, Point)> {
    let knob_y = origin.y + size.height * 0.155;

    match device.model {
        DeviceModel::Minotaur => vec![
            (
                ControlKind::Gain,
                Point::new(origin.x + size.width * 0.25, knob_y),
            ),
            (
                ControlKind::Treble,
                Point::new(origin.x + size.width * 0.75, knob_y),
            ),
            (
                ControlKind::Master,
                Point::new(origin.x + size.width * 0.50, knob_y + size.height * 0.19),
            ),
        ],
        DeviceModel::Springfield => vec![
            (
                ControlKind::Gain,
                Point::new(origin.x + size.width * 0.25, knob_y),
            ),
            (
                ControlKind::Treble,
                Point::new(origin.x + size.width * 0.75, knob_y),
            ),
            (
                ControlKind::Master,
                Point::new(origin.x + size.width * 0.50, knob_y + size.height * 0.19),
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

fn metronome_control_message(control: MetronomeControl, value: f32) -> Message {
    match control {
        MetronomeControl::Volume => Message::MetronomeVolumeChanged(value),
        MetronomeControl::Pan => Message::MetronomePanChanged(value),
    }
}

fn distance(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[derive(Debug, Clone)]
struct ViewIconArt {
    view_mode: ViewMode,
    selected: bool,
    scale: f32,
}

impl canvas::Program<Message> for ViewIconArt {
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
        let center = Point::new(logical_size.width * 0.5, 20.0);
        let ink = Color::from_rgba(0.09, 0.12, 0.24, if self.selected { 1.0 } else { 0.84 });

        match self.view_mode {
            ViewMode::Pedals => draw_pedal_view_icon(&mut frame, center, ink),
            ViewMode::Amp => draw_amp_view_icon(&mut frame, center, ink),
            ViewMode::Cab => draw_cab_view_icon(&mut frame, center, ink),
        }

        if self.selected {
            frame.fill(
                &Path::circle(Point::new(logical_size.width * 0.5, 42.0), 2.8),
                ink,
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_pedal_view_icon(frame: &mut Frame, center: Point, color: Color) {
    let body = rounded_rect(
        Point::new(center.x - 13.0, center.y - 19.0),
        Size::new(26.0, 38.0),
        3.0,
    );
    frame.stroke(&body, Stroke::default().with_color(color).with_width(2.8));

    let wave = Path::new(|path| {
        path.move_to(Point::new(center.x - 8.0, center.y));
        path.line_to(Point::new(center.x - 4.5, center.y));
        path.line_to(Point::new(center.x - 3.0, center.y - 8.0));
        path.line_to(Point::new(center.x + 1.0, center.y + 8.0));
        path.line_to(Point::new(center.x + 3.5, center.y - 3.0));
        path.line_to(Point::new(center.x + 6.5, center.y));
        path.line_to(Point::new(center.x + 9.0, center.y));
    });
    frame.stroke(&wave, Stroke::default().with_color(color).with_width(2.8));
}

fn draw_amp_view_icon(frame: &mut Frame, center: Point, color: Color) {
    let body = rounded_rect(
        Point::new(center.x - 22.0, center.y - 10.0),
        Size::new(44.0, 20.0),
        2.0,
    );
    frame.stroke(&body, Stroke::default().with_color(color).with_width(2.8));
    frame.stroke(
        &Path::line(
            Point::new(center.x - 18.0, center.y),
            Point::new(center.x + 18.0, center.y),
        ),
        Stroke::default().with_color(color).with_width(2.8),
    );
}

fn draw_cab_view_icon(frame: &mut Frame, center: Point, color: Color) {
    frame.stroke(
        &Path::circle(center, 18.0),
        Stroke::default().with_color(color).with_width(2.8),
    );
    frame.stroke(
        &Path::circle(center, 5.6),
        Stroke::default().with_color(color).with_width(2.8),
    );
}

fn draw_stage_circuit_toggle(frame: &mut Frame, size: Size, selected: bool) {
    let center = stage_circuit_toggle_center(size);
    let ink = Color::from_rgba(0.09, 0.12, 0.24, if selected { 1.0 } else { 0.72 });

    draw_circuit_view_icon(frame, center, ink);

    if selected {
        frame.fill(
            &Path::circle(Point::new(center.x, center.y + 31.0), 2.8),
            ink,
        );
    }
}

fn draw_circuit_view_icon(frame: &mut Frame, center: Point, color: Color) {
    let chip = rounded_rect(
        Point::new(center.x - 15.0, center.y - 12.0),
        Size::new(30.0, 24.0),
        3.0,
    );
    frame.stroke(&chip, Stroke::default().with_color(color).with_width(2.6));

    for pin in 0..4 {
        let y = center.y - 8.0 + pin as f32 * 5.3;
        frame.stroke(
            &Path::line(
                Point::new(center.x - 21.0, y),
                Point::new(center.x - 15.0, y),
            ),
            Stroke::default().with_color(color).with_width(2.0),
        );
        frame.stroke(
            &Path::line(
                Point::new(center.x + 15.0, y),
                Point::new(center.x + 21.0, y),
            ),
            Stroke::default().with_color(color).with_width(2.0),
        );
    }

    frame.stroke(
        &Path::line(
            Point::new(center.x - 4.0, center.y + 12.0),
            Point::new(center.x - 4.0, center.y + 20.0),
        ),
        Stroke::default().with_color(color).with_width(2.0),
    );
    frame.stroke(
        &Path::line(
            Point::new(center.x + 7.0, center.y - 12.0),
            Point::new(center.x + 7.0, center.y - 20.0),
        ),
        Stroke::default().with_color(color).with_width(2.0),
    );
}

#[derive(Debug, Clone)]
struct GlobalKnobArt {
    control: GlobalControl,
    value: f32,
    scale: f32,
    label: &'static str,
    readout: String,
}

impl canvas::Program<Message> for GlobalKnobArt {
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
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                state.gesture = Some(DragGesture {
                    index: None,
                    control: ControlKind::Master,
                    start_position: position,
                    start_value: self.value,
                });
                (
                    canvas::event::Status::Captured,
                    Some(Message::SetGlobalControl {
                        control: self.control,
                        value: self.value,
                    }),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(gesture) = state.gesture else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(position) = cursor
                    .position_from(bounds.position())
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                (
                    canvas::event::Status::Captured,
                    Some(Message::SetGlobalControl {
                        control: self.control,
                        value: dragged_value(gesture, position),
                    }),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.gesture = None;
                (canvas::event::Status::Captured, None)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
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
        let radius = 27.0;
        let center = Point::new(logical_size.width * 0.5, 68.0);
        draw_text(
            &mut frame,
            self.label,
            Point::new(logical_size.width * 0.5, 10.0),
            14.0,
            INK,
            Horizontal::Center,
        );
        components::draw_knob(
            &mut frame,
            center,
            radius,
            KnobSpec {
                skin: KnobSkin::HeaderDial,
                ..KnobSpec::normalized("", self.value)
            },
        );
        draw_text(
            &mut frame,
            &self.readout,
            Point::new(logical_size.width * 0.5, 126.0),
            14.0,
            INK,
            Horizontal::Center,
        );
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

struct MetronomeKnobArt {
    control: MetronomeControl,
    value: f32,
    scale: f32,
    label: &'static str,
    readout: String,
}

impl canvas::Program<Message> for MetronomeKnobArt {
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
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                state.gesture = Some(DragGesture {
                    index: None,
                    control: ControlKind::Master,
                    start_position: position,
                    start_value: self.value,
                });
                (
                    canvas::event::Status::Captured,
                    Some(metronome_control_message(self.control, self.value)),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(gesture) = state.gesture else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(position) = cursor
                    .position_from(bounds.position())
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                (
                    canvas::event::Status::Captured,
                    Some(metronome_control_message(
                        self.control,
                        dragged_value(gesture, position),
                    )),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.gesture = None;
                (canvas::event::Status::Captured, None)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
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
        let radius = 35.0;
        let center = Point::new(logical_size.width * 0.5, 70.0);
        draw_text(
            &mut frame,
            self.label,
            Point::new(logical_size.width * 0.5, 13.0),
            14.0,
            Color::from_rgb(0.72, 0.72, 0.72),
            Horizontal::Center,
        );
        components::draw_knob(
            &mut frame,
            center,
            radius,
            KnobSpec {
                skin: KnobSkin::HeaderDial,
                ..KnobSpec::normalized("", self.value)
            },
        );
        draw_text(
            &mut frame,
            &self.readout,
            Point::new(logical_size.width * 0.5, 142.0),
            16.0,
            Color::WHITE,
            Horizontal::Center,
        );
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

struct TunerArt {
    cents: f32,
    confidence: f32,
    reading: String,
    scale: f32,
}

impl canvas::Program<Message> for TunerArt {
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
        let center_y = logical_size.height * 0.48;
        let left = 56.0;
        let right = logical_size.width - 56.0;
        let center_x = (left + right) * 0.5;

        draw_text(
            &mut frame,
            "-50",
            Point::new(left + 12.0, 54.0),
            18.0,
            Color::from_rgb(0.66, 0.66, 0.66),
            Horizontal::Left,
        );
        draw_text(
            &mut frame,
            "+50",
            Point::new(right - 12.0, 54.0),
            18.0,
            Color::from_rgb(0.66, 0.66, 0.66),
            Horizontal::Right,
        );

        frame.stroke(
            &Path::line(Point::new(left, center_y), Point::new(right, center_y)),
            Stroke::default().with_color(Color::WHITE).with_width(3.0),
        );

        let confidence = self.confidence.clamp(0.0, 1.0);
        let visible = confidence > 0.05;
        let cents = if visible {
            self.cents.clamp(-50.0, 50.0)
        } else {
            0.0
        };
        let ball_x = center_x + (cents / 50.0) * ((right - left) * 0.5);
        let in_tune = cents.abs() <= 3.0 && confidence > 0.55;
        let target = Path::circle(Point::new(center_x, center_y), 54.0);
        frame.fill(&target, Color::from_rgb(0.075, 0.075, 0.075));
        frame.stroke(
            &target,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.45 + 0.55 * confidence))
                .with_width(4.0),
        );

        if visible {
            let ball_color = if in_tune {
                Color::from_rgb(0.22, 0.95, 0.36)
            } else {
                Color::from_rgb(1.0, 0.16, 0.16)
            };
            let ball = Path::circle(Point::new(ball_x, center_y), 34.0);
            frame.fill(&ball, ball_color);

            if !in_tune {
                let direction = if cents < 0.0 { ">" } else { "<" };
                let arrow_x = if cents < 0.0 {
                    (ball_x + 84.0).min(center_x - 70.0)
                } else {
                    (ball_x - 84.0).max(center_x + 70.0)
                };
                draw_text(
                    &mut frame,
                    direction,
                    Point::new(arrow_x, 54.0),
                    32.0,
                    ball_color,
                    Horizontal::Center,
                );
            }
        }

        draw_text(
            &mut frame,
            &self.reading,
            Point::new(center_x, center_y + 96.0),
            26.0,
            Color::WHITE,
            Horizontal::Center,
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

fn draw_amp_circuit(frame: &mut Frame, size: Size) {
    let panel_w = size.width.min(1120.0);
    let panel_h = 500.0;
    let origin = Point::new((size.width - panel_w) * 0.5, 62.0);

    let shadow = rounded_rect(
        Point::new(origin.x + 12.0, origin.y + 18.0),
        Size::new(panel_w, panel_h),
        22.0,
    );
    frame.fill(&shadow, Color::from_rgba(0.04, 0.04, 0.06, 0.28));

    let board = rounded_rect(origin, Size::new(panel_w, panel_h), 22.0);
    frame.fill(&board, Color::from_rgb(0.055, 0.070, 0.065));
    frame.stroke(
        &board,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.80, 0.48, 0.46))
            .with_width(2.0),
    );
    draw_circuit_backplate(frame, origin, Size::new(panel_w, panel_h));

    draw_text(
        frame,
        "Nox30 circuit-informed amp",
        Point::new(origin.x + 34.0, origin.y + 34.0),
        22.0,
        Color::from_rgb(0.90, 0.84, 0.68),
        Horizontal::Left,
    );
    draw_text(
        frame,
        "input / preamp / top boost / phase inverter / EL84 power / transformer",
        Point::new(origin.x + 34.0, origin.y + 62.0),
        13.0,
        Color::from_rgba(0.83, 0.78, 0.66, 0.78),
        Horizontal::Left,
    );
    draw_circuit_kind_badge(
        frame,
        Point::new(origin.x + panel_w - 92.0, origin.y + 42.0),
        CircuitDescriptorKind::CircuitInformed,
    );

    let graph_origin = Point::new(origin.x + 44.0, origin.y + 116.0);
    let graph_size = Size::new(panel_w - 88.0, panel_h - 178.0);
    let signal_ids = [
        "input_volume",
        "first_stage",
        "cathode_follower",
        "tone_stack",
        "drive_stage",
        "recovery_stage",
        "phase_inverter",
        "power_stage",
        "output_transformer",
    ];
    let signal_points: Vec<Point> = signal_ids
        .iter()
        .enumerate()
        .map(|(slot, _)| {
            let x = if signal_ids.len() <= 1 {
                0.5
            } else {
                slot as f32 / (signal_ids.len() - 1) as f32
            };
            Point::new(
                graph_origin.x + graph_size.width * x,
                graph_origin.y + graph_size.height * 0.50,
            )
        })
        .collect();

    for points in signal_points.windows(2) {
        draw_amp_circuit_edge(frame, points[0], points[1]);
    }

    for (id, point) in signal_ids.iter().zip(signal_points.iter()) {
        if let Some(boundary) = nox30_boundary(id) {
            draw_amp_circuit_stage(frame, *point, boundary);
        }
    }

    if let Some(cut_presence) = nox30_boundary("cut_presence") {
        let point = Point::new(
            (signal_points[6].x + signal_points[7].x) * 0.5,
            graph_origin.y + graph_size.height * 0.22,
        );
        draw_amp_circuit_stage(frame, point, cut_presence);
        draw_amp_circuit_edge(frame, signal_points[6], point);
        draw_amp_circuit_edge(frame, point, signal_points[7]);
    }

    if let Some(supply) = nox30_boundary("supply_network") {
        let point = Point::new(
            graph_origin.x + graph_size.width * 0.58,
            graph_origin.y + graph_size.height * 0.83,
        );
        draw_amp_circuit_stage(frame, point, supply);
        for target in [1usize, 4, 7] {
            draw_amp_supply_drop(frame, point, signal_points[target]);
        }
    }

    draw_text(
        frame,
        "component boundaries, not a complete PCB or SPICE netlist",
        Point::new(origin.x + panel_w * 0.5, origin.y + panel_h - 30.0),
        13.0,
        Color::from_rgba(0.84, 0.76, 0.58, 0.72),
        Horizontal::Center,
    );
}

fn nox30_boundary(id: &str) -> Option<&'static ComponentBoundary> {
    NOX30_COMPONENT_BOUNDARIES
        .iter()
        .find(|boundary| boundary.id == id)
}

fn draw_amp_circuit_edge(frame: &mut Frame, from: Point, to: Point) {
    frame.stroke(
        &Path::line(from, to),
        Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.58, 0.25, 0.72))
            .with_width(3.0),
    );
    frame.stroke(
        &Path::line(from, to),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.86, 0.48, 0.18))
            .with_width(1.0),
    );
}

fn draw_amp_supply_drop(frame: &mut Frame, from: Point, to: Point) {
    let path = Path::new(|p| {
        p.move_to(from);
        p.line_to(Point::new(to.x, from.y));
        p.line_to(to);
    });
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgba(0.45, 0.78, 0.95, 0.34))
            .with_width(1.8),
    );
}

fn draw_amp_circuit_stage(frame: &mut Frame, center: Point, boundary: &ComponentBoundary) {
    let (label, detail, width, color) = match boundary.id {
        "input_volume" => (
            "Input",
            "volume / bright",
            84.0,
            Color::from_rgb(0.13, 0.22, 0.23),
        ),
        "first_stage" => ("V1", "ECC83 gain", 74.0, Color::from_rgb(0.30, 0.20, 0.13)),
        "cathode_follower" => ("CF", "tone driver", 82.0, Color::from_rgb(0.25, 0.20, 0.13)),
        "tone_stack" => ("Tone", "top boost", 86.0, Color::from_rgb(0.18, 0.24, 0.19)),
        "drive_stage" => ("Drive", "ECC83", 76.0, Color::from_rgb(0.30, 0.20, 0.13)),
        "recovery_stage" => ("Recover", "ECC83", 88.0, Color::from_rgb(0.30, 0.20, 0.13)),
        "phase_inverter" => ("PI", "long-tail", 74.0, Color::from_rgb(0.20, 0.22, 0.18)),
        "cut_presence" => ("Cut", "presence", 72.0, Color::from_rgb(0.18, 0.24, 0.19)),
        "power_stage" => ("EL84", "push-pull", 82.0, Color::from_rgb(0.34, 0.18, 0.12)),
        "supply_network" => ("B+", "sag rail", 78.0, Color::from_rgb(0.13, 0.22, 0.28)),
        "output_transformer" => ("OT", "flux", 66.0, Color::from_rgb(0.13, 0.22, 0.28)),
        _ => (
            boundary.id,
            boundary.role,
            86.0,
            Color::from_rgb(0.16, 0.23, 0.18),
        ),
    };
    let height = 50.0;
    let body = rounded_rect(
        Point::new(center.x - width * 0.5, center.y - height * 0.5),
        Size::new(width, height),
        9.0,
    );
    frame.fill(&body, color);
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(0.96, 0.78, 0.44, 0.52))
            .with_width(1.6),
    );
    draw_text(
        frame,
        label,
        Point::new(center.x, center.y - 7.0),
        13.0,
        Color::from_rgb(0.96, 0.92, 0.78),
        Horizontal::Center,
    );
    draw_text(
        frame,
        detail,
        Point::new(center.x, center.y + 11.0),
        9.0,
        Color::from_rgba(0.86, 0.82, 0.70, 0.72),
        Horizontal::Center,
    );
}

fn draw_pedal(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    device: &DeviceState,
    color: Color,
    _selected: bool,
    circuit_view: bool,
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
            .with_color(Color::from_rgba(0.28, 0.22, 0.15, 0.32))
            .with_width(1.3),
    );

    let inner = rounded_rect(origin, Size::new(size.width, size.height - 18.0), 18.0);
    frame.fill(&inner, lighten(color, 0.10));
    draw_pedal_surface_reflections(frame, origin, Size::new(size.width, size.height - 18.0));
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

    if circuit_view {
        draw_pedal_circuit(frame, origin, size, device);
        return;
    }

    let knob_y = origin.y + size.height * 0.155;
    match device.model {
        DeviceModel::Minotaur => {
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.25, knob_y),
                PEDAL_KNOB_RADIUS,
                "Gain",
                device.gain,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.75, knob_y),
                PEDAL_KNOB_RADIUS,
                "Treble",
                device.treble,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + size.height * 0.19),
                PEDAL_KNOB_RADIUS,
                "Output",
                device.master,
                KnobSkin::Teal,
            );
        }
        DeviceModel::Nox30 => {
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y),
                PEDAL_KNOB_RADIUS,
                "Volume",
                device.gain,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y),
                PEDAL_KNOB_RADIUS,
                "Treble",
                device.treble,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.28, knob_y + size.height * 0.155),
                PEDAL_KNOB_RADIUS,
                "Bass",
                device.bass,
                KnobSkin::AsatoBlack,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.72, knob_y + size.height * 0.155),
                PEDAL_KNOB_RADIUS,
                "Cut",
                device.cut,
                KnobSkin::AsatoBlack,
            );
        }
        DeviceModel::Springfield => {
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.25, knob_y),
                PEDAL_KNOB_RADIUS,
                "Dwell",
                device.gain,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.75, knob_y),
                PEDAL_KNOB_RADIUS,
                "Tone",
                device.treble,
                KnobSkin::Teal,
            );
            draw_component_knob(
                frame,
                Point::new(origin.x + size.width * 0.50, knob_y + size.height * 0.19),
                PEDAL_KNOB_RADIUS,
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

    let title_y = origin.y + size.height * 0.56;
    draw_text(
        frame,
        &device.name,
        Point::new(origin.x + size.width * 0.50, title_y + 1.0),
        27.0,
        Color::from_rgb(0.02, 0.025, 0.03),
        Horizontal::Center,
    );

    let plate_origin = Point::new(origin.x + 20.0, origin.y + size.height * 0.64);
    let plate_size = Size::new(size.width - 40.0, size.height * 0.31);
    draw_texture_plate(frame, plate_origin, plate_size, device.name.as_str());

    draw_status_led(
        frame,
        Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.69),
        11.0,
        !device.bypassed,
    );

    draw_footswitch(
        frame,
        Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.82),
    );
}

fn draw_status_led(frame: &mut Frame, center: Point, radius: f32, active: bool) {
    frame.fill(
        &Path::circle(Point::new(center.x + 1.8, center.y + 2.4), radius + 4.0),
        Color::from_rgba(0.02, 0.03, 0.03, 0.42),
    );

    frame.fill(
        &Path::circle(center, radius + 3.2),
        Color::from_rgb(0.74, 0.77, 0.70),
    );
    frame.stroke(
        &Path::circle(center, radius + 3.2),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.98, 0.88, 0.55))
            .with_width(1.8),
    );

    if active {
        frame.fill(
            &Path::circle(center, radius + 7.0),
            Color::from_rgba(0.0, 0.92, 0.95, 0.16),
        );
        frame.fill(
            &Path::circle(center, radius + 3.8),
            Color::from_rgba(0.0, 0.94, 0.95, 0.22),
        );
    }

    let lens = if active {
        Color::from_rgb(0.00, 0.78, 0.80)
    } else {
        Color::from_rgb(0.08, 0.27, 0.27)
    };
    frame.fill(&Path::circle(center, radius), lens);

    if active {
        for (idx, alpha) in [0.42, 0.28, 0.18].into_iter().enumerate() {
            frame.stroke(
                &Path::circle(center, radius - 2.2 - idx as f32 * 2.2),
                Stroke::default()
                    .with_color(Color::from_rgba(0.82, 1.0, 0.98, alpha))
                    .with_width(1.2),
            );
        }
        frame.fill(
            &Path::circle(
                Point::new(center.x - radius * 0.25, center.y - radius * 0.30),
                4.0,
            ),
            Color::from_rgba(0.96, 1.0, 1.0, 0.72),
        );
        frame.fill(
            &Path::circle(
                Point::new(center.x + radius * 0.22, center.y - radius * 0.10),
                2.2,
            ),
            Color::from_rgba(0.92, 1.0, 1.0, 0.45),
        );
    } else {
        frame.fill(
            &Path::circle(
                Point::new(center.x - radius * 0.20, center.y - radius * 0.25),
                3.5,
            ),
            Color::from_rgba(0.62, 0.84, 0.82, 0.16),
        );
    }

    frame.stroke(
        &Path::circle(center, radius),
        Stroke::default()
            .with_color(Color::from_rgba(
                0.92,
                1.0,
                0.96,
                if active { 0.72 } else { 0.24 },
            ))
            .with_width(1.8),
    );
}

fn draw_pedal_circuit(frame: &mut Frame, origin: Point, size: Size, device: &DeviceState) {
    let Some(descriptor) = ui_circuit_descriptor(device.model) else {
        return;
    };

    let board_origin = Point::new(origin.x + 18.0, origin.y + 40.0);
    let board_size = Size::new(size.width - 36.0, size.height - 78.0);
    let board = rounded_rect(board_origin, board_size, 14.0);
    frame.fill(&board, Color::from_rgb(0.055, 0.070, 0.065));
    frame.stroke(
        &board,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.80, 0.48, 0.42))
            .with_width(1.8),
    );

    draw_circuit_backplate(frame, board_origin, board_size);

    let graph_origin = Point::new(board_origin.x + 14.0, board_origin.y + 52.0);
    let graph_size = Size::new(board_size.width - 28.0, board_size.height - 110.0);
    let layout = layout_circuit_graph(descriptor, graph_origin, graph_size);

    for edge in descriptor.edges {
        if let (Some(from), Some(to)) = (
            circuit_placement_by_id(&layout.placements, edge.from),
            circuit_placement_by_id(&layout.placements, edge.to),
        ) {
            draw_semantic_circuit_edge(frame, from, to, layout.direction);
        }
    }

    for placement in &layout.placements {
        draw_semantic_circuit_node(frame, placement.point, placement.node);
    }

    draw_text(
        frame,
        descriptor.label,
        Point::new(board_origin.x + 18.0, board_origin.y + 22.0),
        14.0,
        Color::from_rgb(0.90, 0.84, 0.68),
        Horizontal::Left,
    );
    draw_text(
        frame,
        circuit_descriptor_summary(descriptor),
        Point::new(board_origin.x + 18.0, board_origin.y + 40.0),
        9.5,
        Color::from_rgba(0.83, 0.78, 0.66, 0.78),
        Horizontal::Left,
    );
    draw_circuit_kind_badge(
        frame,
        Point::new(
            board_origin.x + board_size.width - 62.0,
            board_origin.y + 27.0,
        ),
        descriptor.kind,
    );

    draw_text(
        frame,
        "model-level circuit, not PCB artwork",
        Point::new(
            board_origin.x + board_size.width * 0.5,
            board_origin.y + board_size.height - 20.0,
        ),
        9.5,
        Color::from_rgba(0.84, 0.76, 0.58, 0.72),
        Horizontal::Center,
    );
}

fn ui_circuit_descriptor(model: DeviceModel) -> Option<&'static CircuitDescriptor> {
    match model {
        DeviceModel::Minotaur => device_circuit_descriptor(CoreDeviceConfig::Minotaur),
        DeviceModel::Springfield => device_circuit_descriptor(CoreDeviceConfig::Springfield),
        _ => None,
    }
}

fn circuit_descriptor_summary(descriptor: &CircuitDescriptor) -> &'static str {
    match descriptor.model_id {
        "minotaur" => "buffer / clean blend / clip / presence / output",
        "springfield" => "buffer / dwell driver / spring IR / recovery / mix",
        _ => descriptor.summary,
    }
}

fn draw_circuit_backplate(frame: &mut Frame, origin: Point, size: Size) {
    for row in 0..8 {
        let y = origin.y + 64.0 + row as f32 * (size.height - 120.0) / 7.0;
        frame.stroke(
            &Path::line(
                Point::new(origin.x + 12.0, y),
                Point::new(origin.x + size.width - 12.0, y),
            ),
            Stroke::default()
                .with_color(Color::from_rgba(0.72, 0.60, 0.36, 0.055))
                .with_width(1.0),
        );
    }

    for col in 0..5 {
        let x = origin.x + 18.0 + col as f32 * (size.width - 36.0) / 4.0;
        frame.stroke(
            &Path::line(
                Point::new(x, origin.y + 60.0),
                Point::new(x, origin.y + size.height - 46.0),
            ),
            Stroke::default()
                .with_color(Color::from_rgba(0.72, 0.60, 0.36, 0.035))
                .with_width(1.0),
        );
    }
}

#[derive(Clone, Copy)]
enum CircuitLayoutDirection {
    LeftToRight,
    TopToBottom,
}

struct CircuitGraphLayout<'a> {
    direction: CircuitLayoutDirection,
    placements: Vec<CircuitNodePlacement<'a>>,
}

struct CircuitNodePlacement<'a> {
    id: &'static str,
    node: &'a CircuitNodeDescriptor,
    point: Point,
}

fn layout_circuit_graph<'a>(
    descriptor: &'a CircuitDescriptor,
    origin: Point,
    size: Size,
) -> CircuitGraphLayout<'a> {
    let direction = if size.height > size.width * 1.20 {
        CircuitLayoutDirection::TopToBottom
    } else {
        CircuitLayoutDirection::LeftToRight
    };

    if matches!(direction, CircuitLayoutDirection::TopToBottom) {
        if let Some(layout) = layout_series_parallel_circuit(descriptor, origin, size) {
            return layout;
        }
    }

    let node_count = descriptor.nodes.len();
    let mut outgoing = vec![Vec::<usize>::new(); node_count];
    let mut incoming = vec![Vec::<usize>::new(); node_count];
    let mut indegree = vec![0usize; node_count];

    for edge in descriptor.edges {
        if let (Some(from), Some(to)) = (
            circuit_node_index(descriptor, edge.from),
            circuit_node_index(descriptor, edge.to),
        ) {
            outgoing[from].push(to);
            incoming[to].push(from);
            indegree[to] += 1;
        }
    }

    let mut layers = vec![0usize; node_count];
    let mut queue: Vec<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut visited = 0usize;
    let mut cursor = 0usize;

    while cursor < queue.len() {
        let current = queue[cursor];
        cursor += 1;
        visited += 1;

        for &next in &outgoing[current] {
            layers[next] = layers[next].max(layers[current] + 1);
            indegree[next] = indegree[next].saturating_sub(1);
            if indegree[next] == 0 {
                queue.push(next);
            }
        }
    }

    if visited != node_count {
        return descriptor_layout_fallback(descriptor, origin, size, direction);
    }

    let max_layer = layers.iter().copied().max().unwrap_or(0);
    let mut layer_nodes = vec![Vec::<usize>::new(); max_layer + 1];
    for (index, &layer) in layers.iter().enumerate() {
        layer_nodes[layer].push(index);
    }

    for nodes in &mut layer_nodes {
        nodes.sort_by(|&a, &b| {
            initial_layer_order(&descriptor.nodes[a], direction)
                .partial_cmp(&initial_layer_order(&descriptor.nodes[b], direction))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(&b))
        });
    }

    let mut order = circuit_order_map(&layer_nodes, node_count);
    for _ in 0..4 {
        for layer in 1..layer_nodes.len() {
            sort_layer_by_barycenter(&mut layer_nodes[layer], &incoming, &order);
            order = circuit_order_map(&layer_nodes, node_count);
        }
        for layer in (0..layer_nodes.len().saturating_sub(1)).rev() {
            sort_layer_by_barycenter(&mut layer_nodes[layer], &outgoing, &order);
            order = circuit_order_map(&layer_nodes, node_count);
        }
    }

    let x_pad = 28.0;
    let y_pad = 22.0;
    let usable_width = (size.width - x_pad * 2.0).max(1.0);
    let usable_height = (size.height - y_pad * 2.0).max(1.0);
    let layer_divisor = max_layer.max(1) as f32;
    let mut placements = Vec::with_capacity(node_count);

    for (layer, nodes) in layer_nodes.iter().enumerate() {
        let layer_position = layer as f32 / layer_divisor;
        let cross_positions = layer_cross_positions(nodes, descriptor, direction);
        for (slot, &node_index) in nodes.iter().enumerate() {
            let cross = cross_positions[slot];
            let point = match direction {
                CircuitLayoutDirection::TopToBottom => Point::new(
                    origin.x + x_pad + usable_width * cross,
                    origin.y + y_pad + usable_height * layer_position,
                ),
                CircuitLayoutDirection::LeftToRight => Point::new(
                    origin.x + x_pad + usable_width * layer_position,
                    origin.y + y_pad + usable_height * cross,
                ),
            };
            placements.push(CircuitNodePlacement {
                id: descriptor.nodes[node_index].id,
                node: &descriptor.nodes[node_index],
                point,
            });
        }
    }

    CircuitGraphLayout {
        direction,
        placements,
    }
}

fn layout_series_parallel_circuit<'a>(
    descriptor: &'a CircuitDescriptor,
    origin: Point,
    size: Size,
) -> Option<CircuitGraphLayout<'a>> {
    let node_count = descriptor.nodes.len();
    let mut outgoing = vec![Vec::<usize>::new(); node_count];
    let mut incoming = vec![Vec::<usize>::new(); node_count];

    for edge in descriptor.edges {
        let from = circuit_node_index(descriptor, edge.from)?;
        let to = circuit_node_index(descriptor, edge.to)?;
        outgoing[from].push(to);
        incoming[to].push(from);
    }

    let split = (0..node_count).find(|&index| outgoing[index].len() > 1)?;
    let merge = (0..node_count)
        .filter(|&index| incoming[index].len() > 1)
        .find(|&candidate| {
            outgoing[split]
                .iter()
                .all(|&branch| branch == candidate || reaches(branch, candidate, &outgoing))
        })?;

    let root = (0..node_count)
        .find(|&index| incoming[index].is_empty() && reaches(index, split, &outgoing))?;
    let pre_split = path_between(root, split, &outgoing)?;
    let post_merge = follow_single_output_path(merge, &outgoing);
    let branch_paths: Vec<Vec<usize>> = outgoing[split]
        .iter()
        .map(|&branch| {
            if branch == merge {
                Some(Vec::new())
            } else {
                path_between(branch, merge, &outgoing)
                    .map(|path| path.into_iter().take_while(|&node| node != merge).collect())
            }
        })
        .collect::<Option<Vec<_>>>()?;

    let mut placements = Vec::with_capacity(node_count);
    let x_pad = 38.0;
    let y_pad = 18.0;
    let usable_width = (size.width - x_pad * 2.0).max(1.0);
    let usable_height = (size.height - y_pad * 2.0).max(1.0);

    let point = |x: f32, y: f32| {
        Point::new(
            origin.x + x_pad + usable_width * x,
            origin.y + y_pad + usable_height * y,
        )
    };

    let split_y = 0.30;
    let merge_y = 0.72;
    let pre_len = pre_split.len().saturating_sub(1).max(1);
    for (slot, &node_index) in pre_split.iter().enumerate() {
        let y = split_y * slot as f32 / pre_len as f32;
        placements.push(CircuitNodePlacement {
            id: descriptor.nodes[node_index].id,
            node: &descriptor.nodes[node_index],
            point: point(0.50, y),
        });
    }

    let branch_count = branch_paths.len().max(1);
    for (branch_slot, branch) in branch_paths.iter().enumerate() {
        if branch.is_empty() {
            continue;
        }
        let lane = branch_lane(branch_slot, branch_count);
        let branch_len = branch.len() + 1;
        for (slot, &node_index) in branch.iter().enumerate() {
            let y = split_y + (merge_y - split_y) * (slot + 1) as f32 / branch_len as f32;
            placements.push(CircuitNodePlacement {
                id: descriptor.nodes[node_index].id,
                node: &descriptor.nodes[node_index],
                point: point(lane, y),
            });
        }
    }

    let post_len = post_merge.len().saturating_sub(1).max(1);
    for (slot, &node_index) in post_merge.iter().enumerate() {
        let y = merge_y + (0.97 - merge_y) * slot as f32 / post_len as f32;
        placements.push(CircuitNodePlacement {
            id: descriptor.nodes[node_index].id,
            node: &descriptor.nodes[node_index],
            point: point(0.50, y),
        });
    }

    for node in descriptor.nodes {
        if placements.iter().all(|placement| placement.id != node.id) {
            placements.push(CircuitNodePlacement {
                id: node.id,
                node,
                point: Point::new(
                    origin.x + size.width * node.layout.x,
                    origin.y + size.height * node.layout.y,
                ),
            });
        }
    }

    Some(CircuitGraphLayout {
        direction: CircuitLayoutDirection::TopToBottom,
        placements,
    })
}

fn branch_lane(slot: usize, count: usize) -> f32 {
    if count <= 1 {
        0.50
    } else {
        let span = 0.58;
        0.50 - span * 0.5 + span * slot as f32 / (count - 1) as f32
    }
}

fn reaches(start: usize, target: usize, outgoing: &[Vec<usize>]) -> bool {
    if start == target {
        return true;
    }
    let mut stack = vec![start];
    let mut seen = vec![false; outgoing.len()];
    while let Some(current) = stack.pop() {
        if seen[current] {
            continue;
        }
        seen[current] = true;
        for &next in &outgoing[current] {
            if next == target {
                return true;
            }
            stack.push(next);
        }
    }
    false
}

fn path_between(start: usize, target: usize, outgoing: &[Vec<usize>]) -> Option<Vec<usize>> {
    if start == target {
        return Some(vec![start]);
    }
    for &next in &outgoing[start] {
        if let Some(mut path) = path_between(next, target, outgoing) {
            path.insert(0, start);
            return Some(path);
        }
    }
    None
}

fn follow_single_output_path(start: usize, outgoing: &[Vec<usize>]) -> Vec<usize> {
    let mut path = vec![start];
    let mut current = start;
    while outgoing[current].len() == 1 {
        current = outgoing[current][0];
        path.push(current);
    }
    path
}

fn descriptor_layout_fallback<'a>(
    descriptor: &'a CircuitDescriptor,
    origin: Point,
    size: Size,
    direction: CircuitLayoutDirection,
) -> CircuitGraphLayout<'a> {
    let placements = descriptor
        .nodes
        .iter()
        .map(|node| CircuitNodePlacement {
            id: node.id,
            node,
            point: Point::new(
                origin.x + size.width * node.layout.x,
                origin.y + size.height * node.layout.y,
            ),
        })
        .collect();
    CircuitGraphLayout {
        direction,
        placements,
    }
}

fn circuit_node_index(descriptor: &CircuitDescriptor, id: &str) -> Option<usize> {
    descriptor.nodes.iter().position(|node| node.id == id)
}

fn circuit_placement_by_id<'a>(
    placements: &'a [CircuitNodePlacement<'a>],
    id: &str,
) -> Option<&'a CircuitNodePlacement<'a>> {
    placements.iter().find(|placement| placement.id == id)
}

fn initial_layer_order(node: &CircuitNodeDescriptor, direction: CircuitLayoutDirection) -> f32 {
    match direction {
        CircuitLayoutDirection::TopToBottom => node.layout.x,
        CircuitLayoutDirection::LeftToRight => node.layout.y,
    }
}

fn circuit_order_map(layer_nodes: &[Vec<usize>], node_count: usize) -> Vec<f32> {
    let mut order = vec![0.0; node_count];
    for nodes in layer_nodes {
        for (slot, &node_index) in nodes.iter().enumerate() {
            order[node_index] = slot as f32;
        }
    }
    order
}

fn sort_layer_by_barycenter(nodes: &mut [usize], neighbors: &[Vec<usize>], neighbor_order: &[f32]) {
    nodes.sort_by(|&a, &b| {
        barycenter(a, neighbors, neighbor_order)
            .partial_cmp(&barycenter(b, neighbors, neighbor_order))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(&b))
    });
}

fn barycenter(node_index: usize, neighbors: &[Vec<usize>], neighbor_order: &[f32]) -> f32 {
    let related = &neighbors[node_index];
    if related.is_empty() {
        node_index as f32
    } else {
        related
            .iter()
            .map(|&neighbor| neighbor_order[neighbor])
            .sum::<f32>()
            / related.len() as f32
    }
}

fn layer_cross_positions(
    nodes: &[usize],
    descriptor: &CircuitDescriptor,
    direction: CircuitLayoutDirection,
) -> Vec<f32> {
    const MIN_CROSS_GAP: f32 = 0.30;
    const MIN_CROSS: f32 = 0.11;
    const MAX_CROSS: f32 = 0.89;

    if nodes.is_empty() {
        return Vec::new();
    }

    if nodes.len() == 1 {
        return vec![
            initial_layer_order(&descriptor.nodes[nodes[0]], direction).clamp(MIN_CROSS, MAX_CROSS)
        ];
    }

    let mut crosses: Vec<f32> = nodes
        .iter()
        .map(|&node_index| {
            initial_layer_order(&descriptor.nodes[node_index], direction)
                .clamp(MIN_CROSS, MAX_CROSS)
        })
        .collect();

    for index in 1..crosses.len() {
        let required = crosses[index - 1] + MIN_CROSS_GAP;
        if crosses[index] < required {
            crosses[index] = required;
        }
    }

    if let Some(overflow) = crosses.last().map(|last| (*last - MAX_CROSS).max(0.0)) {
        if overflow > 0.0 {
            for cross in &mut crosses {
                *cross -= overflow;
            }
        }
    }

    for index in (0..crosses.len().saturating_sub(1)).rev() {
        let required = crosses[index + 1] - MIN_CROSS_GAP;
        if crosses[index] > required {
            crosses[index] = required;
        }
    }

    if let Some(underflow) = crosses.first().map(|first| (MIN_CROSS - *first).max(0.0)) {
        if underflow > 0.0 {
            for cross in &mut crosses {
                *cross += underflow;
            }
        }
    }

    crosses
}

fn draw_semantic_circuit_edge(
    frame: &mut Frame,
    from: &CircuitNodePlacement<'_>,
    to: &CircuitNodePlacement<'_>,
    direction: CircuitLayoutDirection,
) {
    let from_point = circuit_edge_anchor(from, direction, true);
    let to_point = circuit_edge_anchor(to, direction, false);
    let path = Path::new(|p| {
        p.move_to(from_point);
        match direction {
            CircuitLayoutDirection::LeftToRight => {
                let mid_x = (from_point.x + to_point.x) * 0.5;
                p.line_to(Point::new(mid_x, from_point.y));
                p.line_to(Point::new(mid_x, to_point.y));
            }
            CircuitLayoutDirection::TopToBottom => {
                let mid_y = (from_point.y + to_point.y) * 0.5;
                p.line_to(Point::new(from_point.x, mid_y));
                p.line_to(Point::new(to_point.x, mid_y));
            }
        }
        p.line_to(to_point);
    });
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.58, 0.25, 0.72))
            .with_width(3.0),
    );
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.86, 0.48, 0.20))
            .with_width(1.0),
    );
}

fn circuit_edge_anchor(
    placement: &CircuitNodePlacement<'_>,
    direction: CircuitLayoutDirection,
    outgoing: bool,
) -> Point {
    let (width, height) = circuit_node_size(placement.node.kind);
    match direction {
        CircuitLayoutDirection::LeftToRight => {
            let offset = if outgoing { width * 0.5 } else { -width * 0.5 };
            Point::new(placement.point.x + offset, placement.point.y)
        }
        CircuitLayoutDirection::TopToBottom => {
            let offset = if outgoing {
                height * 0.5
            } else {
                -height * 0.5
            };
            Point::new(placement.point.x, placement.point.y + offset)
        }
    }
}

fn draw_semantic_circuit_node(frame: &mut Frame, center: Point, node: &CircuitNodeDescriptor) {
    let (width, height) = circuit_node_size(node.kind);
    let origin = Point::new(center.x - width * 0.5, center.y - height * 0.5);
    let radius = match node.kind {
        CircuitNodeKind::Port => 14.0,
        CircuitNodeKind::ClippingCell => 8.0,
        _ => 7.0,
    };
    let shape = rounded_rect(origin, Size::new(width, height), radius);
    frame.fill(&shape, circuit_node_fill(node.kind));
    frame.stroke(
        &shape,
        Stroke::default()
            .with_color(circuit_node_stroke(node.kind))
            .with_width(1.5),
    );

    match node.kind {
        CircuitNodeKind::ClippingCell => draw_clip_symbol(frame, center),
        CircuitNodeKind::ImpulseResponse | CircuitNodeKind::SpringTank => {
            draw_ir_symbol(frame, center)
        }
        CircuitNodeKind::Split | CircuitNodeKind::Mixer => draw_mix_symbol(frame, center),
        _ => {}
    }

    draw_text(
        frame,
        circuit_node_label(node),
        Point::new(center.x, center.y - 3.0),
        8.4,
        Color::from_rgb(0.96, 0.92, 0.78),
        Horizontal::Center,
    );
    draw_text(
        frame,
        circuit_node_detail(node),
        Point::new(center.x, center.y + 8.8),
        6.4,
        Color::from_rgba(0.86, 0.82, 0.70, 0.72),
        Horizontal::Center,
    );

    if let Some(control) = node.control_id {
        draw_control_binding_badge(
            frame,
            Point::new(origin.x + width - 7.0, origin.y + 7.0),
            control_badge_label(control),
            node.confidence,
        );
    } else if matches!(
        node.kind,
        CircuitNodeKind::InputLoad
            | CircuitNodeKind::OutputDriver
            | CircuitNodeKind::Port
            | CircuitNodeKind::ImpulseResponse
            | CircuitNodeKind::SpringTank
    ) {
        draw_confidence_dot(
            frame,
            Point::new(origin.x + width - 5.0, origin.y + height - 5.0),
            node.confidence,
        );
    }
}

fn circuit_node_size(kind: CircuitNodeKind) -> (f32, f32) {
    match kind {
        CircuitNodeKind::Port => (28.0, 28.0),
        CircuitNodeKind::Split | CircuitNodeKind::Mixer => (44.0, 32.0),
        CircuitNodeKind::ImpulseResponse | CircuitNodeKind::SpringTank => (50.0, 36.0),
        CircuitNodeKind::ClippingCell => (42.0, 32.0),
        _ => (46.0, 34.0),
    }
}

fn circuit_node_label(node: &CircuitNodeDescriptor) -> &'static str {
    match node.id {
        "input_jack" => "IN",
        "input_load" => "High-Z",
        "input_coupling" => "Couple",
        "path_split" => "Split",
        "clean_path" => "Clean",
        "drive_gain" => "Drive",
        "soft_clip" => "Clip",
        "drive_filter" => "Smooth",
        "treble_presence" => "Presence",
        "output_level" => "Level",
        "output_driver" => "Driver",
        "output_jack" => "OUT",
        "dwell_driver" => "Dwell",
        "spring_ir_tank" => "Tank IR",
        "splash_diffusion" => "Splash",
        "recovery_tone" => "Recover",
        "wet_dry_mixer" => "Mix",
        _ => node.label,
    }
}

fn circuit_node_detail(node: &CircuitNodeDescriptor) -> &'static str {
    match node.kind {
        CircuitNodeKind::Port => "voltage",
        CircuitNodeKind::InputLoad => "load",
        CircuitNodeKind::CouplingFilter => "HP",
        CircuitNodeKind::Buffer => "buffer",
        CircuitNodeKind::Split => "branch",
        CircuitNodeKind::CleanPath => "attack",
        CircuitNodeKind::GainStage => "gain",
        CircuitNodeKind::ClippingCell => "diodes",
        CircuitNodeKind::ToneNetwork => "filter",
        CircuitNodeKind::LevelControl => "pot",
        CircuitNodeKind::OutputDriver => "low-Z",
        CircuitNodeKind::TransducerDriver => "drive",
        CircuitNodeKind::SpringTank => "tank",
        CircuitNodeKind::ImpulseResponse => "IR",
        CircuitNodeKind::DiffusionNetwork => "delay",
        CircuitNodeKind::Mixer => "sum",
    }
}

fn circuit_node_fill(kind: CircuitNodeKind) -> Color {
    match kind {
        CircuitNodeKind::Port => Color::from_rgb(0.12, 0.18, 0.18),
        CircuitNodeKind::InputLoad | CircuitNodeKind::OutputDriver | CircuitNodeKind::Buffer => {
            Color::from_rgb(0.13, 0.22, 0.23)
        }
        CircuitNodeKind::CouplingFilter | CircuitNodeKind::ToneNetwork => {
            Color::from_rgb(0.18, 0.24, 0.19)
        }
        CircuitNodeKind::GainStage | CircuitNodeKind::TransducerDriver => {
            Color::from_rgb(0.30, 0.20, 0.13)
        }
        CircuitNodeKind::ClippingCell => Color::from_rgb(0.29, 0.14, 0.15),
        CircuitNodeKind::ImpulseResponse | CircuitNodeKind::SpringTank => {
            Color::from_rgb(0.13, 0.22, 0.28)
        }
        CircuitNodeKind::DiffusionNetwork => Color::from_rgb(0.16, 0.18, 0.28),
        CircuitNodeKind::CleanPath | CircuitNodeKind::Split | CircuitNodeKind::Mixer => {
            Color::from_rgb(0.16, 0.23, 0.18)
        }
        CircuitNodeKind::LevelControl => Color::from_rgb(0.25, 0.20, 0.13),
    }
}

fn circuit_node_stroke(kind: CircuitNodeKind) -> Color {
    match kind {
        CircuitNodeKind::ClippingCell => Color::from_rgba(1.0, 0.50, 0.44, 0.62),
        CircuitNodeKind::ImpulseResponse | CircuitNodeKind::SpringTank => {
            Color::from_rgba(0.50, 0.78, 0.95, 0.58)
        }
        CircuitNodeKind::Port | CircuitNodeKind::InputLoad | CircuitNodeKind::OutputDriver => {
            Color::from_rgba(0.78, 0.92, 0.86, 0.52)
        }
        _ => Color::from_rgba(0.96, 0.78, 0.44, 0.48),
    }
}

fn draw_clip_symbol(frame: &mut Frame, center: Point) {
    for offset in [-7.0, 7.0] {
        let diode = Path::new(|path| {
            path.move_to(Point::new(center.x + offset - 5.0, center.y + 13.0));
            path.line_to(Point::new(center.x + offset, center.y + 5.0));
            path.line_to(Point::new(center.x + offset + 5.0, center.y + 13.0));
            path.close();
        });
        frame.stroke(
            &diode,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 0.78, 0.54, 0.64))
                .with_width(1.1),
        );
    }
}

fn draw_ir_symbol(frame: &mut Frame, center: Point) {
    for i in 0..16 {
        let x = center.x - 22.0 + i as f32 * 44.0 / 15.0;
        let y = center.y + 12.0 + (i as f32 * 1.35).sin() * 4.2;
        frame.fill(
            &Path::circle(Point::new(x, y), 1.55),
            Color::from_rgba(0.72, 0.94, 1.0, 0.62),
        );
    }
}

fn draw_mix_symbol(frame: &mut Frame, center: Point) {
    frame.stroke(
        &Path::line(
            Point::new(center.x - 14.0, center.y + 12.0),
            Point::new(center.x + 14.0, center.y + 12.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.92, 0.76, 0.42, 0.52))
            .with_width(1.2),
    );
    frame.stroke(
        &Path::line(
            Point::new(center.x, center.y + 5.0),
            Point::new(center.x, center.y + 19.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.92, 0.76, 0.42, 0.52))
            .with_width(1.2),
    );
}

fn draw_control_binding_badge(
    frame: &mut Frame,
    center: Point,
    control: &'static str,
    confidence: CircuitConfidence,
) {
    let body = Path::circle(center, 7.4);
    frame.fill(&body, Color::from_rgb(0.72, 0.52, 0.25));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.82, 0.42, 0.54))
            .with_width(1.0),
    );
    draw_text(
        frame,
        control,
        Point::new(center.x, center.y - 0.5),
        6.5,
        Color::from_rgb(0.06, 0.05, 0.035),
        Horizontal::Center,
    );
    draw_confidence_dot(
        frame,
        Point::new(center.x + 6.0, center.y + 6.0),
        confidence,
    );
}

fn draw_confidence_dot(frame: &mut Frame, center: Point, confidence: CircuitConfidence) {
    let color = match confidence {
        CircuitConfidence::ExternalReference => Color::from_rgb(0.45, 0.78, 0.95),
        CircuitConfidence::Inferred | CircuitConfidence::Algorithmic => {
            Color::from_rgb(0.95, 0.65, 0.36)
        }
        _ => Color::from_rgb(0.58, 0.88, 0.58),
    };
    frame.fill(&Path::circle(center, 3.0), color);
}

fn draw_circuit_kind_badge(frame: &mut Frame, center: Point, kind: CircuitDescriptorKind) {
    let width = match kind {
        CircuitDescriptorKind::GreyboxIrHybrid => 84.0,
        _ => 64.0,
    };
    let body = rounded_rect(
        Point::new(center.x - width * 0.5, center.y - 11.0),
        Size::new(width, 22.0),
        11.0,
    );
    frame.fill(&body, Color::from_rgba(0.02, 0.025, 0.025, 0.82));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.78, 0.46, 0.42))
            .with_width(1.0),
    );
    draw_text(
        frame,
        circuit_descriptor_kind_label(kind),
        Point::new(center.x, center.y - 1.0),
        7.6,
        Color::from_rgb(0.90, 0.82, 0.64),
        Horizontal::Center,
    );
}

fn control_badge_label(control_id: &str) -> &'static str {
    match control_id {
        "gain" => "G",
        "treble" => "T",
        "output" => "O",
        "dwell" => "D",
        "tone" => "T",
        "mix" => "M",
        _ => "CTRL",
    }
}

fn circuit_descriptor_kind_label(kind: CircuitDescriptorKind) -> &'static str {
    match kind {
        CircuitDescriptorKind::CircuitInformed => "CIRCUIT",
        CircuitDescriptorKind::Greybox => "GREYBOX",
        CircuitDescriptorKind::GreyboxIrHybrid => "GREYBOX / IR",
        CircuitDescriptorKind::Algorithmic => "ALGO",
    }
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

fn draw_pedal_surface_reflections(frame: &mut Frame, origin: Point, size: Size) {
    let top_glint = rounded_rect(
        Point::new(origin.x + 22.0, origin.y + 4.0),
        Size::new(size.width * 0.16, 3.0),
        1.5,
    );
    frame.fill(&top_glint, Color::from_rgba(1.0, 0.98, 0.90, 0.12));

    let left_glint = rounded_rect(
        Point::new(origin.x + 4.0, origin.y + 22.0),
        Size::new(3.0, size.height * 0.08),
        1.5,
    );
    frame.fill(&left_glint, Color::from_rgba(1.0, 0.98, 0.90, 0.08));

    let right_shadow = Path::new(|path| {
        path.move_to(Point::new(origin.x + size.width - 1.0, origin.y + 34.0));
        path.line_to(Point::new(
            origin.x + size.width - 1.0,
            origin.y + size.height - 34.0,
        ));
    });
    frame.stroke(
        &right_shadow,
        Stroke::default()
            .with_color(Color::from_rgba(0.08, 0.07, 0.06, 0.08))
            .with_width(7.0),
    );

    let lower_shadow = Path::new(|path| {
        path.move_to(Point::new(origin.x + 28.0, origin.y + size.height - 1.0));
        path.line_to(Point::new(
            origin.x + size.width - 28.0,
            origin.y + size.height - 1.0,
        ));
    });
    frame.stroke(
        &lower_shadow,
        Stroke::default()
            .with_color(Color::from_rgba(0.08, 0.07, 0.06, 0.06))
            .with_width(6.0),
    );
}

fn draw_side_jack(frame: &mut Frame, origin: Point, left: bool) {
    let sign = if left { -1.0 } else { 1.0 };
    let edge_x = if left {
        origin.x + 20.0
    } else {
        origin.x + 3.0
    };
    let center_y = origin.y + 26.0;
    let outward = |distance: f32| edge_x + sign * distance;

    draw_jack_slice(frame, outward(0.0), center_y, sign, 7.0, 56.0, true);
    draw_jack_slice(frame, outward(6.0), center_y, sign, 11.0, 40.0, false);
    draw_jack_slice(frame, outward(15.0), center_y, sign, 10.0, 28.0, false);
}

fn draw_jack_slice(
    frame: &mut Frame,
    anchor_x: f32,
    center_y: f32,
    sign: f32,
    width: f32,
    height: f32,
    flush: bool,
) {
    let x = if sign < 0.0 {
        anchor_x - width
    } else {
        anchor_x
    };
    let origin = Point::new(x, center_y - height * 0.5);
    let radius = if flush { 2.0 } else { width * 0.48 };
    let body = rounded_rect(origin, Size::new(width, height), radius);
    frame.fill(&body, Color::from_rgb(0.72, 0.52, 0.27));
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(0.18, 0.09, 0.03, 0.48))
            .with_width(1.0),
    );

    frame.stroke(
        &Path::line(
            Point::new(x + width * 0.18, center_y - height * 0.34),
            Point::new(x + width * 0.82, center_y - height * 0.30),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.86, 0.52, 0.70))
            .with_width(1.2),
    );
    frame.stroke(
        &Path::line(
            Point::new(x + width * 0.22, center_y + height * 0.32),
            Point::new(x + width * 0.78, center_y + height * 0.28),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.16, 0.08, 0.03, 0.30))
            .with_width(1.4),
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
