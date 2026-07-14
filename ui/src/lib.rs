pub mod components;

use components::{KnobSkin, KnobSpec};
use greybound::{
    amp_circuit_descriptor, device_circuit_descriptor, AmpControls as CoreAmpControls,
    AuralithControls as CoreAuralithControls, CircuitConfidence, CircuitDescriptor,
    CircuitDescriptorKind, CircuitNodeDescriptor, CircuitNodeKind, CircuitSignalKind,
    DeviceConfig as CoreDeviceConfig, DeviceControls as CoreDeviceControls,
    DeviceSlotControls as CoreDeviceSlotControls, LumenControls as CoreLumenControls,
    MinotaurControls as CoreMinotaurControls, MonarchControls as CoreMonarchControls,
    MuffinControls as CoreMuffinControls, SpringfieldControls as CoreSpringfieldControls,
    StudioDelayControls as CoreStudioDelayControls, StudioVerbAlgorithm as CoreStudioVerbAlgorithm,
    StudioVerbControls as CoreStudioVerbControls,
};
use iced::advanced::graphics::gradient as canvas_gradient;
use iced::advanced::image as advanced_image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::Renderer as _;
use iced::advanced::{overlay, Clipboard, Shell, Widget};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Program as _, Renderer as _, Stroke, Text,
};
use iced::widget::{button, column, container, pick_list, row, text, Space};
use iced::{
    mouse, Alignment, Background, Color, Element, Event, Length, Point, Rectangle, Size, Vector,
};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

const INK: Color = Color::from_rgb(0.90, 0.91, 0.92);
const MUTED_INK: Color = Color::from_rgb(0.56, 0.60, 0.64);
const APP_BACKGROUND: Color = Color::from_rgb(0.047, 0.055, 0.067);
const CONTROL_SURFACE: Color = Color::from_rgb(0.11, 0.13, 0.15);
const PEDAL_CREAM: Color = Color::from_rgb(0.84, 0.80, 0.72);
const PEDAL_PEACH: Color = Color::from_rgb(0.77, 0.56, 0.45);
const PEDAL_SAGE: Color = Color::from_rgb(0.67, 0.62, 0.49);
const TEAL: Color = Color::from_rgb(0.33, 0.72, 0.66);
const GOLD: Color = Color::from_rgb(0.79, 0.54, 0.29);
pub const DESIGN_WIDTH: f32 = 1600.0;
pub const DESIGN_HEIGHT: f32 = 900.0;
const HEADER_HEIGHT: f32 = 86.0;
const FOOTER_HEIGHT: f32 = 44.0;
const MAIN_VIEW_HEIGHT: f32 = DESIGN_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT;
const PEDAL_STANDARD_WIDTH: f32 = 300.0;
const PEDAL_STANDARD_HEIGHT: f32 = 543.0;
const PEDAL_KNOB_RADIUS: f32 = 33.0;
const MUFFIN_KNOB_RADIUS: f32 = 20.0;
const LUMEN_KNOB_RADIUS: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Amp,
    Pedal,
    FxLoop,
    Cab,
}

struct TopNavButton {
    selected: bool,
}

impl button::StyleSheet for TopNavButton {
    type Style = iced::theme::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: self.selected.then_some(Background::Color(Color::from_rgba(
                GOLD.r, GOLD.g, GOLD.b, 0.08,
            ))),
            border_radius: 5.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: if self.selected { GOLD } else { MUTED_INK },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
            border_radius: 5.0.into(),
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
            background: self.selected.then_some(Background::Color(Color::from_rgba(
                GOLD.r, GOLD.g, GOLD.b, 0.16,
            ))),
            border_radius: 5.0.into(),
            border_width: if self.selected { 1.0 } else { 0.0 },
            border_color: Color::from_rgba(GOLD.r, GOLD.g, GOLD.b, 0.36),
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: if self.selected { INK } else { MUTED_INK },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            ..self.active(style)
        }
    }
}

struct FooterContainer;

impl container::StyleSheet for FooterContainer {
    type Style = iced::theme::Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(MUTED_INK),
            background: Some(Background::Color(CONTROL_SURFACE)),
            border_radius: 0.0.into(),
            border_width: 1.0,
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.07),
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
            background: Some(Background::Color(APP_BACKGROUND)),
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

struct TexturedHeader<'a> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
}

impl<'a> TexturedHeader<'a> {
    fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for TexturedHeader<'_> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let child = self.content.as_widget().layout(renderer, &limits);
        let size = limits.resolve(child.size());
        layout::Node::with_children(size, vec![child])
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> iced::event::Status {
        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout.children().next().expect("header content layout"),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("header content layout"),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if let Some(handle) = render_asset_handle(RenderAssetSpec {
            path: "assets/surfaces/header-brushed-gunmetal@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1774,
            pixel_height: 887,
        }) {
            advanced_image::Renderer::draw(renderer, handle, bounds);
        }

        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgba(0.018, 0.024, 0.031, 0.50),
        );
        frame.stroke(
            &Path::line(Point::ORIGIN, Point::new(bounds.width, 0.0)),
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.055))
                .with_width(1.0),
        );
        frame.stroke(
            &Path::line(
                Point::new(0.0, bounds.height - 1.0),
                Point::new(bounds.width, bounds.height - 1.0),
            ),
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.52))
                .with_width(1.0),
        );
        let finish = frame.into_geometry();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw(vec![finish]);
        });

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().expect("header content layout"),
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'b, Message, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("header content layout"),
            renderer,
        )
    }
}

impl<'a> From<TexturedHeader<'a>> for Element<'a, Message> {
    fn from(header: TexturedHeader<'a>) -> Self {
        Element::new(header)
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
            border_color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
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
    SelectAmpModel(AmpModel),
    ToggleCircuitView,
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
    MetronomeMuteProbabilityChanged(f32),
    ToggleDoubler,
    ToggleEq,
    SetEqHpf(f32),
    SetEqLpf(f32),
    SetEqBand {
        index: usize,
        value: f32,
    },
    AudioInputSelected(String),
    AudioInputSourceSelected(AudioInputSource),
    AudioOutputSelected(String),
    AudioSampleRateSelected(String),
    AudioBufferSizeSelected(String),
    LoadWavRequested,
    WavFileSelected(Option<PathBuf>),
    LoadNamRequested,
    NamFileSelected(Option<PathBuf>),
    ToggleRecording,
    RecordingFileSelected(Option<PathBuf>),
    RecordingStarted(PathBuf),
    RecordingStopped(Option<PathBuf>),
    RecordingFailed(String),
    AudioDevicesChanged {
        inputs: Vec<String>,
        outputs: Vec<String>,
        selected_input: Option<String>,
        selected_output: Option<String>,
        status: String,
    },
    AudioStatusChanged(String),
    MeterProbeTick,
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
    DriveChanged(f32),
    BassChanged(f32),
    TrebleChanged(f32),
    CutChanged(f32),
    PresenceChanged(f32),
    SagChanged(f32),
    MasterChanged(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Gain,
    Drive,
    Bass,
    Treble,
    Cut,
    Presence,
    Sag,
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSurfaceKind {
    Pedal,
    AmpHead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAssetFormat {
    PngRgba,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderAssetSpec {
    pub path: &'static str,
    pub format: RenderAssetFormat,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTypographyPolicy {
    BakedIntoAsset,
    DrawnByUi,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderSurfaceSpec {
    pub kind: RenderSurfaceKind,
    pub logical_width: f32,
    pub logical_height: f32,
    pub recommended_pixel_width: u32,
    pub recommended_pixel_height: u32,
    pub transparent_background: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderControlWidget {
    Pot,
    Slider,
    Toggle,
    Footswitch,
    Led,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderControlRole {
    Parameter(ControlKind),
    Bypass,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderRotationSpec {
    pub min_degrees: f32,
    pub max_degrees: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderControlAssetSpec {
    pub image: RenderAssetSpec,
    pub active_image: Option<RenderAssetSpec>,
    pub pressed_image: Option<RenderAssetSpec>,
    pub rotation: Option<RenderRotationSpec>,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderControlSpec {
    pub role: RenderControlRole,
    pub widget: RenderControlWidget,
    pub label: &'static str,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub radius: f32,
    pub hit_radius: f32,
    pub skin: KnobSkin,
    pub asset: Option<RenderControlAssetSpec>,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelRenderSpec {
    pub id: &'static str,
    pub surface: RenderSurfaceSpec,
    pub asset: Option<RenderAssetSpec>,
    pub typography: RenderTypographyPolicy,
    pub controls: &'static [RenderControlSpec],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalControl {
    Input,
    IrMix,
    DoublerDelay,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetronomeControl {
    Volume,
    Pan,
    MuteProbability,
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
    FxLoop,
    Cab,
    Eq,
    Record,
}

fn view_mode_label(view_mode: ViewMode) -> &'static str {
    match view_mode {
        ViewMode::Pedals => "PEDALS",
        ViewMode::Amp => "AMP",
        ViewMode::FxLoop => "LOOP",
        ViewMode::Cab => "CAB",
        ViewMode::Eq => "EQ",
        ViewMode::Record => "REC",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmpModel {
    Nox30,
    NamLoader,
    Daybreaker50,
    WideCombo,
    LeadHead,
}

impl AmpModel {
    fn id(self) -> &'static str {
        match self {
            Self::Nox30 => "nox30",
            Self::NamLoader => "nam2",
            Self::Daybreaker50 => "daybreaker-50",
            Self::WideCombo => "wide-combo",
            Self::LeadHead => "lead-head",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceModel {
    Lumen,
    Muffin,
    Minotaur,
    Monarch,
    Nox30,
    WideCombo,
    LeadHead,
    Springfield,
    DelayFx,
    ReverbFx,
    CabIr,
}

impl DeviceModel {
    fn title(self) -> &'static str {
        match self {
            Self::Lumen => "Lumen",
            Self::Muffin => "Muffin",
            Self::Minotaur => "Minotaur",
            Self::Monarch => "Monarch",
            Self::Nox30 => "Nox30",
            Self::WideCombo => "Wide Combo",
            Self::LeadHead => "Lead Head",
            Self::Springfield => "Springfield",
            Self::DelayFx => "Delay",
            Self::ReverbFx => "Reverb",
            Self::CabIr => "Celestion IR",
        }
    }
}

pub type CircuitDescriptorProvider = fn() -> Option<&'static CircuitDescriptor>;

pub fn no_circuit_descriptor() -> Option<&'static CircuitDescriptor> {
    None
}

pub fn nox30_circuit_descriptor() -> Option<&'static CircuitDescriptor> {
    amp_circuit_descriptor("nox30")
}

pub fn minotaur_circuit_descriptor() -> Option<&'static CircuitDescriptor> {
    device_circuit_descriptor(CoreDeviceConfig::Minotaur)
}

pub fn lumen_circuit_descriptor() -> Option<&'static CircuitDescriptor> {
    device_circuit_descriptor(CoreDeviceConfig::Lumen)
}

pub fn springfield_circuit_descriptor() -> Option<&'static CircuitDescriptor> {
    device_circuit_descriptor(CoreDeviceConfig::Springfield)
}

pub const STANDARD_PEDAL_SURFACE: RenderSurfaceSpec = RenderSurfaceSpec {
    kind: RenderSurfaceKind::Pedal,
    logical_width: PEDAL_STANDARD_WIDTH,
    logical_height: PEDAL_STANDARD_HEIGHT,
    recommended_pixel_width: 1200,
    recommended_pixel_height: 2172,
    transparent_background: true,
};

pub const STANDARD_AMP_HEAD_SURFACE: RenderSurfaceSpec = RenderSurfaceSpec {
    kind: RenderSurfaceKind::AmpHead,
    logical_width: 1240.0,
    logical_height: 500.0,
    recommended_pixel_width: 2480,
    recommended_pixel_height: 1000,
    transparent_background: true,
};

pub const NOX30_CROPPED_AMP_HEAD_SURFACE: RenderSurfaceSpec = RenderSurfaceSpec {
    kind: RenderSurfaceKind::AmpHead,
    logical_width: 810.0,
    logical_height: 428.0,
    recommended_pixel_width: 1620,
    recommended_pixel_height: 856,
    transparent_background: true,
};

pub const MINOTAUR_IVORY_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/minotaur-ivory@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const MINOTAUR_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/minotaur-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/minotaur-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const MINOTAUR_SILVER_FOOTSWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/minotaur-silver-footswitch@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: None,
};

pub const MONARCH_BRASS_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/monarch-brass@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const MONARCH_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/monarch-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/monarch-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const MONARCH_BRASS_FOOTSWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/monarch-brass-footswitch@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: None,
};

pub const MUFFIN_BLACK_BRASS_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/muffin-black-brass@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    // The source art's ivory indicator is at the physical minimum
    // (lower-left). Half travel rotates it to noon.
    rotation: Some(RenderRotationSpec {
        min_degrees: 0.0,
        max_degrees: 270.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const MUFFIN_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/muffin-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/muffin-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const MUFFIN_FOOTSWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/muffin-footswitch@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: None,
};

pub const SPRINGFIELD_STAINLESS_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/springfield-stainless@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const SPRINGFIELD_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/springfield-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/springfield-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const NOX30_BLACK_DIAL_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/nox30-black-dial@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1024,
        pixel_height: 1024,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const DAYBREAKER_BLACK_BRASS_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/daybreaker-black-brass@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const DAYBREAKER_POWER_SWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/daybreaker-power-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/buttons/daybreaker-power-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    }),
    pressed_image: None,
    rotation: None,
};

pub const DAYBREAKER_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/daybreaker-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/daybreaker-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const DAYBREAKER_INPUT_JACK_ASSET: RenderAssetSpec = RenderAssetSpec {
    path: "assets/controls/jacks/daybreaker-input@2x.png",
    format: RenderAssetFormat::PngRgba,
    pixel_width: 512,
    pixel_height: 512,
};

const NOX30_BLACK_DIAL_ROTATION_OFFSET_DEGREES: f32 = -135.0;

pub const AURALITH_BLACK_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/auralith-black@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1024,
        pixel_height: 1024,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const AURALITH_FOOTSWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/auralith-footswitch@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: None,
};

pub const AURALITH_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/auralith-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/auralith-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const LUMEN_PEARL_KNOB_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/knobs/lumen-pearl@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: Some(RenderRotationSpec {
        min_degrees: -135.0,
        max_degrees: 135.0,
        pivot_x: 0.5,
        pivot_y: 0.5,
    }),
};

pub const LUMEN_JEWEL_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/lumen-jewel-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/lumen-jewel-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const LUMEN_FOOTSWITCH_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/buttons/lumen-footswitch@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 512,
        pixel_height: 512,
    },
    active_image: None,
    pressed_image: None,
    rotation: None,
};

pub const NOX30_POWER_LED_ASSET: RenderControlAssetSpec = RenderControlAssetSpec {
    image: RenderAssetSpec {
        path: "assets/controls/leds/nox30-power-off@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    },
    active_image: Some(RenderAssetSpec {
        path: "assets/controls/leds/nox30-power-on@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 256,
        pixel_height: 256,
    }),
    pressed_image: None,
    rotation: None,
};

pub const NOX30_AMP_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Volume",
        anchor_x: 0.2820,
        anchor_y: 0.3035,
        radius: 29.2,
        hit_radius: 46.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_BLACK_DIAL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Bass),
        widget: RenderControlWidget::Pot,
        label: "Bass",
        anchor_x: 0.3837,
        anchor_y: 0.3035,
        radius: 29.2,
        hit_radius: 46.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_BLACK_DIAL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Cut),
        widget: RenderControlWidget::Pot,
        label: "Cut",
        anchor_x: 0.4830,
        anchor_y: 0.3034,
        radius: 29.2,
        hit_radius: 46.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_BLACK_DIAL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Sag),
        widget: RenderControlWidget::Pot,
        label: "Sag",
        anchor_x: 0.5817,
        anchor_y: 0.3036,
        radius: 29.2,
        hit_radius: 46.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_BLACK_DIAL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Treble",
        anchor_x: 0.6866,
        anchor_y: 0.3035,
        radius: 29.2,
        hit_radius: 46.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_BLACK_DIAL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Power LED",
        anchor_x: 0.8095,
        anchor_y: 0.3043,
        radius: 16.0,
        hit_radius: 0.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(NOX30_POWER_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Toggle,
        label: "Power",
        anchor_x: 0.8735,
        anchor_y: 0.2944,
        radius: 21.0,
        hit_radius: 36.0,
        skin: KnobSkin::AsatoBlack,
        asset: None,
    },
];

// Daybreaker deliberately keeps the control surface out of the photorealistic
// cabinet asset. Knobs and the power LED are layered at runtime so their state
// remains interactive and never needs to be baked into the amp background.
pub const DAYBREAKER_AMP_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Gain",
        anchor_x: 0.325,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Treble",
        anchor_x: 0.385,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Bass),
        widget: RenderControlWidget::Pot,
        label: "Bass",
        anchor_x: 0.445,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Cut),
        widget: RenderControlWidget::Pot,
        label: "Mid",
        anchor_x: 0.505,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Presence),
        widget: RenderControlWidget::Pot,
        label: "Presence",
        anchor_x: 0.565,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Volume",
        anchor_x: 0.625,
        anchor_y: 0.225,
        radius: 27.0,
        hit_radius: 42.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Power",
        anchor_x: 0.695,
        anchor_y: 0.225,
        radius: 24.0,
        hit_radius: 36.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_JEWEL_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Toggle,
        label: "Power switch",
        anchor_x: 0.745,
        anchor_y: 0.225,
        radius: 34.0,
        hit_radius: 40.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_POWER_SWITCH_ASSET),
    },
];

// NAM models expose their captured behavior internally. These two controls are
// intentionally external trims: Gain is applied before the model and Volume
// after it, both with unity at their midpoint.
pub const NAM_LOADER_AMP_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Gain",
        anchor_x: 0.625,
        anchor_y: 0.700,
        radius: 19.0,
        hit_radius: 36.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Volume",
        anchor_x: 0.740,
        anchor_y: 0.700,
        radius: 19.0,
        hit_radius: 36.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(DAYBREAKER_BLACK_BRASS_KNOB_ASSET),
    },
];

pub const MINOTAUR_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Gain",
        anchor_x: 0.268,
        anchor_y: 0.237,
        radius: 24.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MINOTAUR_IVORY_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Treble",
        anchor_x: 0.731,
        anchor_y: 0.237,
        radius: 24.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MINOTAUR_IVORY_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Output",
        anchor_x: 0.500,
        anchor_y: 0.365,
        radius: 24.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MINOTAUR_IVORY_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.774,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: Some(MINOTAUR_SILVER_FOOTSWITCH_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.500,
        anchor_y: 0.528,
        radius: 22.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(MINOTAUR_JEWEL_LED_ASSET),
    },
];

pub const MONARCH_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Gain",
        anchor_x: 0.234,
        anchor_y: 0.190,
        radius: 30.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MONARCH_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Tone",
        anchor_x: 0.500,
        anchor_y: 0.190,
        radius: 30.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MONARCH_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Output",
        anchor_x: 0.767,
        anchor_y: 0.190,
        radius: 30.0,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(MONARCH_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.500,
        anchor_y: 0.680,
        radius: 15.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(MONARCH_JEWEL_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.500,
        anchor_y: 0.820,
        radius: 54.0,
        hit_radius: 68.0,
        skin: KnobSkin::Teal,
        asset: Some(MONARCH_BRASS_FOOTSWITCH_ASSET),
    },
];

pub const MUFFIN_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Sustain",
        anchor_x: 0.278,
        anchor_y: 0.459,
        radius: MUFFIN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(MUFFIN_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Tone",
        anchor_x: 0.713,
        anchor_y: 0.458,
        radius: MUFFIN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(MUFFIN_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Level",
        anchor_x: 0.492,
        anchor_y: 0.617,
        radius: MUFFIN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(MUFFIN_BLACK_BRASS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.50,
        anchor_y: 0.073,
        radius: 20.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(MUFFIN_JEWEL_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.841,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: Some(MUFFIN_FOOTSWITCH_ASSET),
    },
];

pub const LUMEN_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Peak",
        anchor_x: 0.28,
        anchor_y: 0.15,
        radius: LUMEN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_PEARL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Gain",
        anchor_x: 0.72,
        anchor_y: 0.15,
        radius: LUMEN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_PEARL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Presence),
        widget: RenderControlWidget::Pot,
        label: "Emphasis",
        anchor_x: 0.292,
        anchor_y: 0.663,
        radius: LUMEN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_PEARL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Mix",
        anchor_x: 0.725,
        anchor_y: 0.663,
        radius: LUMEN_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_PEARL_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.50,
        anchor_y: 0.44,
        radius: 19.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_JEWEL_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.80,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: Some(LUMEN_FOOTSWITCH_ASSET),
    },
];

pub const AMP_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Volume",
        anchor_x: 0.28,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Treble",
        anchor_x: 0.72,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Bass),
        widget: RenderControlWidget::Pot,
        label: "Bass",
        anchor_x: 0.28,
        anchor_y: 0.31,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Cut),
        widget: RenderControlWidget::Pot,
        label: "Cut",
        anchor_x: 0.72,
        anchor_y: 0.31,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.82,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
];

pub const SPRINGFIELD_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Dwell",
        anchor_x: 0.25,
        anchor_y: 0.140,
        radius: 41.0,
        hit_radius: 56.0,
        skin: KnobSkin::Teal,
        asset: Some(SPRINGFIELD_STAINLESS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Tone",
        anchor_x: 0.75,
        anchor_y: 0.140,
        radius: 41.0,
        hit_radius: 56.0,
        skin: KnobSkin::Teal,
        asset: Some(SPRINGFIELD_STAINLESS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Mix",
        anchor_x: 0.50,
        anchor_y: 0.348,
        radius: 41.0,
        hit_radius: 56.0,
        skin: KnobSkin::Teal,
        asset: Some(SPRINGFIELD_STAINLESS_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.82,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.50,
        anchor_y: 0.108,
        radius: 17.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(SPRINGFIELD_JEWEL_LED_ASSET),
    },
];

pub const DELAY_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Time",
        anchor_x: 0.25,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Feedback",
        anchor_x: 0.75,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Mix",
        anchor_x: 0.50,
        anchor_y: 0.345,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.82,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
];

pub const REVERB_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Decay",
        anchor_x: 0.25,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Tone",
        anchor_x: 0.75,
        anchor_y: 0.155,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Mix",
        anchor_x: 0.50,
        anchor_y: 0.345,
        radius: PEDAL_KNOB_RADIUS,
        hit_radius: 48.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.50,
        anchor_y: 0.82,
        radius: 31.0,
        hit_radius: 50.0,
        skin: KnobSkin::Teal,
        asset: None,
    },
];

pub const AURALITH_PEDAL_CONTROLS: &[RenderControlSpec] = &[
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Gain),
        widget: RenderControlWidget::Pot,
        label: "Decay",
        anchor_x: 0.246,
        anchor_y: 0.217,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Bass),
        widget: RenderControlWidget::Pot,
        label: "Size",
        anchor_x: 0.493,
        anchor_y: 0.217,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Cut),
        widget: RenderControlWidget::Pot,
        label: "Texture",
        anchor_x: 0.738,
        anchor_y: 0.217,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Treble),
        widget: RenderControlWidget::Pot,
        label: "Tone",
        anchor_x: 0.246,
        anchor_y: 0.422,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Presence),
        widget: RenderControlWidget::Pot,
        label: "Low Cut",
        anchor_x: 0.493,
        anchor_y: 0.422,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Parameter(ControlKind::Master),
        widget: RenderControlWidget::Pot,
        label: "Mix",
        anchor_x: 0.738,
        anchor_y: 0.422,
        radius: 29.0,
        hit_radius: 48.0,
        skin: KnobSkin::AsatoBlack,
        asset: Some(AURALITH_BLACK_KNOB_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Led,
        label: "Status",
        anchor_x: 0.50,
        anchor_y: 0.60,
        radius: 21.0,
        hit_radius: 0.0,
        skin: KnobSkin::Teal,
        asset: Some(AURALITH_JEWEL_LED_ASSET),
    },
    RenderControlSpec {
        role: RenderControlRole::Bypass,
        widget: RenderControlWidget::Footswitch,
        label: "Bypass",
        anchor_x: 0.493,
        anchor_y: 0.804,
        radius: 40.0,
        hit_radius: 58.0,
        skin: KnobSkin::Teal,
        asset: Some(AURALITH_FOOTSWITCH_ASSET),
    },
];

pub const MINOTAUR_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.minotaur",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/minotaur-v2@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 914,
        pixel_height: 1721,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: MINOTAUR_PEDAL_CONTROLS,
};

pub const MONARCH_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.monarch",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/monarch@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1200,
        pixel_height: 2172,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: MONARCH_PEDAL_CONTROLS,
};

pub const MUFFIN_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.muffin",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/muffin@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1200,
        pixel_height: 2172,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: MUFFIN_PEDAL_CONTROLS,
};

pub const LUMEN_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.lumen",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/lumen@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1200,
        pixel_height: 2172,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: LUMEN_PEDAL_CONTROLS,
};

pub const NOX30_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.nox30",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: AMP_PEDAL_CONTROLS,
};

pub const WIDE_COMBO_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.wide-combo",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: AMP_PEDAL_CONTROLS,
};

pub const LEAD_HEAD_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.lead-head",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: AMP_PEDAL_CONTROLS,
};

pub const SPRINGFIELD_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.springfield",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/springfield@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1200,
        pixel_height: 2260,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: SPRINGFIELD_PEDAL_CONTROLS,
};

pub const DELAY_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.delay",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: DELAY_PEDAL_CONTROLS,
};

pub const REVERB_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.reverb",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: REVERB_PEDAL_CONTROLS,
};

pub const AURALITH_PEDAL_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "pedal.auralith",
    surface: STANDARD_PEDAL_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/pedals/auralith@4x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1200,
        pixel_height: 2172,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: AURALITH_PEDAL_CONTROLS,
};

pub const CAB_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "cab.ir",
    surface: STANDARD_PEDAL_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: &[],
};

pub const NOX30_AMP_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "amp.nox30",
    surface: NOX30_CROPPED_AMP_HEAD_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/amps/nox30-cropped@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1620,
        pixel_height: 856,
    }),
    typography: RenderTypographyPolicy::BakedIntoAsset,
    controls: NOX30_AMP_CONTROLS,
};

pub const WIDE_COMBO_AMP_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "amp.wide-combo",
    surface: STANDARD_AMP_HEAD_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: &[],
};

pub const DAYBREAKER_AMP_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "amp.daybreaker-50",
    surface: STANDARD_AMP_HEAD_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/amps/daybreaker-50-stack-badged-v4@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 2480,
        pixel_height: 1000,
    }),
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: DAYBREAKER_AMP_CONTROLS,
};

pub const LEAD_HEAD_AMP_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "amp.lead-head",
    surface: STANDARD_AMP_HEAD_SURFACE,
    asset: None,
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: &[],
};

pub const NAM_LOADER_AMP_RENDER_SPEC: ModelRenderSpec = ModelRenderSpec {
    id: "amp.nam-loader",
    surface: STANDARD_AMP_HEAD_SURFACE,
    asset: Some(RenderAssetSpec {
        path: "assets/amps/nam-loader-rack@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 2480,
        pixel_height: 1000,
    }),
    typography: RenderTypographyPolicy::DrawnByUi,
    controls: NAM_LOADER_AMP_CONTROLS,
};

#[derive(Debug, Clone, Copy)]
pub struct AppAmpModelDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub visual: AmpModel,
    pub render: &'static ModelRenderSpec,
    pub circuit: CircuitDescriptorProvider,
}

#[derive(Debug, Clone, Copy)]
pub struct AppDeviceModelDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: DeviceKind,
    pub visual: DeviceModel,
    pub runtime_config: Option<CoreDeviceConfig>,
    pub render: &'static ModelRenderSpec,
    pub circuit: CircuitDescriptorProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDeviceSection {
    PreAmp,
    PostAmp,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeDeviceSlot {
    pub model_id: &'static str,
    pub section: RuntimeDeviceSection,
    pub config: CoreDeviceConfig,
    pub bypassed: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeAudioSnapshot {
    pub input_gain: f32,
    pub output_gain: f32,
    pub amp: CoreAmpControls,
    pub amp_enabled: bool,
    pub devices: Vec<CoreDeviceSlotControls>,
    pub cab_mix: f32,
    pub metronome_enabled: bool,
    pub metronome_bpm: f32,
    pub metronome_volume: f32,
    pub metronome_pan: f32,
    pub metronome_mute_probability: f32,
    pub metronome_beats_per_bar: u32,
    pub metronome_rhythm_division: u32,
    pub eq_enabled: bool,
    pub eq_hpf_hz: Option<f32>,
    pub eq_lpf_hz: Option<f32>,
    pub eq_band_gains_db: [f32; EQ_BAND_COUNT],
    pub doubler_enabled: bool,
    pub doubler_delay_ms: f32,
    pub tuner_live: bool,
    pub tuner_muted: bool,
    pub tuner_reference_hz: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct AppProfile {
    pub name: &'static str,
    pub amp_models: &'static [AppAmpModelDescriptor],
    pub devices: &'static [AppDeviceModelDescriptor],
    pub runtime_devices: &'static [RuntimeDeviceSlot],
}

impl AppProfile {
    pub fn greybound_free() -> Self {
        GREYBOUND_FREE_PROFILE
    }

    pub fn greybound_glass() -> Self {
        GREYBOUND_GLASS_PROFILE
    }

    fn has_amp_model(self, model: AmpModel) -> bool {
        self.amp_models
            .iter()
            .any(|descriptor| descriptor.visual == model)
    }

    fn amp_descriptor_for_model(self, model: AmpModel) -> Option<&'static AppAmpModelDescriptor> {
        self.amp_models
            .iter()
            .find(|descriptor| descriptor.visual == model)
    }

    fn device_descriptor_for_model(
        self,
        model: DeviceModel,
    ) -> Option<&'static AppDeviceModelDescriptor> {
        self.devices
            .iter()
            .find(|descriptor| descriptor.visual == model)
    }
}

fn none_star_circuit_descriptor() -> Option<&'static greybound::CircuitDescriptor> {
    greybound::amp_circuit_descriptor("none-star")
}

fn boxer_seven_lead_circuit_descriptor() -> Option<&'static greybound::CircuitDescriptor> {
    greybound::amp_circuit_descriptor("boxer-seven-lead")
}

fn daybreaker_50_circuit_descriptor() -> Option<&'static greybound::CircuitDescriptor> {
    greybound::amp_circuit_descriptor("daybreaker-50")
}

const FREE_AMP_MODELS: &[AppAmpModelDescriptor] = &[
    AppAmpModelDescriptor {
        id: "nox30",
        label: "Nox30",
        visual: AmpModel::Nox30,
        render: &NOX30_AMP_RENDER_SPEC,
        circuit: nox30_circuit_descriptor,
    },
    AppAmpModelDescriptor {
        id: "daybreaker-50",
        label: "Daybreaker 50",
        visual: AmpModel::Daybreaker50,
        render: &DAYBREAKER_AMP_RENDER_SPEC,
        circuit: daybreaker_50_circuit_descriptor,
    },
    AppAmpModelDescriptor {
        id: "nam2",
        label: "NAM Loader",
        visual: AmpModel::NamLoader,
        render: &NAM_LOADER_AMP_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
];
const FREE_DEVICE_MODELS: &[AppDeviceModelDescriptor] = &[
    AppDeviceModelDescriptor {
        id: "lumen",
        label: "Lumen",
        kind: DeviceKind::Pedal,
        visual: DeviceModel::Lumen,
        runtime_config: Some(CoreDeviceConfig::Lumen),
        render: &LUMEN_PEDAL_RENDER_SPEC,
        circuit: lumen_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "muffin",
        label: "Muffin",
        kind: DeviceKind::Pedal,
        visual: DeviceModel::Muffin,
        runtime_config: Some(CoreDeviceConfig::Muffin),
        render: &MUFFIN_PEDAL_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "minotaur",
        label: "Minotaur",
        kind: DeviceKind::Pedal,
        visual: DeviceModel::Minotaur,
        runtime_config: Some(CoreDeviceConfig::Minotaur),
        render: &MINOTAUR_PEDAL_RENDER_SPEC,
        circuit: minotaur_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "monarch",
        label: "Monarch",
        kind: DeviceKind::Pedal,
        visual: DeviceModel::Monarch,
        runtime_config: Some(CoreDeviceConfig::Monarch),
        render: &MONARCH_PEDAL_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "auralith",
        label: "Auralith",
        kind: DeviceKind::FxLoop,
        visual: DeviceModel::ReverbFx,
        runtime_config: Some(CoreDeviceConfig::Auralith),
        render: &AURALITH_PEDAL_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "springfield",
        label: "Springfield",
        kind: DeviceKind::FxLoop,
        visual: DeviceModel::Springfield,
        runtime_config: Some(CoreDeviceConfig::Springfield),
        render: &SPRINGFIELD_PEDAL_RENDER_SPEC,
        circuit: springfield_circuit_descriptor,
    },
];
const FREE_RUNTIME_DEVICES: &[RuntimeDeviceSlot] = &[
    RuntimeDeviceSlot {
        model_id: "lumen",
        section: RuntimeDeviceSection::PreAmp,
        config: CoreDeviceConfig::Lumen,
        bypassed: true,
    },
    RuntimeDeviceSlot {
        model_id: "muffin",
        section: RuntimeDeviceSection::PreAmp,
        config: CoreDeviceConfig::Muffin,
        bypassed: true,
    },
    RuntimeDeviceSlot {
        model_id: "monarch",
        section: RuntimeDeviceSection::PreAmp,
        config: CoreDeviceConfig::Monarch,
        bypassed: true,
    },
    RuntimeDeviceSlot {
        model_id: "minotaur",
        section: RuntimeDeviceSection::PreAmp,
        config: CoreDeviceConfig::Minotaur,
        bypassed: false,
    },
    RuntimeDeviceSlot {
        model_id: "auralith",
        section: RuntimeDeviceSection::PostAmp,
        config: CoreDeviceConfig::Auralith,
        bypassed: false,
    },
    RuntimeDeviceSlot {
        model_id: "springfield",
        section: RuntimeDeviceSection::PostAmp,
        config: CoreDeviceConfig::Springfield,
        bypassed: true,
    },
];

pub const GREYBOUND_FREE_PROFILE: AppProfile = AppProfile {
    name: "Greybound Free",
    amp_models: FREE_AMP_MODELS,
    devices: FREE_DEVICE_MODELS,
    runtime_devices: FREE_RUNTIME_DEVICES,
};

const GLASS_AMP_MODELS: &[AppAmpModelDescriptor] = &[
    AppAmpModelDescriptor {
        id: "nox30",
        label: "Nox30",
        visual: AmpModel::Nox30,
        render: &NOX30_AMP_RENDER_SPEC,
        circuit: nox30_circuit_descriptor,
    },
    AppAmpModelDescriptor {
        id: "none-star",
        label: "None Star",
        visual: AmpModel::WideCombo,
        render: &WIDE_COMBO_AMP_RENDER_SPEC,
        circuit: none_star_circuit_descriptor,
    },
    AppAmpModelDescriptor {
        id: "boxer-seven-lead",
        label: "Boxer Seven Lead",
        visual: AmpModel::LeadHead,
        render: &LEAD_HEAD_AMP_RENDER_SPEC,
        circuit: boxer_seven_lead_circuit_descriptor,
    },
];

const GLASS_DEVICE_MODELS: &[AppDeviceModelDescriptor] = &[
    AppDeviceModelDescriptor {
        id: "minotaur",
        label: "Minotaur",
        kind: DeviceKind::Pedal,
        visual: DeviceModel::Minotaur,
        runtime_config: Some(CoreDeviceConfig::Minotaur),
        render: &MINOTAUR_PEDAL_RENDER_SPEC,
        circuit: minotaur_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "studio-delay",
        label: "StudioDelay",
        kind: DeviceKind::FxLoop,
        visual: DeviceModel::DelayFx,
        runtime_config: Some(CoreDeviceConfig::StudioDelay),
        render: &DELAY_PEDAL_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "springfield",
        label: "Springfield",
        kind: DeviceKind::FxLoop,
        visual: DeviceModel::Springfield,
        runtime_config: Some(CoreDeviceConfig::Springfield),
        render: &SPRINGFIELD_PEDAL_RENDER_SPEC,
        circuit: springfield_circuit_descriptor,
    },
    AppDeviceModelDescriptor {
        id: "studio-verb",
        label: "StudioVerb",
        kind: DeviceKind::FxLoop,
        visual: DeviceModel::ReverbFx,
        runtime_config: Some(CoreDeviceConfig::StudioVerb),
        render: &REVERB_PEDAL_RENDER_SPEC,
        circuit: no_circuit_descriptor,
    },
];

const GLASS_RUNTIME_DEVICES: &[RuntimeDeviceSlot] = &[
    RuntimeDeviceSlot {
        model_id: "minotaur",
        section: RuntimeDeviceSection::PreAmp,
        config: CoreDeviceConfig::Minotaur,
        bypassed: false,
    },
    RuntimeDeviceSlot {
        model_id: "studio-delay",
        section: RuntimeDeviceSection::PostAmp,
        config: CoreDeviceConfig::StudioDelay,
        bypassed: true,
    },
    RuntimeDeviceSlot {
        model_id: "springfield",
        section: RuntimeDeviceSection::PostAmp,
        config: CoreDeviceConfig::Springfield,
        bypassed: true,
    },
    RuntimeDeviceSlot {
        model_id: "studio-verb",
        section: RuntimeDeviceSection::PostAmp,
        config: CoreDeviceConfig::StudioVerb,
        bypassed: false,
    },
];

pub const GREYBOUND_GLASS_PROFILE: AppProfile = AppProfile {
    name: "Greybound Glass",
    amp_models: GLASS_AMP_MODELS,
    devices: GLASS_DEVICE_MODELS,
    runtime_devices: GLASS_RUNTIME_DEVICES,
};

#[derive(Debug, Clone)]
pub struct DeviceState {
    pub name: String,
    pub kind: DeviceKind,
    pub model: DeviceModel,
    pub bypassed: bool,
    pub gain: f32,
    pub drive: f32,
    pub bass: f32,
    pub treble: f32,
    pub cut: f32,
    pub presence: f32,
    pub sag: f32,
    pub master: f32,
}

impl DeviceState {
    pub fn lumen() -> Self {
        Self {
            name: "LUMEN".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Lumen,
            bypassed: true,
            gain: CoreLumenControls::default().peak_reduction,
            drive: 0.0,
            bass: 0.0,
            treble: CoreLumenControls::default().gain,
            cut: 0.0,
            presence: CoreLumenControls::default().emphasis,
            sag: 0.0,
            master: CoreLumenControls::default().mix,
        }
    }

    pub fn minotaur() -> Self {
        Self {
            name: "MINOTAUR".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Minotaur,
            bypassed: false,
            gain: 0.42,
            drive: 0.0,
            bass: 0.0,
            treble: 0.70,
            cut: 0.0,
            presence: 0.0,
            sag: 0.0,
            master: 0.03,
        }
    }

    pub fn monarch() -> Self {
        let controls = CoreMonarchControls::default();
        Self {
            name: "MONARCH".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Monarch,
            // Keep the established Free Minotaur preset unchanged while making
            // Monarch immediately available as the heavier overdrive option.
            bypassed: true,
            gain: controls.gain,
            drive: 0.0,
            bass: 0.0,
            treble: controls.tone,
            cut: 0.0,
            presence: 0.0,
            sag: 0.0,
            master: controls.output,
        }
    }

    pub fn muffin() -> Self {
        Self {
            name: "MUFFIN".to_string(),
            kind: DeviceKind::Pedal,
            model: DeviceModel::Muffin,
            // Free keeps the familiar Minotaur clean-boost preset as its
            // default sound; Muffin is ready at the visual noon reference.
            bypassed: true,
            gain: 0.50,
            drive: 0.0,
            bass: 0.0,
            treble: 0.50,
            cut: 0.0,
            presence: 0.0,
            sag: 0.0,
            master: 0.50,
        }
    }

    pub fn nox30() -> Self {
        Self {
            name: "NOX30".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::Nox30,
            bypassed: false,
            gain: 0.58,
            drive: 0.0,
            bass: 0.54,
            treble: 0.59,
            cut: 0.43,
            presence: 0.0,
            sag: 0.45,
            master: 0.45,
        }
    }

    pub fn nam_loader() -> Self {
        Self {
            name: "NAM LOADER".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::LeadHead,
            bypassed: false,
            gain: 0.50,
            drive: 0.0,
            bass: 0.50,
            treble: 0.50,
            cut: 0.50,
            presence: 0.50,
            sag: 0.50,
            master: 0.50,
        }
    }

    pub fn wide_combo() -> Self {
        Self {
            name: "WIDE COMBO".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::WideCombo,
            bypassed: false,
            gain: 0.35,
            drive: 0.05,
            bass: 0.42,
            treble: 1.0,
            cut: 0.55,
            presence: 1.0,
            sag: 0.20,
            master: 0.15,
        }
    }

    pub fn daybreaker_50() -> Self {
        Self {
            name: "DAYBREAKER 50".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::WideCombo,
            bypassed: false,
            gain: 0.38,
            drive: 0.04,
            bass: 0.46,
            treble: 0.70,
            cut: 0.64,
            presence: 0.66,
            sag: 0.18,
            // This position retains the previous calibrated Daybreaker level
            // with its real audio-tapered master-volume curve.
            master: 0.75,
        }
    }

    pub fn lead_head() -> Self {
        Self {
            name: "LEAD HEAD".to_string(),
            kind: DeviceKind::Amp,
            model: DeviceModel::LeadHead,
            bypassed: false,
            gain: 0.76,
            drive: 0.58,
            bass: 0.48,
            treble: 0.58,
            cut: 0.60,
            presence: 0.56,
            sag: 0.34,
            master: 0.62,
        }
    }

    pub fn springfield() -> Self {
        Self {
            name: "SPRING".to_string(),
            kind: DeviceKind::FxLoop,
            model: DeviceModel::Springfield,
            bypassed: true,
            gain: 0.48,
            drive: 0.0,
            bass: 0.0,
            treble: 0.58,
            cut: 0.0,
            presence: 0.0,
            sag: 0.0,
            master: 0.26,
        }
    }

    pub fn reverb_fx() -> Self {
        Self {
            name: "REVERB".to_string(),
            kind: DeviceKind::FxLoop,
            model: DeviceModel::ReverbFx,
            bypassed: false,
            gain: 0.42,
            drive: 0.0,
            bass: 0.46,
            treble: 0.54,
            cut: 0.64,
            presence: 0.36,
            sag: 0.0,
            master: 0.24,
        }
    }

    pub fn auralith() -> Self {
        Self {
            name: "AURALITH".to_string(),
            kind: DeviceKind::FxLoop,
            model: DeviceModel::ReverbFx,
            bypassed: false,
            gain: 0.52,
            drive: 0.0,
            bass: 0.55,
            treble: 0.55,
            cut: 0.68,
            presence: 0.32,
            sag: 0.0,
            master: 0.24,
        }
    }

    pub fn delay_fx() -> Self {
        Self {
            name: "DELAY".to_string(),
            kind: DeviceKind::FxLoop,
            model: DeviceModel::DelayFx,
            bypassed: true,
            gain: 0.28,
            drive: 0.0,
            bass: 0.0,
            treble: 0.34,
            cut: 0.0,
            presence: 0.08,
            sag: 0.0,
            master: 0.18,
        }
    }

    pub fn cab_ir() -> Self {
        Self {
            name: "CAB IR".to_string(),
            kind: DeviceKind::Cab,
            model: DeviceModel::CabIr,
            bypassed: false,
            gain: 0.0,
            drive: 0.0,
            bass: 0.0,
            treble: 0.0,
            cut: 0.0,
            presence: 0.0,
            sag: 0.0,
            master: 1.0,
        }
    }

    fn control_value(&self, control: ControlKind) -> f32 {
        match control {
            ControlKind::Gain => self.gain,
            ControlKind::Drive => self.drive,
            ControlKind::Bass => self.bass,
            ControlKind::Treble => self.treble,
            ControlKind::Cut => self.cut,
            ControlKind::Presence => self.presence,
            ControlKind::Sag => self.sag,
            ControlKind::Master => self.master,
        }
    }

    fn set_control(&mut self, control: ControlKind, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match control {
            ControlKind::Gain => self.gain = value,
            ControlKind::Drive => self.drive = value,
            ControlKind::Bass => self.bass = value,
            ControlKind::Treble => self.treble = value,
            ControlKind::Cut => self.cut = value,
            ControlKind::Presence => self.presence = value,
            ControlKind::Sag => self.sag = value,
            ControlKind::Master => self.master = value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GreyboundUi {
    pub app_profile: AppProfile,
    pub devices: Vec<DeviceState>,
    pub amp: DeviceState,
    pub amp_model: AmpModel,
    pub cab: DeviceState,
    pub input_gain: f32,
    pub output_gain: f32,
    pub meters: MeterLevels,
    pub audio_settings: AudioSettingsState,
    pub nam_loader: NamLoaderState,
    pub recording: RecordingState,
    pub metronome: MetronomeState,
    pub tuner: TunerState,
    pub doubler: DoublerState,
    pub eq: EqState,
    pub selected_index: usize,
    pub view_mode: ViewMode,
    pub circuit_view: bool,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub struct NamLoaderState {
    pub path: Option<PathBuf>,
    pub status: String,
}

impl Default for NamLoaderState {
    fn default() -> Self {
        Self {
            path: None,
            status: "No NAM model loaded".to_string(),
        }
    }
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
    pub mute_probability: f32,
}

impl Default for MetronomeState {
    fn default() -> Self {
        Self {
            open: false,
            enabled: false,
            bpm: 120.0,
            volume: 0.70,
            pan: 0.50,
            mute_probability: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DoublerState {
    pub enabled: bool,
    pub delay_ms: f32,
}

impl Default for DoublerState {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_ms: 7.15,
        }
    }
}

pub const EQ_BAND_COUNT: usize = 9;
pub const EQ_BAND_LABELS: [&str; EQ_BAND_COUNT] = [
    "65 Hz", "125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz", "16 kHz",
];
pub const EQ_MAX_GAIN_DB: f32 = 12.0;

#[derive(Debug, Clone)]
pub struct EqState {
    pub enabled: bool,
    pub hpf: f32,
    pub lpf: f32,
    pub bands: [f32; EQ_BAND_COUNT],
}

impl Default for EqState {
    fn default() -> Self {
        Self {
            enabled: true,
            hpf: 0.0,
            lpf: 0.0,
            bands: [0.5; EQ_BAND_COUNT],
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioSettingsState {
    pub open: bool,
    pub input_source: AudioInputSource,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub selected_input: Option<String>,
    pub selected_output: Option<String>,
    pub wav_path: Option<PathBuf>,
    pub status: String,
    pub sample_rate: u32,
    pub period_size: u32,
    pub sample_rates: Vec<String>,
    pub period_sizes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RecordingState {
    pub active: bool,
    pub path: Option<PathBuf>,
    pub status: String,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            active: false,
            path: None,
            status: "Ready to record the selected output".to_string(),
        }
    }
}

impl Default for AudioSettingsState {
    fn default() -> Self {
        Self {
            open: false,
            input_source: AudioInputSource::LiveInput,
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected_input: None,
            selected_output: None,
            wav_path: None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioInputSource {
    LiveInput,
    WavFile,
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
        Self::new(AppProfile::greybound_free())
    }
}

impl GreyboundUi {
    pub fn new(app_profile: AppProfile) -> Self {
        let default_amp = app_profile
            .amp_models
            .first()
            .map(|descriptor| descriptor.visual)
            .unwrap_or(AmpModel::Nox30);
        Self {
            app_profile,
            devices: default_devices(app_profile),
            amp: device_state_for_amp_model(default_amp),
            amp_model: default_amp,
            cab: DeviceState::cab_ir(),
            input_gain: 0.50,
            output_gain: 0.58,
            meters: MeterLevels::default(),
            audio_settings: AudioSettingsState::default(),
            nam_loader: NamLoaderState::default(),
            recording: RecordingState::default(),
            metronome: MetronomeState::default(),
            tuner: TunerState::default(),
            doubler: DoublerState::default(),
            eq: EqState::default(),
            selected_index: 0,
            view_mode: ViewMode::Pedals,
            circuit_view: false,
            scale: 1.0,
        }
    }
}

fn default_devices(app_profile: AppProfile) -> Vec<DeviceState> {
    app_profile
        .devices
        .iter()
        .map(device_state_for_descriptor)
        .collect()
}

fn device_state_for_descriptor(descriptor: &AppDeviceModelDescriptor) -> DeviceState {
    let mut state = match descriptor.runtime_config {
        Some(CoreDeviceConfig::Auralith) => DeviceState::auralith(),
        _ => device_state_for_model(descriptor.visual),
    };
    state.name = descriptor.label.to_string();
    state.kind = descriptor.kind;
    state
}

fn device_state_for_model(model: DeviceModel) -> DeviceState {
    match model {
        DeviceModel::Lumen => DeviceState::lumen(),
        DeviceModel::Muffin => DeviceState::muffin(),
        DeviceModel::Minotaur => DeviceState::minotaur(),
        DeviceModel::Monarch => DeviceState::monarch(),
        DeviceModel::Nox30 => DeviceState::nox30(),
        DeviceModel::WideCombo => DeviceState::wide_combo(),
        DeviceModel::LeadHead => DeviceState::lead_head(),
        DeviceModel::Springfield => DeviceState::springfield(),
        DeviceModel::DelayFx => DeviceState::delay_fx(),
        DeviceModel::ReverbFx => DeviceState::reverb_fx(),
        DeviceModel::CabIr => DeviceState::cab_ir(),
    }
}

fn device_state_for_amp_model(model: AmpModel) -> DeviceState {
    match model {
        AmpModel::Nox30 => DeviceState::nox30(),
        AmpModel::NamLoader => DeviceState::nam_loader(),
        AmpModel::Daybreaker50 => DeviceState::daybreaker_50(),
        AmpModel::WideCombo => DeviceState::wide_combo(),
        AmpModel::LeadHead => DeviceState::lead_head(),
    }
}

impl GreyboundUi {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectDevice(index) => {
                if index < self.devices.len() {
                    self.selected_index = index;
                    self.view_mode = view_mode_for_device_kind(self.devices[index].kind);
                }
            }
            Message::SelectView(view_mode) => {
                self.view_mode = view_mode;
            }
            Message::SelectAmpModel(model) => {
                self.select_amp_model(model);
            }
            Message::ToggleCircuitView => {
                self.circuit_view = !self.circuit_view;
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
            Message::MetronomeMuteProbabilityChanged(value) => {
                self.metronome.mute_probability = value.clamp(0.0, 1.0);
            }
            Message::ToggleDoubler => {
                self.doubler.enabled = !self.doubler.enabled;
            }
            Message::ToggleEq => {
                self.eq.enabled = !self.eq.enabled;
            }
            Message::SetEqHpf(value) => {
                self.eq.hpf = snap_eq_filter_value(value);
            }
            Message::SetEqLpf(value) => {
                self.eq.lpf = snap_eq_filter_value(value);
            }
            Message::SetEqBand { index, value } => {
                if let Some(band) = self.eq.bands.get_mut(index) {
                    *band = value.clamp(0.0, 1.0);
                }
            }
            Message::AudioInputSelected(device) => {
                self.audio_settings.selected_input = Some(device);
                self.audio_settings.input_source = AudioInputSource::LiveInput;
                self.audio_settings.status = "Restarting audio engine".to_string();
            }
            Message::AudioInputSourceSelected(source) => {
                self.audio_settings.input_source = source;
                self.audio_settings.status = match (source, self.audio_settings.wav_path.as_ref()) {
                    (AudioInputSource::LiveInput, _) => "Restarting audio engine".to_string(),
                    (AudioInputSource::WavFile, Some(_)) => "Restarting audio engine".to_string(),
                    (AudioInputSource::WavFile, None) => {
                        "Choose a WAV file to use file playback".to_string()
                    }
                };
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
            Message::LoadWavRequested => {
                self.audio_settings.status = "Opening WAV file picker".to_string();
            }
            Message::WavFileSelected(path) => {
                if let Some(path) = path {
                    self.audio_settings.wav_path = Some(path);
                    self.audio_settings.input_source = AudioInputSource::WavFile;
                    self.audio_settings.status = "Restarting audio engine".to_string();
                } else {
                    self.audio_settings.status = "WAV file selection canceled".to_string();
                }
            }
            Message::LoadNamRequested => {
                self.nam_loader.status = "Opening NAM model picker".to_string();
            }
            Message::NamFileSelected(path) => {
                if let Some(path) = path {
                    self.nam_loader.path = Some(path);
                    self.nam_loader.status = "Restarting audio engine with NAM model".to_string();
                    self.select_amp_model(AmpModel::NamLoader);
                    self.audio_settings.status = "Restarting audio engine".to_string();
                } else {
                    self.nam_loader.status = "NAM model selection canceled".to_string();
                }
            }
            Message::ToggleRecording => {
                self.recording.status = if self.recording.active {
                    "Stopping recording".to_string()
                } else {
                    "Choose a WAV destination".to_string()
                };
                self.view_mode = ViewMode::Record;
            }
            Message::RecordingFileSelected(path) => {
                if path.is_none() && !self.recording.active {
                    self.recording.status = "Recording canceled".to_string();
                }
            }
            Message::RecordingStarted(path) => {
                self.recording.active = true;
                self.recording.path = Some(path);
                self.recording.status = "Recording selected output".to_string();
                self.view_mode = ViewMode::Record;
            }
            Message::RecordingStopped(path) => {
                self.recording.active = false;
                if let Some(path) = path {
                    self.recording.path = Some(path);
                }
                self.recording.status = "Recording saved".to_string();
                self.view_mode = ViewMode::Record;
            }
            Message::RecordingFailed(error) => {
                self.recording.active = false;
                self.recording.status = error;
                self.view_mode = ViewMode::Record;
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
            Message::MeterProbeTick => {}
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
                    self.view_mode = view_mode_for_device_kind(device.kind);
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
                    self.view_mode = view_mode_for_device_kind(device.kind);
                }
            }
            Message::SetGlobalControl { control, value } => {
                self.set_global_control(control, value);
            }
            Message::GainChanged(value) => {
                self.active_device_mut().gain = value;
            }
            Message::DriveChanged(value) => {
                self.active_device_mut().drive = value;
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
            Message::PresenceChanged(value) => {
                self.active_device_mut().presence = value;
            }
            Message::SagChanged(value) => {
                self.active_device_mut().sag = value;
            }
            Message::MasterChanged(value) => {
                self.active_device_mut().master = value;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let scale = self.scale;

        let mode_tabs = row![
            self.view_button(ViewMode::Pedals),
            self.view_button(ViewMode::Amp),
            self.view_button(ViewMode::FxLoop),
            self.view_button(ViewMode::Cab),
            self.view_button(ViewMode::Eq),
            self.view_button(ViewMode::Record),
        ]
        .spacing(self.s(6.0))
        .align_items(Alignment::Center);

        let app_header = TexturedHeader::new(
            container(
                row![
                    text("GREYBOUND").size(self.font(18.0)).style(INK),
                    text("│").size(self.font(20.0)).style(MUTED_INK),
                    text(format!("{} · LIVE", self.amp_model_id().to_uppercase()))
                        .size(self.font(13.0))
                        .style(MUTED_INK),
                    mode_tabs,
                    Space::with_width(Length::Fill),
                    self.metered_global_knob(
                        "INPUT",
                        GlobalControl::Input,
                        self.input_gain,
                        normalized_db_readout(self.input_gain, -24.0, 24.0),
                        self.meters.input
                    ),
                    self.global_knob(
                        "IR",
                        GlobalControl::IrMix,
                        self.cab.master,
                        percent_readout(self.cab.master)
                    ),
                    self.doubler_control(),
                    self.output_metered_global_knob(
                        "OUTPUT",
                        GlobalControl::Output,
                        self.output_gain,
                        normalized_db_readout(self.output_gain, -24.0, 6.0),
                        self.meters.output_left,
                        self.meters.output_right
                    ),
                ]
                .spacing(self.s(12.0))
                .align_items(Alignment::Center),
            )
            .width(Length::Fixed(self.s(DESIGN_WIDTH)))
            .height(Length::Fixed(self.s(HEADER_HEIGHT)))
            .padding([self.s(0.0), self.s(30.0)]),
        )
        .width(Length::Fixed(self.s(DESIGN_WIDTH)))
        .height(Length::Fixed(self.s(HEADER_HEIGHT)));

        let top = app_header;

        let main_view: Element<'_, Message> = if self.audio_settings.open {
            self.audio_settings_panel()
        } else if self.metronome.open {
            self.metronome_panel()
        } else if self.tuner.open {
            self.tuner_panel()
        } else {
            match self.view_mode {
                ViewMode::Pedals => BoardCanvas::new(BoardArt {
                    app_profile: self.app_profile,
                    devices: self.device_slots_for_kind(DeviceKind::Pedal),
                    selected_index: self.selected_index,
                    amp_model: self.amp_model,
                    circuit_view: self.circuit_view,
                    pedalboard_backdrop: true,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
                .into(),
                ViewMode::FxLoop => BoardCanvas::new(BoardArt {
                    app_profile: self.app_profile,
                    devices: self.device_slots_for_kind(DeviceKind::FxLoop),
                    selected_index: self.selected_index,
                    amp_model: self.amp_model,
                    circuit_view: self.circuit_view,
                    pedalboard_backdrop: true,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
                .into(),
                ViewMode::Amp => AmpCanvas::new(AmpArt {
                    app_profile: self.app_profile,
                    amp: self.amp.clone(),
                    amp_model: self.amp_model,
                    nam_loader: self.nam_loader.clone(),
                    circuit_view: self.circuit_view,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
                .into(),
                ViewMode::Cab => CabCanvas::new(CabArt {
                    app_profile: self.app_profile,
                    cab: self.cab.clone(),
                    amp_model: self.amp_model,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
                .into(),
                ViewMode::Eq => EqCanvas::new(EqArt {
                    app_profile: self.app_profile,
                    eq: self.eq.clone(),
                    amp_model: self.amp_model,
                    scale,
                })
                .width(Length::Fixed(self.s(DESIGN_WIDTH)))
                .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
                .into(),
                ViewMode::Record => self.recording_view(),
            }
        };

        let bottom_text = MUTED_INK;
        let bottom = container(
            row![
                button(text("TUNER").size(self.font(14.0)).style(Color::WHITE))
                    .on_press(Message::ToggleTuner)
                    .style(iced::theme::Button::custom(FooterButton {
                        selected: self.tuner.open || self.tuner.muted
                    }))
                    .padding([self.s(4.0), self.s(10.0)]),
                text("•  MIDI").size(self.font(14.0)).style(bottom_text),
                button(
                    text(format!("{:.0} BPM", self.metronome.bpm))
                        .size(self.font(14.0))
                        .style(Color::WHITE)
                )
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
                text(format!(
                    "{} Hz  ·  {} samples",
                    self.audio_settings.sample_rate, self.audio_settings.period_size
                ))
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
        let selected = self.view_mode == view_mode;
        button(
            text(view_mode_label(view_mode))
                .size(self.font(14.0))
                .style(if selected { GOLD } else { MUTED_INK }),
        )
        .on_press(Message::SelectView(view_mode))
        .style(iced::theme::Button::custom(TopNavButton { selected }))
        .padding([self.s(10.0), self.s(12.0)])
        .into()
    }

    pub fn amp_model_id(&self) -> &'static str {
        self.app_profile
            .amp_descriptor_for_model(self.amp_model)
            .map(|descriptor| descriptor.id)
            .unwrap_or_else(|| self.amp_model.id())
    }

    pub fn runtime_amp_model_id(&self) -> String {
        match self.amp_model {
            AmpModel::NamLoader => self
                .nam_loader
                .path
                .as_ref()
                .map(|path| format!("nam2?path={}", path.display()))
                .unwrap_or_else(|| "nam2".to_string()),
            _ => self.amp_model_id().to_string(),
        }
    }

    pub fn has_loaded_nam_model(&self) -> bool {
        self.nam_loader.path.is_some()
    }

    pub fn runtime_audio_snapshot(&self) -> RuntimeAudioSnapshot {
        RuntimeAudioSnapshot {
            input_gain: normalized_gain(self.input_gain, -24.0, 24.0),
            output_gain: normalized_gain(self.output_gain, -24.0, 6.0),
            amp: self.runtime_amp_controls(),
            amp_enabled: !self.amp.bypassed,
            devices: self.runtime_device_controls(),
            cab_mix: if !self.cab.bypassed {
                self.cab.master.clamp(0.0, 1.0)
            } else {
                0.0
            },
            metronome_enabled: self.metronome.enabled,
            metronome_bpm: self.metronome.bpm.clamp(30.0, 260.0),
            metronome_volume: self.metronome.volume.clamp(0.0, 1.0),
            metronome_pan: self.metronome.pan.clamp(0.0, 1.0),
            metronome_mute_probability: self.metronome.mute_probability.clamp(0.0, 1.0),
            metronome_beats_per_bar: 4,
            metronome_rhythm_division: 1,
            eq_enabled: self.eq.enabled,
            eq_hpf_hz: eq_hpf_frequency_hz(self.eq.hpf),
            eq_lpf_hz: eq_lpf_frequency_hz(self.eq.lpf),
            eq_band_gains_db: self.eq.bands.map(eq_band_gain_db),
            doubler_enabled: self.doubler.enabled,
            doubler_delay_ms: self.doubler.delay_ms.clamp(0.0, 20.0),
            tuner_live: self.tuner.open && self.tuner.live,
            tuner_muted: self.tuner.muted,
            tuner_reference_hz: self.tuner.reference_hz.clamp(415.0, 466.0),
        }
    }

    fn runtime_amp_controls(&self) -> CoreAmpControls {
        let output = match self.amp_model_id() {
            "none-star" => 0.40 + self.amp.master * 1.20,
            "daybreaker-50" => 1.0,
            "boxer-seven-lead" => 0.20 + self.amp.master * 1.15,
            "nam2" => nam_loader_output_gain(self.amp.master),
            _ => 0.58,
        };
        CoreAmpControls {
            volume: self.amp.gain,
            bass: self.amp.bass,
            treble: self.amp.treble,
            cut: self.amp.cut,
            master: self.amp.master,
            output,
            drive: self.amp.drive,
            presence: self.amp.presence,
            sag: self.amp.sag,
        }
    }

    fn runtime_device_controls(&self) -> Vec<CoreDeviceSlotControls> {
        self.app_profile
            .runtime_devices
            .iter()
            .map(|slot| {
                let device = self.device_for_runtime_slot(slot);
                CoreDeviceSlotControls {
                    bypassed: device
                        .map(|device| device.bypassed)
                        .unwrap_or(slot.bypassed),
                    controls: self.runtime_controls_for_slot(slot, device),
                }
            })
            .collect()
    }

    fn device_for_runtime_slot(&self, slot: &RuntimeDeviceSlot) -> Option<&DeviceState> {
        let model = match slot.config {
            CoreDeviceConfig::Lumen => DeviceModel::Lumen,
            CoreDeviceConfig::Muffin => DeviceModel::Muffin,
            CoreDeviceConfig::Minotaur => DeviceModel::Minotaur,
            CoreDeviceConfig::Monarch => DeviceModel::Monarch,
            CoreDeviceConfig::StudioDelay => DeviceModel::DelayFx,
            CoreDeviceConfig::Springfield => DeviceModel::Springfield,
            CoreDeviceConfig::Auralith => DeviceModel::ReverbFx,
            CoreDeviceConfig::StudioVerb => DeviceModel::ReverbFx,
            _ => return None,
        };
        self.devices.iter().find(|device| device.model == model)
    }

    fn runtime_controls_for_slot(
        &self,
        slot: &RuntimeDeviceSlot,
        device: Option<&DeviceState>,
    ) -> CoreDeviceControls {
        match slot.config {
            CoreDeviceConfig::Lumen => {
                let device = device.cloned().unwrap_or_else(DeviceState::lumen);
                CoreDeviceControls::Lumen(CoreLumenControls {
                    peak_reduction: device.gain,
                    gain: device.treble,
                    emphasis: device.presence,
                    mix: device.master,
                })
            }
            CoreDeviceConfig::Muffin => {
                let device = device.cloned().unwrap_or_else(DeviceState::muffin);
                CoreDeviceControls::Muffin(CoreMuffinControls {
                    sustain: device.gain,
                    tone: device.treble,
                    level: device.master,
                })
            }
            CoreDeviceConfig::Minotaur => {
                let device = device.cloned().unwrap_or_else(DeviceState::minotaur);
                CoreDeviceControls::Minotaur(CoreMinotaurControls {
                    gain: device.gain,
                    treble: device.treble,
                    output: device.master,
                })
            }
            CoreDeviceConfig::Monarch => {
                let device = device.cloned().unwrap_or_else(DeviceState::monarch);
                CoreDeviceControls::Monarch(CoreMonarchControls {
                    gain: device.gain,
                    tone: device.treble,
                    output: device.master,
                })
            }
            CoreDeviceConfig::StudioDelay => {
                let device = device.cloned().unwrap_or_else(DeviceState::delay_fx);
                CoreDeviceControls::StudioDelay(CoreStudioDelayControls {
                    time_ms: 40.0 + device.gain * 1_160.0,
                    feedback: device.treble,
                    tone: 0.58,
                    mod_depth: device.presence,
                    mix: device.master,
                })
            }
            CoreDeviceConfig::Springfield => {
                let device = device.cloned().unwrap_or_else(DeviceState::springfield);
                CoreDeviceControls::Springfield(CoreSpringfieldControls {
                    dwell: device.gain,
                    tone: device.treble,
                    mix: device.master,
                })
            }
            CoreDeviceConfig::StudioVerb => {
                let device = device.cloned().unwrap_or_else(DeviceState::reverb_fx);
                CoreDeviceControls::StudioVerb(CoreStudioVerbControls {
                    algorithm: CoreStudioVerbAlgorithm::Room,
                    decay: device.gain,
                    size: device.bass,
                    pre_delay_ms: 12.0,
                    diffusion: device.cut,
                    tone: device.treble,
                    low_cut: device.presence,
                    mod_depth: 0.18,
                    mix: device.master,
                })
            }
            CoreDeviceConfig::Auralith => {
                let device = device.cloned().unwrap_or_else(DeviceState::auralith);
                CoreDeviceControls::Auralith(CoreAuralithControls {
                    decay: device.gain,
                    size: device.bass,
                    texture: device.cut,
                    tone: device.treble,
                    low_cut: device.presence,
                    mix: device.master,
                })
            }
            _ => CoreDeviceControls::Default,
        }
    }

    pub fn minotaur_device_config(&self) -> CoreDeviceConfig {
        CoreDeviceConfig::Minotaur
    }

    pub fn fx_loop_device_config(&self) -> CoreDeviceConfig {
        self.app_profile
            .runtime_devices
            .iter()
            .find(|slot| slot.section == RuntimeDeviceSection::PostAmp)
            .map(|slot| slot.config)
            .unwrap_or(CoreDeviceConfig::Auralith)
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
        let live_selected = settings.input_source == AudioInputSource::LiveInput;
        let wav_selected = settings.input_source == AudioInputSource::WavFile;
        let wav_name = settings
            .wav_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("No WAV loaded");
        let nam_name = self
            .nam_loader
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("No NAM loaded");
        let input_source = row![
            button(text("LIVE INPUT").size(self.font(15.0)).style(Color::WHITE))
                .on_press(Message::AudioInputSourceSelected(
                    AudioInputSource::LiveInput
                ))
                .style(iced::theme::Button::custom(FooterButton {
                    selected: live_selected
                }))
                .padding([self.s(13.0), self.s(18.0)]),
            button(text("WAV FILE").size(self.font(15.0)).style(Color::WHITE))
                .on_press(Message::AudioInputSourceSelected(AudioInputSource::WavFile))
                .style(iced::theme::Button::custom(FooterButton {
                    selected: wav_selected
                }))
                .padding([self.s(13.0), self.s(18.0)]),
            button(text("LOAD WAV").size(self.font(15.0)).style(Color::WHITE))
                .on_press(Message::LoadWavRequested)
                .style(iced::theme::Button::custom(FooterButton {
                    selected: false
                }))
                .padding([self.s(13.0), self.s(18.0)]),
            container(text(wav_name).size(self.font(15.0)).style(Color::WHITE))
                .width(Length::Fill)
                .center_y(),
        ]
        .spacing(self.s(12.0))
        .align_items(Alignment::Center);
        let nam_loader = row![
            button(text("LOAD NAM").size(self.font(15.0)).style(Color::WHITE))
                .on_press(Message::LoadNamRequested)
                .style(iced::theme::Button::custom(FooterButton {
                    selected: self.amp_model == AmpModel::NamLoader
                }))
                .padding([self.s(13.0), self.s(18.0)]),
            container(text(nam_name).size(self.font(15.0)).style(Color::WHITE))
                .width(Length::FillPortion(2))
                .center_y(),
            container(
                text(self.nam_loader.status.as_str())
                    .size(self.font(15.0))
                    .style(Color::from_rgb(0.82, 0.84, 0.90))
            )
            .width(Length::FillPortion(3))
            .center_y(),
        ]
        .spacing(self.s(12.0))
        .align_items(Alignment::Center);

        let content = column![
            row![
                self.settings_field(
                    "Audio Device Type",
                    text(audio_device_type_label(settings))
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
            self.settings_select_field("Input Source", input_source.into(), 846.0),
            self.settings_select_field("NAM Loader", nam_loader.into(), 846.0),
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
            .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
            .center_x()
            .center_y()
            .style(ghost_container(Color::from_rgba(0.04, 0.05, 0.08, 0.58)))
            .into()
    }

    fn recording_view(&self) -> Element<'_, Message> {
        RecordCanvas::new(RecordArt {
            active: self.recording.active,
            sample_rate: self.audio_settings.sample_rate,
            output_left: self.meters.output_left,
            output_right: self.meters.output_right,
            scale: self.scale,
        })
        .width(Length::Fixed(self.s(DESIGN_WIDTH)))
        .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
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
                        .width(Length::Fixed(self.s(180.0)))
                        .height(Length::Fixed(self.s(58.0)))
                        .style(dark_field_container())
                        .into(),
                    180.0
                ),
                self.settings_select_field(
                    "SOUND",
                    container(text("Blip").size(self.font(18.0)).style(Color::WHITE))
                        .padding([self.s(14.0), self.s(16.0)])
                        .width(Length::Fixed(self.s(180.0)))
                        .height(Length::Fixed(self.s(58.0)))
                        .style(dark_field_container())
                        .into(),
                    180.0
                ),
                self.metronome_knob(
                    "MUTE CHANCE",
                    MetronomeControl::MuteProbability,
                    metro.mute_probability,
                    metronome_mute_probability_readout(metro.mute_probability)
                ),
                self.metronome_knob(
                    "PAN",
                    MetronomeControl::Pan,
                    metro.pan,
                    metronome_pan_readout(metro.pan)
                ),
            ]
            .spacing(self.s(28.0))
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
            .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
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
            .height(Length::Fixed(self.s(MAIN_VIEW_HEIGHT)))
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
        self.settings_box_width(label, value, 185.0)
    }

    fn settings_box_width(
        &self,
        label: &'static str,
        value: &str,
        width: f32,
    ) -> Element<'_, Message> {
        container(
            column![
                text(label).size(self.font(15.0)).style(Color::WHITE),
                container(
                    text(value.to_string())
                        .size(self.font(18.0))
                        .style(Color::WHITE)
                )
                .padding([self.s(14.0), self.s(16.0)])
                .width(Length::Fixed(self.s(width)))
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
            GlobalKnobCanvas::new(GlobalKnobArt {
                control,
                value,
                meter_level: value,
                scale: self.scale,
                label,
                readout,
            })
            .width(Length::Fixed(self.s(84.0)))
            .height(Length::Fixed(self.s(80.0))),
        )
        .into()
    }

    fn doubler_control(&self) -> Element<'_, Message> {
        DoublerCanvas::new(DoublerArt {
            enabled: self.doubler.enabled,
            value: doubler_delay_to_normalized(self.doubler.delay_ms),
            readout: format!("{:.2} ms", self.doubler.delay_ms),
            scale: self.scale,
        })
        .width(Length::Fixed(self.s(94.0)))
        .height(Length::Fixed(self.s(80.0)))
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
        GlobalKnobCanvas::new(GlobalKnobArt {
            control,
            value,
            meter_level,
            scale: self.scale,
            label,
            readout,
        })
        .width(Length::Fixed(self.s(84.0)))
        .height(Length::Fixed(self.s(80.0)))
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
        GlobalKnobCanvas::new(GlobalKnobArt {
            control,
            value,
            meter_level: left_level.max(right_level),
            scale: self.scale,
            label,
            readout,
        })
        .width(Length::Fixed(self.s(84.0)))
        .height(Length::Fixed(self.s(80.0)))
        .into()
    }

    fn set_global_control(&mut self, control: GlobalControl, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match control {
            GlobalControl::Input => self.input_gain = value,
            GlobalControl::IrMix => self.cab.master = value,
            GlobalControl::DoublerDelay => {
                self.doubler.delay_ms = normalized_to_doubler_delay(value)
            }
            GlobalControl::Output => self.output_gain = value,
        }
    }

    fn select_amp_model(&mut self, model: AmpModel) {
        if self.amp_model == model || !self.app_profile.has_amp_model(model) {
            return;
        }
        self.amp_model = model;
        self.amp = device_state_for_amp_model(model);
        self.view_mode = ViewMode::Amp;
        self.circuit_view = false;
    }

    fn active_device_mut(&mut self) -> &mut DeviceState {
        match self.view_mode {
            ViewMode::Pedals | ViewMode::FxLoop => {
                let index = self
                    .selected_index
                    .min(self.devices.len().saturating_sub(1));
                &mut self.devices[index]
            }
            ViewMode::Amp => &mut self.amp,
            ViewMode::Cab => &mut self.cab,
            ViewMode::Eq | ViewMode::Record => &mut self.amp,
        }
    }

    fn device_slots_for_kind(&self, kind: DeviceKind) -> Vec<BoardDeviceSlot> {
        self.devices
            .iter()
            .cloned()
            .enumerate()
            .filter_map(|(source_index, device)| {
                (device.kind == kind).then_some(BoardDeviceSlot {
                    source_index,
                    device,
                })
            })
            .collect()
    }

    fn s(&self, value: f32) -> f32 {
        value * self.scale
    }

    fn font(&self, value: f32) -> u16 {
        (value * self.scale).round().max(1.0) as u16
    }
}

fn nam_loader_output_gain(value: f32) -> f32 {
    10.0_f32.powf((value.clamp(0.0, 1.0) - 0.5) * 36.0 / 20.0)
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

pub fn eq_band_gain_db(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) - 0.5) * EQ_MAX_GAIN_DB * 2.0
}

fn eq_band_readout(value: f32) -> String {
    format!("{:.1} dB", eq_band_gain_db(value))
}

pub fn eq_hpf_frequency_hz(value: f32) -> Option<f32> {
    let value = value.clamp(0.0, 1.0);
    (value > 0.01).then(|| log_lerp(20.0, 250.0, (value - 0.01) / 0.99))
}

pub fn eq_lpf_frequency_hz(value: f32) -> Option<f32> {
    let value = value.clamp(0.0, 1.0);
    (value > 0.01).then(|| log_lerp(20_000.0, 2_000.0, (value - 0.01) / 0.99))
}

fn snap_eq_filter_value(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value < 0.035 {
        0.0
    } else {
        value
    }
}

fn audio_device_type_label(settings: &AudioSettingsState) -> &'static str {
    if settings
        .selected_input
        .as_deref()
        .is_some_and(|input| input.starts_with("Browser "))
        || settings
            .selected_output
            .as_deref()
            .is_some_and(|output| output.starts_with("Browser "))
    {
        "WebAudio"
    } else {
        "CoreAudio"
    }
}

fn log_lerp(min: f32, max: f32, t: f32) -> f32 {
    min * (max / min).powf(t.clamp(0.0, 1.0))
}

fn eq_filter_readout(prefix: &str, frequency: Option<f32>) -> String {
    match frequency {
        None => format!("{prefix} : OFF"),
        Some(frequency) if frequency >= 1_000.0 => {
            format!("{prefix} : {:.1} kHz", frequency / 1_000.0)
        }
        Some(frequency) => format!("{prefix} : {:.0} Hz", frequency),
    }
}

fn normalized_to_doubler_delay(value: f32) -> f32 {
    value.clamp(0.0, 1.0) * 20.0
}

fn doubler_delay_to_normalized(delay_ms: f32) -> f32 {
    (delay_ms / 20.0).clamp(0.0, 1.0)
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

fn metronome_mute_probability_readout(value: f32) -> String {
    format!("{:.0}%", value.clamp(0.0, 1.0) * 100.0)
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

fn view_mode_for_device_kind(kind: DeviceKind) -> ViewMode {
    match kind {
        DeviceKind::Pedal => ViewMode::Pedals,
        DeviceKind::FxLoop => ViewMode::FxLoop,
        DeviceKind::Amp => ViewMode::Amp,
        DeviceKind::Cab => ViewMode::Cab,
    }
}

#[derive(Debug, Clone)]
struct BoardDeviceSlot {
    source_index: usize,
    device: DeviceState,
}

#[derive(Debug, Clone)]
struct BoardArt {
    app_profile: AppProfile,
    devices: Vec<BoardDeviceSlot>,
    selected_index: usize,
    amp_model: AmpModel,
    circuit_view: bool,
    pedalboard_backdrop: bool,
    scale: f32,
}

#[derive(Debug, Clone)]
struct BoardCanvas {
    width: Length,
    height: Length,
    art: BoardArt,
}

impl BoardCanvas {
    const DEFAULT_SIZE: f32 = 100.0;

    fn new(art: BoardArt) -> Self {
        Self {
            width: Length::Fixed(Self::DEFAULT_SIZE),
            height: Length::Fixed(Self::DEFAULT_SIZE),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for BoardCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<DragState>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DragState::default())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        if let Some(canvas_event) = canvas_event {
            let state = tree.state.downcast_mut::<DragState>();
            let (status, message) = self
                .art
                .update(state, canvas_event, layout.bounds(), cursor);
            if let Some(message) = message {
                shell.publish(message);
            }
            return status;
        }

        iced::event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<DragState>();
        self.art.mouse_interaction(state, layout.bounds(), cursor)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<DragState>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let background = draw_board_background(renderer, &self.art, bounds.size());
            renderer.draw(vec![background]);
            if self.art.pedalboard_backdrop {
                draw_pedalboard_backdrop(renderer, bounds.size());
            }
            draw_board_assets(renderer, &self.art, bounds.size());
            draw_board_control_assets(renderer, &self.art, bounds.size());
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
        });
    }
}

impl<'a> From<BoardCanvas> for Element<'a, Message> {
    fn from(board: BoardCanvas) -> Self {
        Element::new(board)
    }
}

#[derive(Debug, Clone)]
struct AmpCanvas {
    width: Length,
    height: Length,
    art: AmpArt,
}

impl AmpCanvas {
    const DEFAULT_SIZE: f32 = 100.0;

    fn new(art: AmpArt) -> Self {
        Self {
            width: Length::Fixed(Self::DEFAULT_SIZE),
            height: Length::Fixed(Self::DEFAULT_SIZE),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for AmpCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<DragState>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DragState::default())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        if let Some(canvas_event) = canvas_event {
            let state = tree.state.downcast_mut::<DragState>();
            let (status, message) = self
                .art
                .update(state, canvas_event, layout.bounds(), cursor);
            if let Some(message) = message {
                shell.publish(message);
            }
            return status;
        }

        iced::event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<DragState>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let background = draw_amp_background(renderer, &self.art, bounds.size());
            renderer.draw(vec![background]);
            if !self.art.circuit_view {
                draw_amp_studio_backdrop(renderer, bounds.size());
            }
            draw_amp_asset(renderer, &self.art, bounds.size());
            draw_amp_control_assets(renderer, &self.art, bounds.size());
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
        });
    }
}

impl<'a> From<AmpCanvas> for Element<'a, Message> {
    fn from(amp: AmpCanvas) -> Self {
        Element::new(amp)
    }
}

#[derive(Debug, Clone)]
struct EqCanvas {
    width: Length,
    height: Length,
    art: EqArt,
}

impl EqCanvas {
    const DEFAULT_SIZE: f32 = 100.0;

    fn new(art: EqArt) -> Self {
        Self {
            width: Length::Fixed(Self::DEFAULT_SIZE),
            height: Length::Fixed(Self::DEFAULT_SIZE),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for EqCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<EqDragState>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(EqDragState::default())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        if let Some(canvas_event) = canvas_event {
            let state = tree.state.downcast_mut::<EqDragState>();
            let (status, message) = self
                .art
                .update(state, canvas_event, layout.bounds(), cursor);
            if let Some(message) = message {
                shell.publish(message);
            }
            return status;
        }

        iced::event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<EqDragState>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let background = draw_eq_background(renderer, &self.art, bounds.size());
            renderer.draw(vec![background]);
            draw_amp_studio_backdrop(renderer, bounds.size());
            draw_eq_panel_asset(renderer, self.art.scale);
            draw_eq_slider_cap_assets(renderer, &self.art);
            draw_eq_filter_knob_assets(renderer, &self.art);
            draw_eq_power_switch_asset(renderer, self.art.scale);
            draw_eq_power_led_asset(renderer, &self.art);
            renderer.with_layer(
                Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
                |renderer| {
                    renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
                },
            );
        });
    }
}

impl<'a> From<EqCanvas> for Element<'a, Message> {
    fn from(eq: EqCanvas) -> Self {
        Element::new(eq)
    }
}

fn draw_board_assets(renderer: &mut iced::Renderer, art: &BoardArt, bounds: Size) {
    if art.circuit_view || !render_assets_enabled() {
        return;
    }

    let logical_size = unscale_size(bounds, art.scale);
    let layout = board_layout(art.devices.len(), logical_size);
    let y = pedal_board_y(logical_size, layout.pedal_h);

    for (visible_index, slot) in art.devices.iter().enumerate() {
        let render_spec = device_render_spec(art.app_profile, slot.device.model);
        let Some(asset) = render_spec.asset else {
            continue;
        };
        let Some(handle) = render_asset_handle(asset) else {
            continue;
        };

        let x = layout.start_x + visible_index as f32 * (layout.pedal_w + layout.gap);
        let image_bounds = Rectangle {
            x: x * art.scale,
            y: y * art.scale,
            width: layout.pedal_w * art.scale,
            height: layout.pedal_h * art.scale,
        };
        advanced_image::Renderer::draw(renderer, handle, image_bounds);
    }
}

fn draw_board_control_assets(renderer: &mut iced::Renderer, art: &BoardArt, bounds: Size) {
    if art.circuit_view || !render_assets_enabled() {
        return;
    }

    let logical_size = unscale_size(bounds, art.scale);
    let layout = board_layout(art.devices.len(), logical_size);
    let y = pedal_board_y(logical_size, layout.pedal_h);
    let pedal_size = Size::new(layout.pedal_w, layout.pedal_h);

    for (visible_index, slot) in art.devices.iter().enumerate() {
        let origin = Point::new(
            layout.start_x + visible_index as f32 * (layout.pedal_w + layout.gap),
            y,
        );
        let render_spec = device_render_spec(art.app_profile, slot.device.model);

        for control in render_spec.controls {
            let Some(asset) = control.asset else {
                continue;
            };
            let value = match control.role {
                RenderControlRole::Parameter(kind) => slot.device.control_value(kind),
                RenderControlRole::Bypass => bypass_asset_value(slot.device.bypassed),
            };
            let Some(handle) = render_control_asset_handle(asset, value) else {
                continue;
            };
            let center = render_control_center(control, origin, pedal_size);
            let diameter = control.radius * 2.45;
            let image_bounds = Rectangle {
                x: (center.x - diameter * 0.5) * art.scale,
                y: (center.y - diameter * 0.5) * art.scale,
                width: diameter * art.scale,
                height: diameter * art.scale,
            };
            advanced_image::Renderer::draw(renderer, handle, image_bounds);
        }
    }
}

fn draw_board_background(renderer: &iced::Renderer, art: &BoardArt, bounds: Size) -> Geometry {
    let mut frame = Frame::new(renderer, bounds);
    frame.scale(art.scale);
    draw_stage_background(&mut frame, unscale_size(bounds, art.scale));
    frame.into_geometry()
}

fn draw_pedalboard_backdrop(renderer: &mut iced::Renderer, bounds: Size) {
    let Some(handle) = render_asset_handle(RenderAssetSpec {
        path: "assets/surfaces/pedalboard-studio@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1807,
        pixel_height: 870,
    }) else {
        return;
    };

    advanced_image::Renderer::draw(
        renderer,
        handle,
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: bounds.width,
            height: bounds.height,
        },
    );
}

fn draw_amp_background(renderer: &iced::Renderer, art: &AmpArt, bounds: Size) -> Geometry {
    let mut frame = Frame::new(renderer, bounds);
    frame.scale(art.scale);
    draw_stage_background(&mut frame, unscale_size(bounds, art.scale));
    frame.into_geometry()
}

fn draw_amp_studio_backdrop(renderer: &mut iced::Renderer, bounds: Size) {
    let Some(handle) = render_asset_handle(RenderAssetSpec {
        path: "assets/surfaces/amp-studio@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1672,
        pixel_height: 941,
    }) else {
        return;
    };

    advanced_image::Renderer::draw(
        renderer,
        handle,
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: bounds.width,
            height: bounds.height,
        },
    );
}

fn draw_cab_background(renderer: &iced::Renderer, art: &CabArt, bounds: Size) -> Geometry {
    let mut frame = Frame::new(renderer, bounds);
    frame.scale(art.scale);
    draw_stage_background(&mut frame, unscale_size(bounds, art.scale));
    frame.into_geometry()
}

fn draw_eq_background(renderer: &iced::Renderer, art: &EqArt, bounds: Size) -> Geometry {
    let mut frame = Frame::new(renderer, bounds);
    frame.scale(art.scale);
    draw_stage_background(&mut frame, unscale_size(bounds, art.scale));
    frame.into_geometry()
}

fn draw_global_knob_asset(renderer: &mut iced::Renderer, art: &GlobalKnobArt, bounds: Size) {
    const FRAME_COUNT: usize = 121;
    let index = ((FRAME_COUNT - 1) as f32 * art.value.clamp(0.0, 1.0)).round() as usize;
    let Some(handle) = global_copper_knob_handles().get(index) else {
        return;
    };

    let logical_size = unscale_size(bounds, art.scale);
    let center = Point::new(logical_size.width * 0.5, 44.0);
    let diameter = 50.0 * art.scale;
    let image_bounds = Rectangle {
        x: center.x * art.scale - diameter * 0.5,
        y: center.y * art.scale - diameter * 0.5,
        width: diameter,
        height: diameter,
    };
    advanced_image::Renderer::draw(renderer, handle.clone(), image_bounds);
}

fn draw_doubler_knob_asset(renderer: &mut iced::Renderer, art: &DoublerArt, bounds: Size) {
    const FRAME_COUNT: usize = 121;
    let index = ((FRAME_COUNT - 1) as f32 * art.value.clamp(0.0, 1.0)).round() as usize;
    let Some(handle) = global_copper_knob_handles().get(index) else {
        return;
    };

    let logical_size = unscale_size(bounds, art.scale);
    let center = Point::new(logical_size.width * 0.5, 44.0);
    let diameter = 50.0 * art.scale;
    let image_bounds = Rectangle {
        x: center.x * art.scale - diameter * 0.5,
        y: center.y * art.scale - diameter * 0.5,
        width: diameter,
        height: diameter,
    };
    advanced_image::Renderer::draw(renderer, handle.clone(), image_bounds);
}

fn draw_eq_panel_asset(renderer: &mut iced::Renderer, scale: f32) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: "assets/effects/eq-rose-gold-clean-v2@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 2816,
        pixel_height: 784,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };
    advanced_image::Renderer::draw(
        renderer,
        handle,
        Rectangle {
            x: (EQ_RENDER_OFFSET_X + EQ_PANEL_X * EQ_RENDER_SCALE) * scale,
            y: (EQ_RENDER_OFFSET_Y + EQ_PANEL_Y * EQ_RENDER_SCALE) * scale,
            width: EQ_PANEL_W * EQ_RENDER_SCALE * scale,
            height: EQ_PANEL_H * EQ_RENDER_SCALE * scale,
        },
    );
}

fn draw_eq_slider_cap_assets(renderer: &mut iced::Renderer, art: &EqArt) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: "assets/effects/eq-slider-cap-v2@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 116,
        pixel_height: 80,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };

    let scale = art.scale;
    let render_scale = scale * EQ_RENDER_SCALE;
    let width = 58.0 * render_scale;
    let height = 40.0 * render_scale;
    let bottom = EQ_SLIDER_TOP + EQ_SLIDER_H;
    for index in 0..EQ_BAND_COUNT {
        let x = EQ_FIRST_SLIDER_X + index as f32 * EQ_SLIDER_SPACING;
        let value = art.eq.bands[index].clamp(0.0, 1.0);
        let y = bottom - value * EQ_SLIDER_H;
        advanced_image::Renderer::draw(
            renderer,
            handle.clone(),
            Rectangle {
                x: (EQ_RENDER_OFFSET_X + (x - 29.0) * EQ_RENDER_SCALE) * scale,
                y: (EQ_RENDER_OFFSET_Y + (y - 20.0) * EQ_RENDER_SCALE) * scale,
                width,
                height,
            },
        );
    }
}

fn draw_eq_filter_knob_assets(renderer: &mut iced::Renderer, art: &EqArt) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: "assets/effects/eq-filter-knob-v2@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 240,
        pixel_height: 240,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };

    let scale = art.scale;
    let render_scale = scale * EQ_RENDER_SCALE;
    let size = 120.0 * render_scale;
    for center in [eq_hpf_knob_center(), eq_lpf_knob_center()] {
        advanced_image::Renderer::draw(
            renderer,
            handle.clone(),
            Rectangle {
                x: (EQ_RENDER_OFFSET_X + (center.x - 60.0) * EQ_RENDER_SCALE) * scale,
                y: (EQ_RENDER_OFFSET_Y + (center.y - 60.0) * EQ_RENDER_SCALE) * scale,
                width: size,
                height: size,
            },
        );
    }
}

fn draw_eq_power_led_asset(renderer: &mut iced::Renderer, art: &EqArt) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: if art.eq.enabled {
            "assets/effects/eq-power-led-on-v2@2x.png"
        } else {
            "assets/effects/eq-power-led-v2@2x.png"
        },
        format: RenderAssetFormat::PngRgba,
        pixel_width: 92,
        pixel_height: 92,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };

    let center = eq_power_led_center();
    let scale = art.scale;
    let render_scale = scale * EQ_RENDER_SCALE;
    advanced_image::Renderer::draw(
        renderer,
        handle,
        Rectangle {
            x: (EQ_RENDER_OFFSET_X + (center.x - 23.0) * EQ_RENDER_SCALE) * scale,
            y: (EQ_RENDER_OFFSET_Y + (center.y - 23.0) * EQ_RENDER_SCALE) * scale,
            width: 46.0 * render_scale,
            height: 46.0 * render_scale,
        },
    );
}

fn draw_eq_power_switch_asset(renderer: &mut iced::Renderer, scale: f32) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: "assets/effects/eq-power-switch-v2@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 130,
        pixel_height: 160,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };

    let center = eq_power_switch_center();
    let render_scale = scale * EQ_RENDER_SCALE;
    advanced_image::Renderer::draw(
        renderer,
        handle,
        Rectangle {
            x: (EQ_RENDER_OFFSET_X + (center.x - 32.5) * EQ_RENDER_SCALE) * scale,
            y: (EQ_RENDER_OFFSET_Y + (center.y - 50.0) * EQ_RENDER_SCALE) * scale,
            width: 65.0 * render_scale,
            height: 80.0 * render_scale,
        },
    );
}

fn draw_amp_asset(renderer: &mut iced::Renderer, art: &AmpArt, bounds: Size) {
    if art.circuit_view || !render_assets_enabled() {
        return;
    }

    let render_spec = amp_render_spec(art.app_profile, art.amp_model);
    let Some(asset) = render_spec.asset else {
        return;
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };
    let logical_size = unscale_size(bounds, art.scale);
    let image_bounds = scaled_rectangle(amp_render_bounds(logical_size, render_spec), art.scale);
    advanced_image::Renderer::draw(renderer, handle, image_bounds);
}

fn draw_cab_asset(renderer: &mut iced::Renderer, art: &CabArt, bounds: Size) {
    if !render_assets_enabled() {
        return;
    }

    let asset = RenderAssetSpec {
        path: "assets/cabs/greybound-2x12@2x.png",
        format: RenderAssetFormat::PngRgba,
        pixel_width: 1821,
        pixel_height: 864,
    };
    let Some(handle) = render_asset_handle(asset) else {
        return;
    };

    let logical_size = unscale_size(bounds, art.scale);
    advanced_image::Renderer::draw(
        renderer,
        handle,
        scaled_rectangle(cab_render_bounds(logical_size), art.scale),
    );
}

fn draw_record_background(renderer: &iced::Renderer, art: &RecordArt, bounds: Size) -> Geometry {
    let mut frame = Frame::new(renderer, bounds);
    frame.scale(art.scale);
    draw_stage_background(&mut frame, unscale_size(bounds, art.scale));
    frame.into_geometry()
}

fn draw_record_assets(renderer: &mut iced::Renderer, art: &RecordArt, bounds: Size) {
    let logical_size = unscale_size(bounds, art.scale);
    let layout = record_layout(logical_size);

    for (asset, rect) in [
        (
            RenderAssetSpec {
                path: "assets/record/atomic/rack-chassis.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1757,
                pixel_height: 425,
            },
            layout.rack,
        ),
        (
            RenderAssetSpec {
                path: "assets/record/atomic/vu-meter.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1331,
                pixel_height: 705,
            },
            layout.left_meter,
        ),
        (
            RenderAssetSpec {
                path: "assets/record/atomic/vu-meter.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1331,
                pixel_height: 705,
            },
            layout.right_meter,
        ),
        (
            RenderAssetSpec {
                path: "assets/record/atomic/timecode-display.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1230,
                pixel_height: 364,
            },
            layout.display,
        ),
        (
            RenderAssetSpec {
                path: "assets/record/atomic/rec-button.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 958,
                pixel_height: 965,
            },
            layout.record_button,
        ),
    ] {
        let Some(handle) = render_asset_handle(asset) else {
            continue;
        };
        advanced_image::Renderer::draw(renderer, handle, scaled_rectangle(rect, art.scale));
    }
}

fn draw_amp_control_assets(renderer: &mut iced::Renderer, art: &AmpArt, bounds: Size) {
    if art.circuit_view || !render_assets_enabled() {
        return;
    }

    let logical_size = unscale_size(bounds, art.scale);
    let render_spec = amp_render_spec(art.app_profile, art.amp_model);
    let render_bounds = amp_render_bounds(logical_size, render_spec);
    let origin = Point::new(render_bounds.x, render_bounds.y);
    let size = Size::new(render_bounds.width, render_bounds.height);

    for control in render_spec.controls {
        let Some(asset) = control.asset else {
            continue;
        };
        let value = match control.role {
            RenderControlRole::Parameter(kind) => art.amp.control_value(kind),
            RenderControlRole::Bypass => bypass_asset_value(art.amp.bypassed),
        };
        let Some(handle) = render_control_asset_handle(asset, value) else {
            continue;
        };
        let center = render_control_center(control, origin, size);
        let diameter = control.radius * 2.45;
        let image_bounds = Rectangle {
            x: (center.x - diameter * 0.5) * art.scale,
            y: (center.y - diameter * 0.5) * art.scale,
            width: diameter * art.scale,
            height: diameter * art.scale,
        };
        advanced_image::Renderer::draw(renderer, handle, image_bounds);
    }

    if art.amp_model == AmpModel::Daybreaker50 {
        let Some(handle) = render_asset_handle(DAYBREAKER_INPUT_JACK_ASSET) else {
            return;
        };
        let center = Point::new(
            origin.x + size.width * 0.265,
            origin.y + size.height * 0.225,
        );
        let diameter = 54.0;
        advanced_image::Renderer::draw(
            renderer,
            handle,
            Rectangle {
                x: (center.x - diameter * 0.5) * art.scale,
                y: (center.y - diameter * 0.5) * art.scale,
                width: diameter * art.scale,
                height: diameter * art.scale,
            },
        );
    }
}

fn scaled_rectangle(rect: Rectangle, scale: f32) -> Rectangle {
    Rectangle {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

pub fn preload_render_assets() {
    if !render_assets_enabled() {
        return;
    }

    const PRELOAD_ASSETS: &[RenderAssetSpec] = &[
        RenderAssetSpec {
            path: "assets/surfaces/header-brushed-gunmetal@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1774,
            pixel_height: 887,
        },
        RenderAssetSpec {
            path: "assets/surfaces/amp-studio@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1672,
            pixel_height: 941,
        },
        RenderAssetSpec {
            path: "assets/pedals/lumen@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2172,
        },
        RenderAssetSpec {
            path: "assets/pedals/minotaur-v2@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 914,
            pixel_height: 1721,
        },
        RenderAssetSpec {
            path: "assets/pedals/springfield@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2260,
        },
        RenderAssetSpec {
            path: "assets/pedals/auralith@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2172,
        },
        RenderAssetSpec {
            path: "assets/amps/nox30-cropped@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1620,
            pixel_height: 856,
        },
        RenderAssetSpec {
            path: "assets/amps/daybreaker-50-stack-badged-v4@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 2480,
            pixel_height: 1000,
        },
        RenderAssetSpec {
            path: "assets/amps/nam-loader-rack@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 2480,
            pixel_height: 1000,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/daybreaker-black-brass@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/buttons/daybreaker-power-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/buttons/daybreaker-power-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/daybreaker-jewel-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/daybreaker-jewel-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        DAYBREAKER_INPUT_JACK_ASSET,
        RenderAssetSpec {
            path: "assets/cabs/greybound-2x12@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1821,
            pixel_height: 864,
        },
        RenderAssetSpec {
            path: "assets/record/atomic/rack-chassis.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1757,
            pixel_height: 425,
        },
        RenderAssetSpec {
            path: "assets/record/atomic/vu-meter.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1331,
            pixel_height: 705,
        },
        RenderAssetSpec {
            path: "assets/record/atomic/timecode-display.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1230,
            pixel_height: 364,
        },
        RenderAssetSpec {
            path: "assets/record/atomic/rec-button.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 958,
            pixel_height: 965,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-rose-gold-clean-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 2816,
            pixel_height: 784,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-slider-cap-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 116,
            pixel_height: 80,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-filter-knob-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 240,
            pixel_height: 240,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-power-led-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 92,
            pixel_height: 92,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-power-led-on-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 92,
            pixel_height: 92,
        },
        RenderAssetSpec {
            path: "assets/effects/eq-power-switch-v2@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 130,
            pixel_height: 160,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/lumen-pearl@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/buttons/lumen-footswitch@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/lumen-jewel-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/lumen-jewel-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/minotaur-ivory@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/buttons/minotaur-silver-footswitch@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/nox30-black-dial@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1024,
            pixel_height: 1024,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/auralith-black@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1024,
            pixel_height: 1024,
        },
        RenderAssetSpec {
            path: "assets/controls/buttons/auralith-footswitch@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/auralith-jewel-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/auralith-jewel-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/knobs/springfield-stainless@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 512,
            pixel_height: 512,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/minotaur-jewel-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/minotaur-jewel-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/springfield-jewel-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/springfield-jewel-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/nox30-power-off@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
        RenderAssetSpec {
            path: "assets/controls/leds/nox30-power-on@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 256,
            pixel_height: 256,
        },
    ];

    for asset in PRELOAD_ASSETS {
        let _ = render_asset_handle(*asset);
    }

    let _ = minotaur_ivory_knob_handles().len();
    let _ = nox30_black_dial_knob_handles().len();
    let _ = daybreaker_black_brass_knob_handles().len();
    let _ = auralith_black_knob_handles().len();
    let _ = lumen_pearl_knob_handles().len();
    let _ = springfield_stainless_knob_handles().len();
}

fn render_assets_enabled() -> bool {
    true
}

fn decoded_embedded_png_handle(
    bytes: &'static [u8],
    expected_width: u32,
    expected_height: u32,
    label: &'static str,
) -> advanced_image::Handle {
    let image = image::load_from_memory(bytes)
        .unwrap_or_else(|error| panic!("embedded render asset {label} must decode: {error}"))
        .to_rgba8();
    debug_assert_eq!(image.width(), expected_width, "{label} width changed");
    debug_assert_eq!(image.height(), expected_height, "{label} height changed");
    let width = image.width();
    let height = image.height();
    advanced_image::Handle::from_pixels(width, height, image.into_raw())
}

fn render_asset_handle(asset: RenderAssetSpec) -> Option<advanced_image::Handle> {
    macro_rules! decoded_handle {
        ($path:literal, $width:literal, $height:literal) => {{
            static HANDLE: OnceLock<advanced_image::Handle> = OnceLock::new();
            Some(
                HANDLE
                    .get_or_init(|| {
                        decoded_embedded_png_handle(
                            include_bytes!($path).as_slice(),
                            $width,
                            $height,
                            $path,
                        )
                    })
                    .clone(),
            )
        }};
    }

    match asset.path {
        "assets/surfaces/header-brushed-gunmetal@2x.png" => decoded_handle!(
            "../assets/surfaces/header-brushed-gunmetal@2x.png",
            1774,
            887
        ),
        "assets/surfaces/amp-studio@2x.png" => {
            decoded_handle!("../assets/surfaces/amp-studio@2x.png", 1672, 941)
        }
        "assets/surfaces/pedalboard-studio@2x.png" => {
            decoded_handle!("../assets/surfaces/pedalboard-studio@2x.png", 1807, 870)
        }
        "assets/pedals/lumen@4x.png" => {
            decoded_handle!("../assets/pedals/lumen@4x.png", 1200, 2172)
        }
        "assets/pedals/minotaur-v2@4x.png" => {
            decoded_handle!("../assets/pedals/minotaur-v2@4x.png", 914, 1721)
        }
        "assets/pedals/monarch@4x.png" => {
            decoded_handle!("../assets/pedals/monarch@4x.png", 1200, 2172)
        }
        "assets/pedals/muffin@4x.png" => {
            decoded_handle!("../assets/pedals/muffin@4x.png", 1200, 2172)
        }
        "assets/pedals/springfield@4x.png" => {
            decoded_handle!("../assets/pedals/springfield@4x.png", 1200, 2260)
        }
        "assets/pedals/auralith@4x.png" => {
            decoded_handle!("../assets/pedals/auralith@4x.png", 1200, 2172)
        }
        "assets/amps/nox30-cropped@2x.png" => {
            decoded_handle!("../assets/amps/nox30-cropped@2x.png", 1620, 856)
        }
        "assets/amps/daybreaker-50-stack-badged-v4@2x.png" => {
            decoded_handle!(
                "../assets/amps/daybreaker-50-stack-badged-v4@2x.png",
                2480,
                1000
            )
        }
        "assets/amps/nam-loader-rack@2x.png" => {
            decoded_handle!("../assets/amps/nam-loader-rack@2x.png", 2480, 1000)
        }
        "assets/controls/knobs/daybreaker-black-brass@2x.png" => {
            decoded_handle!(
                "../assets/controls/knobs/daybreaker-black-brass@2x.png",
                512,
                512
            )
        }
        "assets/controls/buttons/daybreaker-power-off@2x.png" => {
            decoded_handle!(
                "../assets/controls/buttons/daybreaker-power-off@2x.png",
                512,
                512
            )
        }
        "assets/controls/buttons/daybreaker-power-on@2x.png" => {
            decoded_handle!(
                "../assets/controls/buttons/daybreaker-power-on@2x.png",
                512,
                512
            )
        }
        "assets/controls/leds/daybreaker-jewel-off@2x.png" => {
            decoded_handle!(
                "../assets/controls/leds/daybreaker-jewel-off@2x.png",
                256,
                256
            )
        }
        "assets/controls/leds/daybreaker-jewel-on@2x.png" => {
            decoded_handle!(
                "../assets/controls/leds/daybreaker-jewel-on@2x.png",
                256,
                256
            )
        }
        "assets/controls/jacks/daybreaker-input@2x.png" => {
            decoded_handle!("../assets/controls/jacks/daybreaker-input@2x.png", 512, 512)
        }
        "assets/cabs/greybound-2x12@2x.png" => {
            decoded_handle!("../assets/cabs/greybound-2x12@2x.png", 1821, 864)
        }
        "assets/record/atomic/rack-chassis.png" => {
            decoded_handle!("../assets/record/atomic/rack-chassis.png", 1757, 425)
        }
        "assets/record/atomic/vu-meter.png" => {
            decoded_handle!("../assets/record/atomic/vu-meter.png", 1331, 705)
        }
        "assets/record/atomic/timecode-display.png" => {
            decoded_handle!("../assets/record/atomic/timecode-display.png", 1230, 364)
        }
        "assets/record/atomic/rec-button.png" => {
            decoded_handle!("../assets/record/atomic/rec-button.png", 958, 965)
        }
        "assets/effects/eq-rose-gold@2x.png" => {
            decoded_handle!("../assets/effects/eq-rose-gold@2x.png", 2816, 784)
        }
        "assets/effects/eq-rose-gold-clean-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-rose-gold-clean-v2@2x.png", 2816, 784)
        }
        "assets/effects/eq-slider-cap@2x.png" => {
            decoded_handle!("../assets/effects/eq-slider-cap@2x.png", 116, 80)
        }
        "assets/effects/eq-slider-cap-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-slider-cap-v2@2x.png", 116, 80)
        }
        "assets/effects/eq-filter-knob-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-filter-knob-v2@2x.png", 240, 240)
        }
        "assets/effects/eq-power-led-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-power-led-v2@2x.png", 92, 92)
        }
        "assets/effects/eq-power-led-on-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-power-led-on-v2@2x.png", 92, 92)
        }
        "assets/effects/eq-power-switch-v2@2x.png" => {
            decoded_handle!("../assets/effects/eq-power-switch-v2@2x.png", 130, 160)
        }
        "assets/controls/knobs/minotaur-ivory@2x.png" => {
            decoded_handle!("../assets/controls/knobs/minotaur-ivory@2x.png", 512, 512)
        }
        "assets/controls/knobs/monarch-brass@2x.png" => {
            decoded_handle!("../assets/controls/knobs/monarch-brass@2x.png", 512, 512)
        }
        "assets/controls/buttons/minotaur-silver-footswitch@2x.png" => decoded_handle!(
            "../assets/controls/buttons/minotaur-silver-footswitch@2x.png",
            512,
            512
        ),
        "assets/controls/buttons/monarch-brass-footswitch@2x.png" => decoded_handle!(
            "../assets/controls/buttons/monarch-brass-footswitch@2x.png",
            512,
            512
        ),
        "assets/controls/knobs/muffin-black-brass@2x.png" => {
            decoded_handle!(
                "../assets/controls/knobs/muffin-black-brass@2x.png",
                512,
                512
            )
        }
        "assets/controls/knobs/nox30-black-dial@2x.png" => decoded_handle!(
            "../assets/controls/knobs/nox30-black-dial@2x.png",
            1024,
            1024
        ),
        "assets/controls/knobs/auralith-black@2x.png" => {
            decoded_handle!("../assets/controls/knobs/auralith-black@2x.png", 1024, 1024)
        }
        "assets/controls/knobs/lumen-pearl@2x.png" => {
            decoded_handle!("../assets/controls/knobs/lumen-pearl@2x.png", 512, 512)
        }
        "assets/controls/buttons/lumen-footswitch@2x.png" => decoded_handle!(
            "../assets/controls/buttons/lumen-footswitch@2x.png",
            512,
            512
        ),
        "assets/controls/buttons/auralith-footswitch@2x.png" => decoded_handle!(
            "../assets/controls/buttons/auralith-footswitch@2x.png",
            512,
            512
        ),
        "assets/controls/buttons/muffin-footswitch@2x.png" => decoded_handle!(
            "../assets/controls/buttons/muffin-footswitch@2x.png",
            512,
            512
        ),
        "assets/controls/leds/lumen-jewel-off@2x.png" => {
            decoded_handle!("../assets/controls/leds/lumen-jewel-off@2x.png", 256, 256)
        }
        "assets/controls/leds/lumen-jewel-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/lumen-jewel-on@2x.png", 256, 256)
        }
        "assets/controls/leds/auralith-jewel-off@2x.png" => decoded_handle!(
            "../assets/controls/leds/auralith-jewel-off@2x.png",
            256,
            256
        ),
        "assets/controls/leds/auralith-jewel-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/auralith-jewel-on@2x.png", 256, 256)
        }
        "assets/controls/leds/muffin-jewel-off@2x.png" => {
            decoded_handle!("../assets/controls/leds/muffin-jewel-off@2x.png", 256, 256)
        }
        "assets/controls/leds/muffin-jewel-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/muffin-jewel-on@2x.png", 256, 256)
        }
        "assets/controls/knobs/springfield-stainless@2x.png" => decoded_handle!(
            "../assets/controls/knobs/springfield-stainless@2x.png",
            512,
            512
        ),
        "assets/controls/leds/minotaur-jewel-off@2x.png" => decoded_handle!(
            "../assets/controls/leds/minotaur-jewel-off@2x.png",
            256,
            256
        ),
        "assets/controls/leds/minotaur-jewel-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/minotaur-jewel-on@2x.png", 256, 256)
        }
        "assets/controls/leds/monarch-jewel-off@2x.png" => {
            decoded_handle!("../assets/controls/leds/monarch-jewel-off@2x.png", 256, 256)
        }
        "assets/controls/leds/monarch-jewel-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/monarch-jewel-on@2x.png", 256, 256)
        }
        "assets/controls/leds/springfield-jewel-off@2x.png" => decoded_handle!(
            "../assets/controls/leds/springfield-jewel-off@2x.png",
            256,
            256
        ),
        "assets/controls/leds/springfield-jewel-on@2x.png" => decoded_handle!(
            "../assets/controls/leds/springfield-jewel-on@2x.png",
            256,
            256
        ),
        "assets/controls/leds/nox30-power-off@2x.png" => {
            decoded_handle!("../assets/controls/leds/nox30-power-off@2x.png", 256, 256)
        }
        "assets/controls/leds/nox30-power-on@2x.png" => {
            decoded_handle!("../assets/controls/leds/nox30-power-on@2x.png", 256, 256)
        }
        _ => None,
    }
}

fn render_control_asset_handle(
    asset: RenderControlAssetSpec,
    value: f32,
) -> Option<advanced_image::Handle> {
    if asset.image.path == "assets/controls/knobs/minotaur-ivory@2x.png" && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return minotaur_ivory_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/monarch-brass@2x.png" && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return monarch_brass_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/muffin-black-brass@2x.png"
        && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return muffin_black_brass_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/nox30-black-dial@2x.png"
        && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return nox30_black_dial_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/daybreaker-black-brass@2x.png"
        && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return daybreaker_black_brass_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/auralith-black@2x.png" && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return auralith_black_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/springfield-stainless@2x.png"
        && asset.rotation.is_some()
    {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return springfield_stainless_knob_handles().get(index).cloned();
    }
    if asset.image.path == "assets/controls/knobs/lumen-pearl@2x.png" && asset.rotation.is_some() {
        const FRAME_COUNT: usize = 121;
        let index = ((FRAME_COUNT - 1) as f32 * value.clamp(0.0, 1.0)).round() as usize;
        return lumen_pearl_knob_handles().get(index).cloned();
    }

    if value >= 0.5 {
        if let Some(active_image) = asset.active_image {
            return render_asset_handle(active_image);
        }
    }

    render_asset_handle(asset.image)
}

fn bypass_asset_value(bypassed: bool) -> f32 {
    if bypassed {
        0.0
    } else {
        1.0
    }
}

fn minotaur_ivory_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/minotaur-ivory@2x.png"
        ))
        .expect("embedded minotaur knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = MINOTAUR_IVORY_KNOB_ASSET
                    .rotation
                    .expect("minotaur knob asset must define a rotation range");
                let angle = (rotation.max_degrees - rotation.min_degrees).to_radians() * t;
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn monarch_brass_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/monarch-brass@2x.png"
        ))
        .expect("embedded Monarch knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();
        let rotation = MONARCH_BRASS_KNOB_ASSET
            .rotation
            .expect("Monarch knob asset must define a rotation range");

        (0..FRAME_COUNT)
            .map(|index| {
                let value = index as f32 / (FRAME_COUNT - 1) as f32;
                let angle = (rotation.max_degrees - rotation.min_degrees) * value;
                let pixels = rotate_rgba_pixels(&source, angle.to_radians());
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn muffin_black_brass_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/muffin-black-brass@2x.png"
        ))
        .expect("embedded Muffin knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();
        let rotation = MUFFIN_BLACK_BRASS_KNOB_ASSET
            .rotation
            .expect("Muffin knob asset must define a rotation range");

        (0..FRAME_COUNT)
            .map(|index| {
                let value = index as f32 / (FRAME_COUNT - 1) as f32;
                let angle = (rotation.min_degrees
                    + (rotation.max_degrees - rotation.min_degrees) * value)
                    .to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn nox30_black_dial_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/nox30-black-dial@2x.png"
        ))
        .expect("embedded nox30 knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = NOX30_BLACK_DIAL_KNOB_ASSET
                    .rotation
                    .expect("nox30 knob asset must define a rotation range");
                let angle = ((rotation.max_degrees - rotation.min_degrees) * t
                    + NOX30_BLACK_DIAL_ROTATION_OFFSET_DEGREES)
                    .to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn daybreaker_black_brass_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/daybreaker-black-brass@2x.png"
        ))
        .expect("embedded Daybreaker knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = DAYBREAKER_BLACK_BRASS_KNOB_ASSET
                    .rotation
                    .expect("Daybreaker knob asset must define a rotation range");
                let angle = (rotation.min_degrees
                    + (rotation.max_degrees - rotation.min_degrees) * t)
                    .to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn global_copper_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/global-copper@2x.png"
        ))
        .expect("embedded global copper knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let value = index as f32 / (FRAME_COUNT - 1) as f32;
                let angle = (value * 270.0).to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn auralith_black_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/auralith-black@2x.png"
        ))
        .expect("embedded auralith knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = AURALITH_BLACK_KNOB_ASSET
                    .rotation
                    .expect("auralith knob asset must define a rotation range");
                let angle = (rotation.min_degrees
                    + (rotation.max_degrees - rotation.min_degrees) * t)
                    .to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn springfield_stainless_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/springfield-stainless@2x.png"
        ))
        .expect("embedded springfield knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = SPRINGFIELD_STAINLESS_KNOB_ASSET
                    .rotation
                    .expect("springfield knob asset must define a rotation range");
                let angle = (rotation.max_degrees - rotation.min_degrees).to_radians() * t;
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn lumen_pearl_knob_handles() -> &'static [advanced_image::Handle] {
    static HANDLES: OnceLock<Vec<advanced_image::Handle>> = OnceLock::new();
    HANDLES.get_or_init(|| {
        const FRAME_COUNT: usize = 121;
        let source = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/lumen-pearl@2x.png"
        ))
        .expect("embedded lumen knob asset must decode")
        .to_rgba8();
        let width = source.width();
        let height = source.height();

        (0..FRAME_COUNT)
            .map(|index| {
                let t = index as f32 / (FRAME_COUNT - 1) as f32;
                let rotation = LUMEN_PEARL_KNOB_ASSET
                    .rotation
                    .expect("lumen knob asset must define a rotation range");
                let angle = (rotation.min_degrees
                    + (rotation.max_degrees - rotation.min_degrees) * t)
                    .to_radians();
                let pixels = rotate_rgba_pixels(&source, angle);
                advanced_image::Handle::from_pixels(width, height, pixels)
            })
            .collect()
    })
}

fn rotate_rgba_pixels(source: &image::RgbaImage, angle: f32) -> Vec<u8> {
    let width = source.width() as usize;
    let height = source.height() as usize;
    let cx = (width as f32 - 1.0) * 0.5;
    let cy = (height as f32 - 1.0) * 0.5;
    let cos = angle.cos();
    let sin = angle.sin();
    let raw = source.as_raw();
    let mut out = vec![0_u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let sx = cos * dx + sin * dy + cx;
            let sy = -sin * dx + cos * dy + cy;
            let pixel = sample_rgba_bilinear(raw, width, height, sx, sy);
            let offset = (y * width + x) * 4;
            out[offset..offset + 4].copy_from_slice(&pixel);
        }
    }

    out
}

fn sample_rgba_bilinear(raw: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
    if x < 0.0 || y < 0.0 || x >= (width - 1) as f32 || y >= (height - 1) as f32 {
        return [0, 0, 0, 0];
    }

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let p00 = raw_pixel(raw, width, x0, y0);
    let p10 = raw_pixel(raw, width, x1, y0);
    let p01 = raw_pixel(raw, width, x0, y1);
    let p11 = raw_pixel(raw, width, x1, y1);
    let mut out = [0_u8; 4];

    for channel in 0..4 {
        let top = p00[channel] as f32 * (1.0 - tx) + p10[channel] as f32 * tx;
        let bottom = p01[channel] as f32 * (1.0 - tx) + p11[channel] as f32 * tx;
        out[channel] = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
    }

    out
}

fn raw_pixel(raw: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * width + x) * 4;
    [
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ]
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

                if let Some(model) = hit_test_amp_spine(
                    self.app_profile,
                    unscale_size(bounds.size(), self.scale),
                    position,
                ) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SelectAmpModel(model)),
                    );
                }

                if !self.circuit_view {
                    if let Some((index, control)) = hit_test_pedal_knob(
                        self.app_profile,
                        &self.devices,
                        unscale_size(bounds.size(), self.scale),
                        position,
                    ) {
                        let start_value = self
                            .devices
                            .iter()
                            .find(|slot| slot.source_index == index)
                            .map(|slot| slot.device.control_value(control))
                            .unwrap_or(0.0);
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

                    if let Some(source_index) = hit_test_pedal_footswitch(
                        self.app_profile,
                        &self.devices,
                        unscale_size(bounds.size(), self.scale),
                        position,
                    ) {
                        return (
                            canvas::event::Status::Captured,
                            Some(Message::ToggleDeviceBypass(source_index)),
                        );
                    }
                }

                if let Some(index) = hit_test_pedal(
                    self.devices.len(),
                    unscale_size(bounds.size(), self.scale),
                    position,
                ) {
                    let source_index = self.devices[index].source_index;
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SelectDevice(source_index)),
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

        let layout = board_layout(self.devices.len(), logical_size);
        let y = pedal_board_y(logical_size, layout.pedal_h);

        for (index, slot) in self.devices.iter().enumerate() {
            let device = &slot.device;
            let x = layout.start_x + index as f32 * (layout.pedal_w + layout.gap);
            let palette = match device.model {
                DeviceModel::Lumen => Color::from_rgb(0.70, 0.73, 0.76),
                DeviceModel::Muffin => Color::from_rgb(0.31, 0.19, 0.39),
                DeviceModel::Minotaur => Color::from_rgb(0.73, 0.65, 0.47),
                DeviceModel::Monarch => Color::from_rgb(0.24, 0.33, 0.44),
                DeviceModel::Nox30 | DeviceModel::WideCombo | DeviceModel::LeadHead => PEDAL_CREAM,
                DeviceModel::Springfield => PEDAL_PEACH,
                DeviceModel::DelayFx => Color::from_rgb(0.47, 0.50, 0.68),
                DeviceModel::ReverbFx => Color::from_rgb(0.41, 0.48, 0.58),
                DeviceModel::CabIr => PEDAL_SAGE,
            };
            draw_pedal(
                &mut frame,
                Point::new(x, y),
                Size::new(layout.pedal_w, layout.pedal_h),
                self.app_profile,
                device,
                palette,
                slot.source_index == self.selected_index,
                self.circuit_view,
            );
        }

        draw_stage_circuit_toggle(&mut frame, logical_size, self.circuit_view);
        draw_amp_spine(&mut frame, logical_size, self.app_profile, self.amp_model);

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
    app_profile: AppProfile,
    amp: DeviceState,
    amp_model: AmpModel,
    nam_loader: NamLoaderState,
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

                if let Some(model) = hit_test_amp_spine(
                    self.app_profile,
                    unscale_size(bounds.size(), self.scale),
                    position,
                ) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SelectAmpModel(model)),
                    );
                }

                if self.circuit_view {
                    return (canvas::event::Status::Ignored, None);
                }

                if self.amp_model == AmpModel::NamLoader
                    && hit_test_nam_loader_button(unscale_size(bounds.size(), self.scale), position)
                {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::LoadNamRequested),
                    );
                }

                if hit_test_amp_bypass(
                    self.app_profile,
                    unscale_size(bounds.size(), self.scale),
                    self.amp_model,
                    position,
                ) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::ToggleBypass(!self.amp.bypassed)),
                    );
                }

                if let Some(control) = hit_test_amp_knob(
                    self.app_profile,
                    unscale_size(bounds.size(), self.scale),
                    self.amp_model,
                    position,
                ) {
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
        let render_spec = amp_render_spec(self.app_profile, self.amp_model);
        if self.circuit_view {
            draw_amp_circuit(&mut frame, logical_size, self.app_profile, self.amp_model);
        } else if render_spec.asset.is_none() || !render_assets_enabled() {
            draw_amp_head(&mut frame, logical_size, &self.amp);
        }
        if !self.circuit_view && self.amp_model == AmpModel::Daybreaker50 {
            draw_daybreaker_amp_overlay(&mut frame, logical_size);
        }
        if !self.circuit_view && self.amp_model == AmpModel::NamLoader {
            draw_nam_loader_overlay(&mut frame, logical_size, &self.nam_loader);
        }
        draw_stage_circuit_toggle(&mut frame, logical_size, self.circuit_view);
        draw_amp_spine(&mut frame, logical_size, self.app_profile, self.amp_model);
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone)]
struct CabArt {
    app_profile: AppProfile,
    cab: DeviceState,
    amp_model: AmpModel,
    scale: f32,
}

impl canvas::Program<Message> for CabArt {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let Some(position) = cursor
                .position_in(bounds)
                .map(|p| unscale_point(p, self.scale))
            else {
                return (canvas::event::Status::Ignored, None);
            };
            if let Some(model) = hit_test_amp_spine(
                self.app_profile,
                unscale_size(bounds.size(), self.scale),
                position,
            ) {
                return (
                    canvas::event::Status::Captured,
                    Some(Message::SelectAmpModel(model)),
                );
            }
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
        let cab_bounds = cab_render_bounds(logical_size);
        draw_text(
            &mut frame,
            self.cab.model.title(),
            Point::new(cab_bounds.x + cab_bounds.width * 0.5, cab_bounds.y - 22.0),
            15.0,
            MUTED_INK,
            Horizontal::Center,
        );
        draw_text(
            &mut frame,
            "lab/references/tone3000-irs/celestion.wav",
            Point::new(
                cab_bounds.x + cab_bounds.width * 0.5,
                cab_bounds.y + cab_bounds.height + 24.0,
            ),
            13.0,
            MUTED_INK,
            Horizontal::Center,
        );
        draw_amp_spine(&mut frame, logical_size, self.app_profile, self.amp_model);
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone)]
struct CabCanvas {
    width: Length,
    height: Length,
    art: CabArt,
}

impl CabCanvas {
    fn new(art: CabArt) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for CabCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<()>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        let Some(canvas_event) = canvas_event else {
            return iced::event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<()>();
        let (status, message) = self
            .art
            .update(state, canvas_event, layout.bounds(), cursor);
        if let Some(message) = message {
            shell.publish(message);
        }
        status
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<()>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let background = draw_cab_background(renderer, &self.art, bounds.size());
            renderer.draw(vec![background]);
            draw_amp_studio_backdrop(renderer, bounds.size());
            draw_cab_asset(renderer, &self.art, bounds.size());
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
        });
    }
}

impl<'a> From<CabCanvas> for Element<'a, Message> {
    fn from(cab: CabCanvas) -> Self {
        Element::new(cab)
    }
}

#[derive(Debug, Clone)]
struct EqArt {
    app_profile: AppProfile,
    eq: EqState,
    amp_model: AmpModel,
    scale: f32,
}

#[derive(Debug, Clone, Copy)]
enum EqDragTarget {
    Band(usize),
    Hpf,
    Lpf,
}

#[derive(Debug, Clone, Copy)]
struct EqDragGesture {
    target: EqDragTarget,
    start_position: Point,
    start_value: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct EqDragState {
    gesture: Option<EqDragGesture>,
}

impl canvas::Program<Message> for EqArt {
    type State = EqDragState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(stage_position) = cursor
                    .position_in(bounds)
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                if let Some(model) = hit_test_amp_spine(
                    self.app_profile,
                    unscale_size(bounds.size(), self.scale),
                    stage_position,
                ) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SelectAmpModel(model)),
                    );
                }
                let position = Point::new(
                    (stage_position.x - EQ_RENDER_OFFSET_X) / EQ_RENDER_SCALE,
                    (stage_position.y - EQ_RENDER_OFFSET_Y) / EQ_RENDER_SCALE,
                );
                if hit_test_eq_toggle(position) {
                    return (canvas::event::Status::Captured, Some(Message::ToggleEq));
                }
                if hit_test_eq_hpf(position) {
                    state.gesture = Some(EqDragGesture {
                        target: EqDragTarget::Hpf,
                        start_position: position,
                        start_value: self.eq.hpf,
                    });
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqHpf(self.eq.hpf)),
                    );
                }
                if hit_test_eq_lpf(position) {
                    state.gesture = Some(EqDragGesture {
                        target: EqDragTarget::Lpf,
                        start_position: position,
                        start_value: self.eq.lpf,
                    });
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqLpf(self.eq.lpf)),
                    );
                }
                if let Some((index, value)) = eq_slider_value_at(position) {
                    state.gesture = Some(EqDragGesture {
                        target: EqDragTarget::Band(index),
                        start_position: position,
                        start_value: value,
                    });
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqBand { index, value }),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(gesture) = state.gesture else {
                    return (canvas::event::Status::Ignored, None);
                };
                let Some(stage_position) = cursor
                    .position_in(bounds)
                    .map(|position| unscale_point(position, self.scale))
                else {
                    return (canvas::event::Status::Ignored, None);
                };
                let position = Point::new(
                    (stage_position.x - EQ_RENDER_OFFSET_X) / EQ_RENDER_SCALE,
                    (stage_position.y - EQ_RENDER_OFFSET_Y) / EQ_RENDER_SCALE,
                );
                return match gesture.target {
                    EqDragTarget::Band(index) => (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqBand {
                            index,
                            value: eq_value_from_y(position.y),
                        }),
                    ),
                    EqDragTarget::Hpf => (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqHpf(eq_dragged_filter_value(
                            gesture, position,
                        ))),
                    ),
                    EqDragTarget::Lpf => (
                        canvas::event::Status::Captured,
                        Some(Message::SetEqLpf(eq_dragged_filter_value(
                            gesture, position,
                        ))),
                    ),
                };
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
        let size = unscale_size(bounds.size(), self.scale);
        frame.with_save(|frame| {
            frame.translate(Vector::new(EQ_RENDER_OFFSET_X, EQ_RENDER_OFFSET_Y));
            frame.scale(EQ_RENDER_SCALE);
            draw_eq_panel(frame, size, &self.eq);
        });
        draw_amp_spine(&mut frame, size, self.app_profile, self.amp_model);
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
    let gap = 74.0;
    let stage_width = amp_stage_width(size);
    let available_width = stage_width - 86.0 - gap * (count - 1.0);
    let pedal_w = (available_width / count)
        .min(PEDAL_STANDARD_WIDTH)
        .max(220.0);
    let pedal_h = (pedal_w * (PEDAL_STANDARD_HEIGHT / PEDAL_STANDARD_WIDTH))
        .min(size.height - 44.0)
        .max(420.0);
    let total = pedal_w * count + gap * (count - 1.0);

    BoardLayout {
        start_x: (stage_width - total) * 0.5,
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
    Point::new((amp_spine_bounds(size).x - 34.0).max(32.0), 32.0)
}

fn hit_test_stage_circuit_toggle(size: Size, position: Point) -> bool {
    let center = stage_circuit_toggle_center(size);
    position.x >= center.x - 30.0
        && position.x <= center.x + 30.0
        && position.y >= center.y - 26.0
        && position.y <= center.y + 32.0
}

fn device_render_spec(app_profile: AppProfile, model: DeviceModel) -> &'static ModelRenderSpec {
    app_profile
        .device_descriptor_for_model(model)
        .map(|descriptor| descriptor.render)
        .unwrap_or_else(|| fallback_device_render_spec(model))
}

fn amp_render_spec(app_profile: AppProfile, model: AmpModel) -> &'static ModelRenderSpec {
    app_profile
        .amp_descriptor_for_model(model)
        .map(|descriptor| descriptor.render)
        .unwrap_or_else(|| fallback_amp_render_spec(model))
}

fn fallback_amp_render_spec(model: AmpModel) -> &'static ModelRenderSpec {
    match model {
        AmpModel::Nox30 => &NOX30_AMP_RENDER_SPEC,
        AmpModel::NamLoader => &NAM_LOADER_AMP_RENDER_SPEC,
        AmpModel::Daybreaker50 => &DAYBREAKER_AMP_RENDER_SPEC,
        AmpModel::WideCombo => &WIDE_COMBO_AMP_RENDER_SPEC,
        AmpModel::LeadHead => &LEAD_HEAD_AMP_RENDER_SPEC,
    }
}

fn amp_render_bounds(size: Size, render_spec: &ModelRenderSpec) -> Rectangle {
    let stage_width = amp_stage_width(size);
    let max_width = render_spec
        .surface
        .logical_width
        .min((stage_width - 80.0).max(1.0));
    let surface_ratio = render_spec.surface.logical_height / render_spec.surface.logical_width;
    let max_height = (size.height - 112.0).max(1.0);
    let mut width = max_width;
    let mut height = width * surface_ratio;
    if height > max_height {
        height = max_height;
        width = height / surface_ratio;
    }

    Rectangle {
        x: (stage_width - width) * 0.5,
        y: (size.height - height).max(0.0),
        width,
        height,
    }
}

fn cab_render_bounds(size: Size) -> Rectangle {
    const CAB_ASPECT_RATIO: f32 = 864.0 / 1821.0;

    let stage_width = amp_stage_width(size);
    let max_width = (stage_width - 72.0).min(900.0).max(1.0);
    let max_height = (size.height - 116.0).max(1.0);
    let mut width = max_width;
    let mut height = width * CAB_ASPECT_RATIO;
    if height > max_height {
        height = max_height;
        width = height / CAB_ASPECT_RATIO;
    }

    Rectangle {
        x: (stage_width - width) * 0.5,
        y: ((size.height - height) * 0.5).max(28.0),
        width,
        height,
    }
}

#[derive(Debug, Clone, Copy)]
struct RecordLayout {
    rack: Rectangle,
    left_meter: Rectangle,
    right_meter: Rectangle,
    display: Rectangle,
    record_button: Rectangle,
    waveform: Rectangle,
}

fn record_layout(size: Size) -> RecordLayout {
    let rack_width = (size.width * 0.61).clamp(720.0, 980.0);
    let rack_height = rack_width * (425.0 / 1757.0);
    let rack = Rectangle {
        x: ((size.width - rack_width) * 0.5).max(28.0),
        y: ((size.height - rack_height) * 0.5).max(34.0),
        width: rack_width,
        height: rack_height,
    };
    let row_center_y = rack.y + rack.height * 0.45;
    let meter_width = rack.width * 0.195;
    let meter_height = meter_width * (705.0 / 1331.0);
    let meter_y = row_center_y - meter_height * 0.5;
    let left_meter = Rectangle {
        x: rack.x + rack.width * 0.085,
        y: meter_y,
        width: meter_width,
        height: meter_height,
    };
    let right_meter = Rectangle {
        x: left_meter.x + meter_width * 1.04,
        ..left_meter
    };
    let display = Rectangle {
        x: rack.x + rack.width * 0.53,
        y: row_center_y - rack.height * 0.105,
        width: rack.width * 0.21,
        height: rack.height * 0.21,
    };
    let button_size = rack.height * 0.31;
    let record_button = Rectangle {
        x: rack.x + rack.width * 0.79,
        y: row_center_y - button_size * 0.5,
        width: button_size,
        height: button_size,
    };
    let waveform = Rectangle {
        x: rack.x + rack.width * 0.11,
        y: rack.y + rack.height * 0.72,
        width: rack.width * 0.75,
        height: rack.height * 0.12,
    };
    RecordLayout {
        rack,
        left_meter,
        right_meter,
        display,
        record_button,
        waveform,
    }
}

fn record_button_center(size: Size) -> Point {
    let button = record_layout(size).record_button;
    Point::new(
        button.x + button.width * 0.5,
        button.y + button.height * 0.5,
    )
}

fn record_button_radius(size: Size) -> f32 {
    record_layout(size).record_button.width * 0.5
}

fn fallback_device_render_spec(model: DeviceModel) -> &'static ModelRenderSpec {
    match model {
        DeviceModel::Lumen => &LUMEN_PEDAL_RENDER_SPEC,
        DeviceModel::Muffin => &MUFFIN_PEDAL_RENDER_SPEC,
        DeviceModel::Minotaur => &MINOTAUR_PEDAL_RENDER_SPEC,
        DeviceModel::Monarch => &MONARCH_PEDAL_RENDER_SPEC,
        DeviceModel::Nox30 => &NOX30_PEDAL_RENDER_SPEC,
        DeviceModel::WideCombo => &WIDE_COMBO_PEDAL_RENDER_SPEC,
        DeviceModel::LeadHead => &LEAD_HEAD_PEDAL_RENDER_SPEC,
        DeviceModel::Springfield => &SPRINGFIELD_PEDAL_RENDER_SPEC,
        DeviceModel::DelayFx => &DELAY_PEDAL_RENDER_SPEC,
        DeviceModel::ReverbFx => &REVERB_PEDAL_RENDER_SPEC,
        DeviceModel::CabIr => &CAB_RENDER_SPEC,
    }
}

fn render_control_center(control: &RenderControlSpec, origin: Point, size: Size) -> Point {
    Point::new(
        origin.x + size.width * control.anchor_x,
        origin.y + size.height * control.anchor_y,
    )
}

fn amp_spine_bounds(size: Size) -> Rectangle {
    let width = (size.width * 0.19).clamp(190.0, 292.0);
    Rectangle {
        x: size.width - width,
        y: 0.0,
        width,
        height: size.height,
    }
}

fn amp_stage_width(size: Size) -> f32 {
    (amp_spine_bounds(size).x - 30.0).max(360.0)
}

fn amp_spine_layout(app_profile: AppProfile, size: Size) -> Vec<(AmpModel, Rectangle)> {
    let bounds = amp_spine_bounds(size);
    let count = app_profile.amp_models.len().max(1) as f32;
    let row_height = bounds.height / count;

    app_profile
        .amp_models
        .iter()
        .enumerate()
        .map(|(index, descriptor)| {
            (
                descriptor.visual,
                Rectangle {
                    x: bounds.x,
                    y: bounds.y + index as f32 * row_height,
                    width: bounds.width,
                    height: row_height,
                },
            )
        })
        .collect()
}

fn hit_test_amp_spine(app_profile: AppProfile, size: Size, position: Point) -> Option<AmpModel> {
    amp_spine_layout(app_profile, size)
        .into_iter()
        .find(|(_, bounds)| {
            position.x >= bounds.x
                && position.x <= bounds.x + bounds.width
                && position.y >= bounds.y
                && position.y <= bounds.y + bounds.height
        })
        .map(|(model, _)| model)
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
    app_profile: AppProfile,
    devices: &[BoardDeviceSlot],
    size: Size,
    position: Point,
) -> Option<(usize, ControlKind)> {
    let layout = board_layout(devices.len(), size);
    let y = pedal_board_y(size, layout.pedal_h);

    devices
        .iter()
        .enumerate()
        .find_map(|(visible_index, slot)| {
            let origin = Point::new(
                layout.start_x + visible_index as f32 * (layout.pedal_w + layout.gap),
                y,
            );
            let size = Size::new(layout.pedal_w, layout.pedal_h);
            pedal_knob_centers(app_profile, &slot.device, origin, size)
                .into_iter()
                .find(|(_, center, hit_radius)| distance(*center, position) <= *hit_radius)
                .map(|(control, _, _)| (slot.source_index, control))
        })
}

fn hit_test_pedal_footswitch(
    app_profile: AppProfile,
    devices: &[BoardDeviceSlot],
    size: Size,
    position: Point,
) -> Option<usize> {
    let layout = board_layout(devices.len(), size);
    let y = pedal_board_y(size, layout.pedal_h);

    devices
        .iter()
        .enumerate()
        .find_map(|(visible_index, slot)| {
            let origin = Point::new(
                layout.start_x + visible_index as f32 * (layout.pedal_w + layout.gap),
                y,
            );
            let size = Size::new(layout.pedal_w, layout.pedal_h);
            device_render_spec(app_profile, slot.device.model)
                .controls
                .iter()
                .find(|control| {
                    control.role == RenderControlRole::Bypass
                        && control.widget == RenderControlWidget::Footswitch
                        && control.hit_radius > 0.0
                        && distance(render_control_center(control, origin, size), position)
                            <= control.hit_radius
                })
                .map(|_| slot.source_index)
        })
}

fn pedal_knob_centers(
    app_profile: AppProfile,
    device: &DeviceState,
    origin: Point,
    size: Size,
) -> Vec<(ControlKind, Point, f32)> {
    device_render_spec(app_profile, device.model)
        .controls
        .iter()
        .filter_map(|control| match control.role {
            RenderControlRole::Parameter(kind) => Some((
                kind,
                render_control_center(control, origin, size),
                control.hit_radius,
            )),
            RenderControlRole::Bypass => None,
        })
        .collect()
}

fn hit_test_amp_knob(
    app_profile: AppProfile,
    size: Size,
    model: AmpModel,
    position: Point,
) -> Option<ControlKind> {
    let render_spec = amp_render_spec(app_profile, model);
    if !render_spec.controls.is_empty() {
        let render_bounds = amp_render_bounds(size, render_spec);
        let origin = Point::new(render_bounds.x, render_bounds.y);
        let render_size = Size::new(render_bounds.width, render_bounds.height);
        return render_spec
            .controls
            .iter()
            .filter_map(|control| match control.role {
                RenderControlRole::Parameter(kind) => Some((kind, control)),
                RenderControlRole::Bypass => None,
            })
            .find(|(_, control)| {
                distance(
                    render_control_center(control, origin, render_size),
                    position,
                ) <= control.hit_radius
            })
            .map(|(kind, _)| kind);
    }

    let knobs = amp_knob_layout(size, model);

    knobs
        .into_iter()
        .find(|(_, center)| distance(*center, position) <= 48.0)
        .map(|(control, _)| control)
}

fn hit_test_amp_bypass(
    app_profile: AppProfile,
    size: Size,
    model: AmpModel,
    position: Point,
) -> bool {
    let render_spec = amp_render_spec(app_profile, model);
    if render_spec.controls.is_empty() {
        return false;
    }

    let render_bounds = amp_render_bounds(size, render_spec);
    let origin = Point::new(render_bounds.x, render_bounds.y);
    let render_size = Size::new(render_bounds.width, render_bounds.height);

    render_spec
        .controls
        .iter()
        .filter(|control| control.role == RenderControlRole::Bypass && control.hit_radius > 0.0)
        .any(|control| {
            distance(
                render_control_center(control, origin, render_size),
                position,
            ) <= control.hit_radius
        })
}

fn amp_knob_layout(size: Size, model: AmpModel) -> Vec<(ControlKind, Point)> {
    match model {
        AmpModel::Nox30 => {
            let amp_w = size.width.min(1080.0);
            let origin = Point::new((size.width - amp_w) * 0.5, 74.0);
            let first_knob_x = origin.x + amp_w * 0.38;
            let knob_y = origin.y + 96.0;
            let spacing = 120.0;
            vec![
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
                    ControlKind::Sag,
                    Point::new(first_knob_x + spacing * 3.0, knob_y),
                ),
                (
                    ControlKind::Treble,
                    Point::new(first_knob_x + spacing * 4.0, knob_y),
                ),
            ]
        }
        AmpModel::Daybreaker50 | AmpModel::WideCombo => {
            let amp_w = size.width.min(1210.0);
            let origin = Point::new((size.width - amp_w) * 0.5, 70.0);
            let panel_x = origin.x + 210.0;
            let first_knob_x = panel_x + 155.0;
            let knob_y = origin.y + 80.0;
            let spacing = 87.0;
            vec![
                (ControlKind::Gain, Point::new(first_knob_x, knob_y)),
                (
                    ControlKind::Treble,
                    Point::new(first_knob_x + spacing, knob_y),
                ),
                (
                    ControlKind::Bass,
                    Point::new(first_knob_x + spacing * 2.0, knob_y),
                ),
                (
                    ControlKind::Cut,
                    Point::new(first_knob_x + spacing * 3.0, knob_y),
                ),
                (
                    ControlKind::Presence,
                    Point::new(first_knob_x + spacing * 4.0, knob_y),
                ),
                (
                    ControlKind::Master,
                    Point::new(first_knob_x + spacing * 5.0, knob_y),
                ),
            ]
        }
        AmpModel::LeadHead => {
            let amp_w = size.width.min(1240.0);
            let origin = Point::new((size.width - amp_w) * 0.5, 62.0);
            let panel_x = origin.x + 135.0;
            let first_knob_x = panel_x + 165.0;
            let knob_y = origin.y + 310.0;
            let spacing = 102.0;
            vec![
                (ControlKind::Presence, Point::new(first_knob_x, knob_y)),
                (ControlKind::Sag, Point::new(first_knob_x + spacing, knob_y)),
                (
                    ControlKind::Gain,
                    Point::new(first_knob_x + spacing * 2.0, knob_y),
                ),
                (
                    ControlKind::Drive,
                    Point::new(first_knob_x + spacing * 3.0, knob_y),
                ),
                (
                    ControlKind::Treble,
                    Point::new(first_knob_x + spacing * 4.0, knob_y),
                ),
                (
                    ControlKind::Cut,
                    Point::new(first_knob_x + spacing * 5.0, knob_y),
                ),
                (
                    ControlKind::Bass,
                    Point::new(first_knob_x + spacing * 6.0, knob_y),
                ),
                (
                    ControlKind::Master,
                    Point::new(first_knob_x + spacing * 7.0, knob_y),
                ),
            ]
        }
        AmpModel::NamLoader => Vec::new(),
    }
}

fn hit_test_eq_toggle(position: Point) -> bool {
    distance(eq_power_switch_center(), position) <= 38.0
}

fn hit_test_eq_hpf(position: Point) -> bool {
    distance(eq_hpf_knob_center(), position) <= EQ_FILTER_KNOB_HIT_RADIUS
}

fn hit_test_eq_lpf(position: Point) -> bool {
    distance(eq_lpf_knob_center(), position) <= EQ_FILTER_KNOB_HIT_RADIUS
}

fn eq_slider_value_at(position: Point) -> Option<(usize, f32)> {
    (0..EQ_BAND_COUNT).find_map(|index| {
        let x = EQ_FIRST_SLIDER_X + index as f32 * EQ_SLIDER_SPACING;
        let inside_x = position.x >= x - 34.0 && position.x <= x + 34.0;
        let inside_y =
            position.y >= EQ_SLIDER_TOP - 30.0 && position.y <= EQ_SLIDER_TOP + EQ_SLIDER_H + 30.0;
        (inside_x && inside_y).then_some((index, eq_value_from_y(position.y)))
    })
}

fn eq_value_from_y(y: f32) -> f32 {
    ((EQ_SLIDER_TOP + EQ_SLIDER_H - y) / EQ_SLIDER_H).clamp(0.0, 1.0)
}

fn eq_dragged_filter_value(gesture: EqDragGesture, position: Point) -> f32 {
    let horizontal = position.x - gesture.start_position.x;
    let vertical = gesture.start_position.y - position.y;
    snap_eq_filter_value((gesture.start_value + (horizontal + vertical) / 260.0).clamp(0.0, 1.0))
}

fn eq_hpf_knob_center() -> Point {
    Point::new(EQ_PANEL_X + 97.0, EQ_PANEL_Y + 163.0)
}

fn eq_lpf_knob_center() -> Point {
    Point::new(EQ_PANEL_X + EQ_PANEL_W - 97.0, EQ_PANEL_Y + 163.0)
}

fn eq_power_led_center() -> Point {
    Point::new(EQ_PANEL_X + EQ_PANEL_W - 91.0, EQ_PANEL_Y + 282.0)
}

fn eq_power_switch_center() -> Point {
    Point::new(EQ_PANEL_X + 97.0, EQ_PANEL_Y + 279.0)
}

fn dragged_value(gesture: DragGesture, position: Point) -> f32 {
    let horizontal = position.x - gesture.start_position.x;
    let vertical = gesture.start_position.y - position.y;
    (gesture.start_value + (horizontal + vertical) / 240.0).clamp(0.0, 1.0)
}

fn control_message(control: ControlKind, value: f32) -> Message {
    match control {
        ControlKind::Gain => Message::GainChanged(value),
        ControlKind::Drive => Message::DriveChanged(value),
        ControlKind::Bass => Message::BassChanged(value),
        ControlKind::Treble => Message::TrebleChanged(value),
        ControlKind::Cut => Message::CutChanged(value),
        ControlKind::Presence => Message::PresenceChanged(value),
        ControlKind::Sag => Message::SagChanged(value),
        ControlKind::Master => Message::MasterChanged(value),
    }
}

fn metronome_control_message(control: MetronomeControl, value: f32) -> Message {
    match control {
        MetronomeControl::Volume => Message::MetronomeVolumeChanged(value),
        MetronomeControl::Pan => Message::MetronomePanChanged(value),
        MetronomeControl::MuteProbability => Message::MetronomeMuteProbabilityChanged(value),
    }
}

fn distance(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn draw_amp_spine(frame: &mut Frame, size: Size, app_profile: AppProfile, selected: AmpModel) {
    let spine = amp_spine_bounds(size);
    frame.fill(
        &Path::rectangle(Point::new(spine.x, spine.y), spine.size()),
        Color::from_rgb(0.035, 0.042, 0.052),
    );
    frame.stroke(
        &Path::line(
            Point::new(spine.x + 0.5, spine.y),
            Point::new(spine.x + 0.5, spine.y + spine.height),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))
            .with_width(1.0),
    );

    for (index, (model, row)) in amp_spine_layout(app_profile, size).into_iter().enumerate() {
        let active = model == selected;
        if active {
            frame.fill(
                &Path::rectangle(Point::new(row.x, row.y), row.size()),
                Color::from_rgba(GOLD.r, GOLD.g, GOLD.b, 0.055),
            );
            frame.fill_rectangle(Point::new(row.x, row.y), Size::new(3.0, row.height), GOLD);
        }

        if index > 0 {
            frame.stroke(
                &Path::line(
                    Point::new(row.x + 1.0, row.y + 0.5),
                    Point::new(row.x + row.width, row.y + 0.5),
                ),
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))
                    .with_width(1.0),
            );
        }

        let (name, character) = amp_spine_copy(model);
        let text_x = row.x + 42.0;
        let center_y = row.y + row.height * 0.5;
        let name_color = if active {
            GOLD
        } else {
            Color::from_rgb(0.58, 0.60, 0.62)
        };
        let character_color = if active {
            Color::from_rgba(GOLD.r, GOLD.g, GOLD.b, 0.88)
        } else {
            MUTED_INK
        };
        let name_size = amp_spine_name_size(name, row.height);

        draw_text(
            frame,
            name,
            Point::new(text_x, center_y - name_size * 0.22),
            name_size,
            name_color,
            Horizontal::Left,
        );
        draw_text(
            frame,
            character,
            Point::new(text_x, center_y + name_size * 0.72),
            12.0,
            character_color,
            Horizontal::Left,
        );

        let indicator = Path::circle(Point::new(row.x + row.width - 32.0, center_y), 8.5);
        frame.stroke(
            &indicator,
            Stroke::default()
                .with_color(if active {
                    GOLD
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.24)
                })
                .with_width(1.5),
        );
        if active {
            frame.fill(
                &Path::circle(Point::new(row.x + row.width - 32.0, center_y), 4.7),
                GOLD,
            );
        }
    }
}

fn amp_spine_copy(model: AmpModel) -> (&'static str, &'static str) {
    match model {
        AmpModel::Nox30 => ("NOX 30", "BRITISH"),
        AmpModel::Daybreaker50 => ("DAYBREAKER", "CLEAN 50"),
        AmpModel::WideCombo => ("STAR", "CLEAN"),
        AmpModel::LeadHead => ("SEVEN", "LEAD"),
        AmpModel::NamLoader => ("NAM", "CAPTURE"),
    }
}

/// Keep the amp picker legible without letting longer model names collide with
/// the selection indicator. Eight characters use the standard type scale;
/// longer names switch to the compact picker type scale.
fn amp_spine_name_size(name: &str, row_height: f32) -> f32 {
    let base_size = (row_height * 0.20).clamp(25.0, 43.0);
    if name.chars().count() > 8 {
        base_size * 0.60
    } else {
        base_size
    }
}

#[derive(Debug, Clone)]
struct RecordArt {
    active: bool,
    sample_rate: u32,
    output_left: f32,
    output_right: f32,
    scale: f32,
}

impl canvas::Program<Message> for RecordArt {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let Some(position) = cursor
                .position_in(bounds)
                .map(|position| unscale_point(position, self.scale))
            else {
                return (canvas::event::Status::Ignored, None);
            };
            let logical_size = unscale_size(bounds.size(), self.scale);
            let center = record_button_center(logical_size);
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            if dx * dx + dy * dy <= record_button_radius(logical_size).powi(2) * 1.42 {
                return (
                    canvas::event::Status::Captured,
                    Some(Message::ToggleRecording),
                );
            }
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
        let layout = record_layout(logical_size);
        let center = record_button_center(logical_size);
        let radius = record_button_radius(logical_size);
        let transport_label = if self.active {
            "WRITE · LIVE".to_string()
        } else {
            format!("READY · {} Hz", self.sample_rate)
        };

        if self.active {
            frame.stroke(
                &Path::circle(center, radius * 1.18),
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 0.16, 0.12, 0.82))
                    .with_width(2.2),
            );
            frame.fill(
                &Path::circle(Point::new(center.x, center.y + radius * 1.48), 3.2),
                Color::from_rgb(0.98, 0.18, 0.14),
            );
        }

        draw_text(
            &mut frame,
            "REC",
            center,
            radius * 0.24,
            Color::from_rgba(1.0, 0.93, 0.90, 0.94),
            Horizontal::Center,
        );
        draw_text(
            &mut frame,
            transport_label.as_str(),
            Point::new(
                layout.display.x + layout.display.width * 0.5,
                layout.display.y + layout.display.height * 0.56,
            ),
            (layout.display.height * 0.20).clamp(12.0, 20.0),
            if self.active {
                Color::from_rgb(1.0, 0.30, 0.20)
            } else {
                Color::from_rgba(0.95, 0.65, 0.34, 0.80)
            },
            Horizontal::Center,
        );

        draw_record_vu_meter(&mut frame, layout.left_meter, self.output_left, "L");
        draw_record_vu_meter(&mut frame, layout.right_meter, self.output_right, "R");
        draw_record_waveform(
            &mut frame,
            layout.waveform,
            self.output_left,
            self.output_right,
            self.active,
        );
        vec![frame.into_geometry()]
    }
}

fn draw_record_vu_meter(frame: &mut Frame, meter: Rectangle, level: f32, channel: &str) {
    let pivot = Point::new(meter.x + meter.width * 0.5, meter.y + meter.height * 0.83);
    let deflection = level.clamp(0.0, 1.0).powf(0.38);
    let angle = -2.62 + deflection * 2.18;
    let length = meter.width * 0.43;
    let tip = Point::new(
        pivot.x + angle.cos() * length,
        pivot.y + angle.sin() * length,
    );
    frame.stroke(
        &Path::line(pivot, tip),
        Stroke::default()
            .with_color(Color::from_rgba(0.18, 0.025, 0.018, 0.92))
            .with_width(4.4),
    );
    frame.stroke(
        &Path::line(pivot, tip),
        Stroke::default()
            .with_color(Color::from_rgb(0.95, 0.16, 0.10))
            .with_width(1.75),
    );
    frame.stroke(
        &Path::line(
            pivot,
            Point::new(
                pivot.x - angle.cos() * meter.width * 0.075,
                pivot.y - angle.sin() * meter.width * 0.075,
            ),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.66, 0.08, 0.05, 0.86))
            .with_width(2.0),
    );
    frame.fill(
        &Path::circle(pivot, meter.height * 0.025),
        Color::from_rgb(0.14, 0.045, 0.028),
    );
    frame.stroke(
        &Path::circle(pivot, meter.height * 0.025),
        Stroke::default()
            .with_color(Color::from_rgba(0.94, 0.69, 0.38, 0.72))
            .with_width(0.8),
    );
    draw_text(
        frame,
        channel,
        Point::new(meter.x + meter.width * 0.5, meter.y + meter.height * 0.92),
        (meter.height * 0.085).clamp(9.0, 14.0),
        Color::from_rgba(0.17, 0.10, 0.05, 0.72),
        Horizontal::Center,
    );
}

fn draw_record_waveform(frame: &mut Frame, bounds: Rectangle, left: f32, right: f32, active: bool) {
    let panel = rounded_rect(Point::new(bounds.x, bounds.y), bounds.size(), 5.0);
    frame.fill(&panel, Color::from_rgba(0.005, 0.008, 0.010, 0.75));
    frame.stroke(
        &panel,
        Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.58, 0.30, 0.30))
            .with_width(1.0),
    );

    let center_y = bounds.y + bounds.height * 0.5;
    let amplitude = (left.max(right) * bounds.height * 0.34).max(2.0);
    let trace = Path::new(|path| {
        for index in 0..96 {
            let progress = index as f32 / 95.0;
            let x = bounds.x + bounds.width * progress;
            let carrier = (progress * 68.0).sin();
            let envelope = 0.34 + (progress * 9.0).sin().abs() * 0.66;
            let y = center_y + carrier * amplitude * envelope;
            if index == 0 {
                path.move_to(Point::new(x, y));
            } else {
                path.line_to(Point::new(x, y));
            }
        }
    });
    frame.stroke(
        &trace,
        Stroke::default()
            .with_color(if active {
                Color::from_rgb(0.96, 0.24, 0.17)
            } else {
                Color::from_rgba(GOLD.r, GOLD.g, GOLD.b, 0.74)
            })
            .with_width(1.2),
    );
}

#[derive(Debug, Clone)]
struct RecordCanvas {
    width: Length,
    height: Length,
    art: RecordArt,
}

impl RecordCanvas {
    fn new(art: RecordArt) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for RecordCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<()>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };
        let Some(canvas_event) = canvas_event else {
            return iced::event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<()>();
        let (status, message) = self
            .art
            .update(state, canvas_event, layout.bounds(), cursor);
        if let Some(message) = message {
            shell.publish(message);
        }
        status
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::Pointer
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<()>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let background = draw_record_background(renderer, &self.art, bounds.size());
            renderer.draw(vec![background]);
            draw_amp_studio_backdrop(renderer, bounds.size());
            draw_record_assets(renderer, &self.art, bounds.size());
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
        });
    }
}

impl<'a> From<RecordCanvas> for Element<'a, Message> {
    fn from(record: RecordCanvas) -> Self {
        Element::new(record)
    }
}

fn draw_stage_circuit_toggle(frame: &mut Frame, size: Size, selected: bool) {
    let center = stage_circuit_toggle_center(size);
    let ink = Color::from_rgba(INK.r, INK.g, INK.b, if selected { 1.0 } else { 0.52 });

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
    meter_level: f32,
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
        let center = Point::new(logical_size.width * 0.5, 44.0);
        draw_text(
            &mut frame,
            self.label,
            Point::new(logical_size.width * 0.5, 7.0),
            9.0,
            MUTED_INK,
            Horizontal::Center,
        );
        if matches!(self.control, GlobalControl::Input | GlobalControl::Output) {
            frame.fill(
                &Path::circle(center, 25.0),
                Color::from_rgba(TEAL.r, TEAL.g, TEAL.b, 0.06 + self.meter_level * 0.16),
            );
        }
        draw_text(
            &mut frame,
            &self.readout,
            Point::new(logical_size.width * 0.5, 75.0),
            9.0,
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

#[derive(Debug, Clone)]
struct GlobalKnobCanvas {
    width: Length,
    height: Length,
    art: GlobalKnobArt,
}

impl GlobalKnobCanvas {
    fn new(art: GlobalKnobArt) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for GlobalKnobCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<DragState>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DragState::default())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        let Some(canvas_event) = canvas_event else {
            return iced::event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<DragState>();
        let (status, message) = self
            .art
            .update(state, canvas_event, layout.bounds(), cursor);
        if let Some(message) = message {
            shell.publish(message);
        }
        status
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<DragState>();
        self.art.mouse_interaction(state, layout.bounds(), cursor)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<DragState>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
            draw_global_knob_asset(renderer, &self.art, bounds.size());
        });
    }
}

impl<'a> From<GlobalKnobCanvas> for Element<'a, Message> {
    fn from(control: GlobalKnobCanvas) -> Self {
        Element::new(control)
    }
}

#[derive(Debug, Clone)]
struct DoublerArt {
    enabled: bool,
    value: f32,
    readout: String,
    scale: f32,
}

impl canvas::Program<Message> for DoublerArt {
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
                if position.x >= 58.0
                    && position.x <= 86.0
                    && position.y >= 2.0
                    && position.y <= 20.0
                {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::ToggleDoubler),
                    );
                }
                state.gesture = Some(DragGesture {
                    index: None,
                    control: ControlKind::Master,
                    start_position: position,
                    start_value: self.value,
                });
                (
                    canvas::event::Status::Captured,
                    Some(Message::SetGlobalControl {
                        control: GlobalControl::DoublerDelay,
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
                        control: GlobalControl::DoublerDelay,
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
        draw_text(
            &mut frame,
            "DOUBLER",
            Point::new(logical_size.width * 0.5 - 7.0, 7.0),
            9.0,
            MUTED_INK,
            Horizontal::Center,
        );

        let switch_origin = Point::new(logical_size.width - 24.0, 2.0);
        let switch = rounded_rect(switch_origin, Size::new(22.0, 12.0), 6.0);
        let switch_color = if self.enabled {
            GOLD
        } else {
            Color::from_rgba(0.56, 0.60, 0.64, 0.50)
        };
        frame.fill(&switch, switch_color);
        let thumb_x = if self.enabled {
            logical_size.width - 7.0
        } else {
            logical_size.width - 18.0
        };
        frame.fill(&Path::circle(Point::new(thumb_x, 8.0), 3.6), Color::WHITE);

        draw_text(
            &mut frame,
            &self.readout,
            Point::new(logical_size.width * 0.5, 70.0),
            9.0,
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

#[derive(Debug, Clone)]
struct DoublerCanvas {
    width: Length,
    height: Length,
    art: DoublerArt,
}

impl DoublerCanvas {
    fn new(art: DoublerArt) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            art,
        }
    }

    fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl Widget<Message, iced::Renderer> for DoublerCanvas {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<DragState>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DragState::default())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let canvas_event = match event {
            Event::Mouse(event) => Some(canvas::Event::Mouse(event)),
            Event::Touch(event) => Some(canvas::Event::Touch(event)),
            Event::Keyboard(event) => Some(canvas::Event::Keyboard(event)),
            _ => None,
        };

        let Some(canvas_event) = canvas_event else {
            return iced::event::Status::Ignored;
        };

        let state = tree.state.downcast_mut::<DragState>();
        let (status, message) = self
            .art
            .update(state, canvas_event, layout.bounds(), cursor);
        if let Some(message) = message {
            shell.publish(message);
        }
        status
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<DragState>();
        self.art.mouse_interaction(state, layout.bounds(), cursor)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<DragState>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw(self.art.draw(state, renderer, theme, bounds, cursor));
            draw_doubler_knob_asset(renderer, &self.art, bounds.size());
        });
    }
}

impl<'a> From<DoublerCanvas> for Element<'a, Message> {
    fn from(control: DoublerCanvas) -> Self {
        Element::new(control)
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

fn draw_stage_background(frame: &mut Frame, size: Size) {
    let gradient = canvas_gradient::Linear::new(Point::ORIGIN, Point::new(0.0, size.height))
        .add_stop(0.0, Color::from_rgb(0.090, 0.108, 0.130))
        .add_stop(0.56, Color::from_rgb(0.067, 0.080, 0.096))
        .add_stop(1.0, Color::from_rgb(0.045, 0.055, 0.068));
    frame.fill(&Path::rectangle(Point::ORIGIN, size), gradient);

    frame.stroke(
        &Path::line(Point::new(0.0, 0.5), Point::new(size.width, 0.5)),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.045))
            .with_width(1.0),
    );
}

fn draw_amp_head(frame: &mut Frame, size: Size, amp: &DeviceState) {
    let stage_size = Size::new(amp_stage_width(size), size.height);
    if amp.model == DeviceModel::WideCombo {
        draw_wide_combo_amp_head(frame, stage_size, amp);
        return;
    }
    if amp.model == DeviceModel::LeadHead {
        draw_boxer_seven_amp_head(frame, stage_size, amp);
        return;
    }

    let amp_w = stage_size.width.min(1080.0);
    let amp_h = 390.0;
    let origin = Point::new((stage_size.width - amp_w) * 0.5, 74.0);

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
        amp.sag,
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
        &amp.name,
        Point::new(origin.x + amp_w - 112.0, origin.y + amp_h - 52.0),
        32.0,
        Color::from_rgb(0.04, 0.025, 0.03),
        Horizontal::Center,
    );
}

fn draw_boxer_seven_amp_head(frame: &mut Frame, size: Size, amp: &DeviceState) {
    let amp_w = size.width.min(1240.0);
    let amp_h = 420.0;
    let origin = Point::new((size.width - amp_w) * 0.5, 62.0);

    let shadow = rounded_rect(
        Point::new(origin.x + 16.0, origin.y + 24.0),
        Size::new(amp_w, amp_h),
        20.0,
    );
    frame.fill(&shadow, Color::from_rgba(0.035, 0.032, 0.030, 0.30));

    let handle = Path::new(|p| {
        p.move_to(Point::new(origin.x + amp_w * 0.39, origin.y - 16.0));
        p.quadratic_curve_to(
            Point::new(origin.x + amp_w * 0.50, origin.y - 48.0),
            Point::new(origin.x + amp_w * 0.61, origin.y - 16.0),
        );
    });
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgb(0.025, 0.023, 0.022))
            .with_width(10.0),
    );
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.82, 0.72, 0.52))
            .with_width(1.2),
    );
    for x in [origin.x + amp_w * 0.40, origin.x + amp_w * 0.60] {
        frame.fill(
            &Path::circle(Point::new(x, origin.y - 16.0), 9.0),
            Color::from_rgb(0.06, 0.055, 0.052),
        );
        frame.fill(
            &Path::circle(Point::new(x, origin.y - 16.0), 4.0),
            Color::from_rgb(0.78, 0.71, 0.58),
        );
    }

    let body = rounded_rect(origin, Size::new(amp_w, amp_h), 18.0);
    let cream = Color::from_rgb(0.79, 0.76, 0.68);
    frame.fill(&body, cream);
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.93, 0.86, 0.86))
            .with_width(4.0),
    );
    frame.stroke(
        &rounded_rect(
            Point::new(origin.x + 10.0, origin.y + 10.0),
            Size::new(amp_w - 20.0, amp_h - 20.0),
            11.0,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.28, 0.24, 0.18, 0.18))
            .with_width(1.4),
    );

    for index in 0..70 {
        let x = origin.x + 24.0 + (index as f32 * 37.0) % (amp_w - 48.0);
        let y = origin.y + 18.0 + (index as f32 * 19.0) % (amp_h - 36.0);
        frame.fill(
            &Path::circle(Point::new(x, y), 0.9),
            Color::from_rgba(0.30, 0.27, 0.22, 0.14),
        );
    }

    let grille_origin = Point::new(origin.x + 44.0, origin.y + 46.0);
    let grille_size = Size::new(amp_w - 88.0, 158.0);
    let grille = rounded_rect(grille_origin, grille_size, 4.0);
    frame.fill(&grille, Color::from_rgb(0.39, 0.36, 0.29));
    frame.stroke(
        &grille,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.78, 0.44, 0.62))
            .with_width(2.6),
    );
    draw_boxer_grille(frame, grille_origin, grille_size);

    draw_text(
        frame,
        "Boxer",
        Point::new(origin.x + amp_w * 0.5, origin.y + 122.0),
        62.0,
        Color::from_rgb(0.88, 0.82, 0.62),
        Horizontal::Center,
    );
    draw_text(
        frame,
        "SEVEN LEAD",
        Point::new(origin.x + amp_w * 0.5, origin.y + 160.0),
        15.0,
        Color::from_rgba(0.91, 0.85, 0.66, 0.82),
        Horizontal::Center,
    );

    let lower = rounded_rect(
        Point::new(origin.x + 40.0, origin.y + 207.0),
        Size::new(amp_w - 80.0, 152.0),
        4.0,
    );
    frame.fill(&lower, Color::from_rgb(0.76, 0.73, 0.65));
    frame.stroke(
        &lower,
        Stroke::default()
            .with_color(Color::from_rgba(0.95, 0.91, 0.76, 0.36))
            .with_width(1.6),
    );

    let panel_x = origin.x + 135.0;
    let panel_y = origin.y + 272.0;
    let panel_w = amp_w - 270.0;
    let panel_h = 74.0;
    let panel = rounded_rect(
        Point::new(panel_x, panel_y),
        Size::new(panel_w, panel_h),
        10.0,
    );
    frame.fill(&panel, Color::from_rgb(0.70, 0.52, 0.25));
    frame.fill_rectangle(
        Point::new(panel_x + 14.0, panel_y + 6.0),
        Size::new(panel_w - 28.0, 5.0),
        Color::from_rgba(1.0, 0.88, 0.54, 0.20),
    );
    frame.stroke(
        &panel,
        Stroke::default()
            .with_color(Color::from_rgba(0.14, 0.09, 0.04, 0.42))
            .with_width(1.6),
    );

    draw_text(
        frame,
        "INPUT",
        Point::new(panel_x + 48.0, panel_y + 18.0),
        9.0,
        Color::from_rgb(0.12, 0.085, 0.045),
        Horizontal::Center,
    );
    draw_jack(frame, Point::new(panel_x + 48.0, panel_y + 42.0), 11.5);
    draw_amp_status_led(
        frame,
        Point::new(panel_x + 92.0, panel_y + 42.0),
        !amp.bypassed,
    );

    let knob_y = panel_y + 38.0;
    let first_knob_x = panel_x + 165.0;
    let spacing = 102.0;
    let knob_radius = 17.5;
    for (offset, label, value) in [
        (0.0, "Presence", amp.presence),
        (1.0, "Excursion", amp.sag),
        (2.0, "Gain", amp.gain),
        (3.0, "Drive", amp.drive),
        (4.0, "Treble", amp.treble),
        (5.0, "Middle", amp.cut),
        (6.0, "Bass", amp.bass),
        (7.0, "Output", amp.master),
    ] {
        draw_component_knob(
            frame,
            Point::new(first_knob_x + spacing * offset, knob_y),
            knob_radius,
            label,
            value,
            KnobSkin::AsatoBlack,
        );
    }

    let power_x = panel_x + panel_w - 36.0;
    draw_text(
        frame,
        "POWER",
        Point::new(power_x, panel_y + 18.0),
        9.0,
        Color::from_rgb(0.12, 0.085, 0.045),
        Horizontal::Center,
    );
    draw_jack(frame, Point::new(power_x, panel_y + 42.0), 12.0);
    frame.stroke(
        &Path::line(
            Point::new(power_x - 2.0, panel_y + 30.0),
            Point::new(power_x + 4.0, panel_y + 52.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgb(0.035, 0.030, 0.025))
            .with_width(3.8),
    );

    for &(x, y, w, h) in &[
        (origin.x + 10.0, origin.y + 16.0, 38.0, 190.0),
        (origin.x + amp_w - 48.0, origin.y + 16.0, 38.0, 190.0),
        (origin.x + 10.0, origin.y + amp_h - 70.0, 38.0, 48.0),
        (origin.x + amp_w - 48.0, origin.y + amp_h - 70.0, 38.0, 48.0),
    ] {
        let protector = rounded_rect(Point::new(x, y), Size::new(w, h), 10.0);
        frame.fill(&protector, Color::from_rgba(0.62, 0.59, 0.52, 0.58));
        frame.stroke(
            &protector,
            Stroke::default()
                .with_color(Color::from_rgba(0.96, 0.92, 0.80, 0.28))
                .with_width(1.0),
        );
    }

    for foot_x in [origin.x + 96.0, origin.x + amp_w - 96.0] {
        let foot = rounded_rect(
            Point::new(foot_x - 34.0, origin.y + amp_h - 8.0),
            Size::new(68.0, 22.0),
            8.0,
        );
        frame.fill(&foot, Color::from_rgb(0.035, 0.032, 0.031));
    }
}

fn draw_boxer_grille(frame: &mut Frame, origin: Point, size: Size) {
    let dark = Color::from_rgba(0.08, 0.07, 0.055, 0.62);
    let light = Color::from_rgba(0.86, 0.78, 0.58, 0.46);
    let cell = 10.0;
    let cols = (size.width / cell).ceil() as usize;
    let rows = (size.height / cell).ceil() as usize;
    for row in 0..rows {
        let y = origin.y + row as f32 * cell;
        frame.stroke(
            &Path::line(
                Point::new(origin.x + 4.0, y),
                Point::new(origin.x + size.width - 4.0, y),
            ),
            Stroke::default()
                .with_color(if row % 2 == 0 { dark } else { light })
                .with_width(0.8),
        );
    }
    for col in 0..cols {
        let x = origin.x + col as f32 * cell;
        frame.stroke(
            &Path::line(
                Point::new(x, origin.y + 4.0),
                Point::new(x, origin.y + size.height - 4.0),
            ),
            Stroke::default()
                .with_color(if col % 2 == 0 { light } else { dark })
                .with_width(0.8),
        );
    }
    for diag in 0..(cols + rows) {
        let x0 = origin.x + (diag as f32 - rows as f32) * cell;
        let y0 = origin.y + size.height;
        let x1 = x0 + size.height;
        let y1 = origin.y;
        frame.stroke(
            &Path::line(Point::new(x0, y0), Point::new(x1, y1)),
            Stroke::default()
                .with_color(Color::from_rgba(0.02, 0.018, 0.014, 0.18))
                .with_width(0.7),
        );
    }
}

fn draw_jack(frame: &mut Frame, center: Point, radius: f32) {
    frame.fill(
        &Path::circle(center, radius),
        Color::from_rgb(0.78, 0.69, 0.50),
    );
    frame.stroke(
        &Path::circle(center, radius),
        Stroke::default()
            .with_color(Color::from_rgba(0.12, 0.075, 0.03, 0.72))
            .with_width(1.4),
    );
    frame.fill(
        &Path::circle(center, radius * 0.55),
        Color::from_rgb(0.036, 0.032, 0.034),
    );
    frame.fill(
        &Path::circle(
            Point::new(center.x - radius * 0.20, center.y - radius * 0.24),
            radius * 0.16,
        ),
        Color::from_rgba(1.0, 0.94, 0.76, 0.42),
    );
}

fn draw_wide_combo_amp_head(frame: &mut Frame, size: Size, amp: &DeviceState) {
    let amp_w = size.width.min(1210.0);
    let amp_h = 386.0;
    let origin = Point::new((size.width - amp_w) * 0.5, 70.0);

    let shadow = rounded_rect(
        Point::new(origin.x + 14.0, origin.y + 24.0),
        Size::new(amp_w, amp_h),
        18.0,
    );
    frame.fill(&shadow, Color::from_rgba(0.025, 0.030, 0.050, 0.34));

    let handle = Path::new(|p| {
        p.move_to(Point::new(origin.x + amp_w * 0.43, origin.y - 18.0));
        p.quadratic_curve_to(
            Point::new(origin.x + amp_w * 0.50, origin.y - 42.0),
            Point::new(origin.x + amp_w * 0.57, origin.y - 18.0),
        );
    });
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgb(0.025, 0.022, 0.026))
            .with_width(9.0),
    );
    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(Color::from_rgba(0.86, 0.74, 0.53, 0.55))
            .with_width(1.2),
    );

    let body = rounded_rect(origin, Size::new(amp_w, amp_h), 18.0);
    let tolex = Color::from_rgb(0.080, 0.115, 0.145);
    frame.fill(&body, tolex);
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(Color::from_rgba(0.02, 0.025, 0.035, 0.64))
            .with_width(3.0),
    );
    frame.stroke(
        &rounded_rect(
            Point::new(origin.x + 8.0, origin.y + 8.0),
            Size::new(amp_w - 16.0, amp_h - 16.0),
            12.0,
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.78, 0.83, 0.87, 0.16))
            .with_width(1.2),
    );

    let top_panel = rounded_rect(
        Point::new(origin.x + 20.0, origin.y + 18.0),
        Size::new(amp_w - 40.0, 112.0),
        10.0,
    );
    frame.fill(&top_panel, Color::from_rgba(0.05, 0.07, 0.09, 0.26));

    let panel_x = origin.x + 210.0;
    let panel_y = origin.y + 36.0;
    let panel_w = amp_w - 300.0;
    let panel_h = 96.0;
    let panel = rounded_rect(
        Point::new(panel_x, panel_y),
        Size::new(panel_w, panel_h),
        12.0,
    );
    frame.fill(&panel, Color::from_rgb(0.84, 0.84, 0.79));
    frame.fill_rectangle(
        Point::new(panel_x + 12.0, panel_y + 6.0),
        Size::new(panel_w - 24.0, 5.0),
        Color::from_rgba(1.0, 1.0, 1.0, 0.18),
    );
    frame.stroke(
        &panel,
        Stroke::default()
            .with_color(Color::from_rgba(0.10, 0.10, 0.10, 0.34))
            .with_width(1.3),
    );

    draw_text(
        frame,
        "INPUT",
        Point::new(panel_x + 45.0, panel_y + 20.0),
        10.0,
        Color::from_rgb(0.10, 0.11, 0.12),
        Horizontal::Center,
    );
    frame.fill(
        &Path::circle(Point::new(panel_x + 45.0, panel_y + 50.0), 12.0),
        Color::from_rgb(0.74, 0.68, 0.55),
    );
    frame.fill(
        &Path::circle(Point::new(panel_x + 45.0, panel_y + 50.0), 6.5),
        Color::from_rgb(0.035, 0.035, 0.040),
    );

    let knob_y = panel_y + 44.0;
    let first_knob_x = panel_x + 155.0;
    let spacing = 87.0;
    draw_component_knob(
        frame,
        Point::new(first_knob_x, knob_y),
        17.0,
        "Gain",
        amp.gain,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing, knob_y),
        17.0,
        "Treble",
        amp.treble,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 2.0, knob_y),
        17.0,
        "Bass",
        amp.bass,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 3.0, knob_y),
        17.0,
        "Mid",
        amp.cut,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 4.0, knob_y),
        17.0,
        "Presence",
        amp.presence,
        KnobSkin::AsatoBlack,
    );
    draw_component_knob(
        frame,
        Point::new(first_knob_x + spacing * 5.0, knob_y),
        17.0,
        "Volume",
        amp.master,
        KnobSkin::AsatoBlack,
    );

    let power_x = panel_x + panel_w - 86.0;
    let logo_x = power_x - 176.0;
    let led_x = power_x + 42.0;

    let logo_plate = rounded_rect(
        Point::new(logo_x, panel_y + 30.0),
        Size::new(92.0, 24.0),
        4.0,
    );
    frame.fill(&logo_plate, Color::from_rgb(0.92, 0.91, 0.84));
    frame.stroke(
        &logo_plate,
        Stroke::default()
            .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.42))
            .with_width(1.2),
    );
    draw_text(
        frame,
        "WIDE COMBO",
        Point::new(logo_x + 46.0, panel_y + 42.0),
        11.0,
        Color::from_rgb(0.06, 0.06, 0.065),
        Horizontal::Center,
    );

    draw_text(
        frame,
        "POWER",
        Point::new(power_x, panel_y + 20.0),
        10.0,
        Color::from_rgb(0.10, 0.11, 0.12),
        Horizontal::Center,
    );
    frame.fill(
        &Path::circle(Point::new(power_x, panel_y + 50.0), 12.0),
        GOLD,
    );
    frame.stroke(
        &Path::line(
            Point::new(power_x, panel_y + 38.0),
            Point::new(power_x + 3.0, panel_y + 61.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgb(0.05, 0.045, 0.05))
            .with_width(4.0),
    );
    draw_amp_status_led(frame, Point::new(led_x, panel_y + 50.0), !amp.bypassed);

    let badge = rounded_rect(
        Point::new(origin.x + amp_w * 0.5 - 72.0, origin.y + 136.0),
        Size::new(144.0, 38.0),
        7.0,
    );
    frame.fill(&badge, Color::from_rgb(0.08, 0.09, 0.10));
    frame.stroke(
        &badge,
        Stroke::default()
            .with_color(Color::from_rgba(0.94, 0.88, 0.72, 0.72))
            .with_width(1.6),
    );
    draw_text(
        frame,
        "GREYBOUND",
        Point::new(origin.x + amp_w * 0.5, origin.y + 157.0),
        15.0,
        Color::from_rgb(0.92, 0.89, 0.78),
        Horizontal::Center,
    );
    draw_text(
        frame,
        "ENGINEERING",
        Point::new(origin.x + amp_w * 0.5, origin.y + 170.0),
        6.5,
        Color::from_rgba(0.92, 0.89, 0.78, 0.76),
        Horizontal::Center,
    );

    let grille_origin = Point::new(origin.x + 30.0, origin.y + 188.0);
    let grille_size = Size::new(amp_w - 60.0, 132.0);
    let grille = rounded_rect(grille_origin, grille_size, 2.0);
    frame.fill(&grille, Color::from_rgb(0.68, 0.67, 0.60));
    frame.stroke(
        &grille,
        Stroke::default()
            .with_color(Color::from_rgba(0.02, 0.02, 0.02, 0.76))
            .with_width(2.0),
    );
    draw_wide_combo_grille(frame, grille_origin, grille_size);

    for &(x, y) in &[
        (origin.x + 10.0, origin.y + 10.0),
        (origin.x + amp_w - 42.0, origin.y + 10.0),
        (origin.x + 10.0, origin.y + amp_h - 42.0),
        (origin.x + amp_w - 42.0, origin.y + amp_h - 42.0),
    ] {
        let guard = rounded_rect(Point::new(x, y), Size::new(32.0, 32.0), 8.0);
        frame.fill(&guard, Color::from_rgba(0.02, 0.02, 0.025, 0.72));
        frame.stroke(
            &guard,
            Stroke::default()
                .with_color(Color::from_rgba(0.82, 0.78, 0.66, 0.20))
                .with_width(1.0),
        );
    }
}

fn draw_daybreaker_amp_overlay(frame: &mut Frame, size: Size) {
    let render_bounds = amp_render_bounds(size, &DAYBREAKER_AMP_RENDER_SPEC);
    let origin = Point::new(render_bounds.x, render_bounds.y);
    let render_size = Size::new(render_bounds.width, render_bounds.height);
    let label_color = Color::from_rgb(0.16, 0.13, 0.09);

    let jack_center = Point::new(
        origin.x + render_size.width * 0.265,
        origin.y + render_size.height * 0.225,
    );
    draw_daybreaker_label(
        frame,
        "INPUT",
        Point::new(jack_center.x, jack_center.y + 30.0),
        10.5,
        label_color,
        Horizontal::Center,
    );
    for control in DAYBREAKER_AMP_CONTROLS {
        let RenderControlRole::Parameter(_) = control.role else {
            continue;
        };
        let center = render_control_center(control, origin, render_size);
        draw_daybreaker_label(
            frame,
            control.label,
            Point::new(center.x, center.y + 30.0),
            10.5,
            label_color,
            Horizontal::Center,
        );
    }

    let led = DAYBREAKER_AMP_CONTROLS
        .iter()
        .find(|control| control.widget == RenderControlWidget::Led)
        .expect("Daybreaker must provide a power LED");
    let toggle = DAYBREAKER_AMP_CONTROLS
        .iter()
        .find(|control| control.widget == RenderControlWidget::Toggle)
        .expect("Daybreaker must provide a power switch");
    let led_center = render_control_center(led, origin, render_size);
    let switch_center = render_control_center(toggle, origin, render_size);
    draw_daybreaker_label(
        frame,
        "POWER",
        Point::new((led_center.x + switch_center.x) * 0.5, led_center.y + 30.0),
        10.5,
        label_color,
        Horizontal::Center,
    );
}

fn draw_daybreaker_label(
    frame: &mut Frame,
    content: &str,
    position: Point,
    size: f32,
    color: Color,
    align: Horizontal,
) {
    draw_text(frame, content, position, size, color, align);
    draw_text(
        frame,
        content,
        Point::new(position.x + 0.55, position.y),
        size,
        color,
        align,
    );
}

fn nam_loader_rack_bounds(size: Size) -> Rectangle {
    amp_render_bounds(size, &NAM_LOADER_AMP_RENDER_SPEC)
}

fn nam_loader_button_bounds(size: Size) -> Rectangle {
    let rack = nam_loader_rack_bounds(size);
    Rectangle {
        x: rack.x + rack.width * 0.285,
        y: rack.y + rack.height * 0.650,
        width: rack.width * 0.220,
        height: rack.height * 0.105,
    }
}

fn hit_test_nam_loader_button(size: Size, position: Point) -> bool {
    let button = nam_loader_button_bounds(size);
    position.x >= button.x
        && position.x <= button.x + button.width
        && position.y >= button.y
        && position.y <= button.y + button.height
}

fn draw_nam_loader_overlay(frame: &mut Frame, size: Size, state: &NamLoaderState) {
    let rack = nam_loader_rack_bounds(size);
    let display_center = Point::new(rack.x + rack.width * 0.500, rack.y + rack.height * 0.390);
    let model_name = state
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(|name| truncate_nam_label(name, 42))
        .unwrap_or_else(|| "NO MODEL LOADED".to_string());
    let model_color = if state.path.is_some() {
        Color::from_rgb(0.71, 0.93, 0.86)
    } else {
        Color::from_rgb(0.78, 0.62, 0.44)
    };

    draw_text(
        frame,
        "NAM LOADER",
        Point::new(display_center.x, display_center.y - 34.0),
        13.0,
        Color::from_rgb(0.63, 0.79, 0.87),
        Horizontal::Center,
    );
    draw_text(
        frame,
        &model_name,
        Point::new(display_center.x, display_center.y - 5.0),
        18.0,
        model_color,
        Horizontal::Center,
    );
    draw_text(
        frame,
        "A2 MODEL · EXTERNAL PRE GAIN / POST VOLUME",
        Point::new(display_center.x, display_center.y + 25.0),
        10.0,
        Color::from_rgb(0.49, 0.62, 0.68),
        Horizontal::Center,
    );

    let button = nam_loader_button_bounds(size);
    let button_shape = rounded_rect(Point::new(button.x, button.y), button.size(), 6.0);
    frame.fill(&button_shape, Color::from_rgb(0.08, 0.30, 0.38));
    frame.stroke(
        &button_shape,
        Stroke::default()
            .with_color(Color::from_rgb(0.41, 0.86, 0.92))
            .with_width(1.5),
    );
    draw_text(
        frame,
        "LOAD .NAM MODEL",
        Point::new(
            button.x + button.width * 0.5,
            button.y + button.height * 0.5,
        ),
        13.0,
        Color::from_rgb(0.90, 0.98, 1.0),
        Horizontal::Center,
    );
    for control in NAM_LOADER_AMP_CONTROLS {
        let center = render_control_center(control, Point::new(rack.x, rack.y), rack.size());
        draw_daybreaker_label(
            frame,
            control.label,
            Point::new(center.x, center.y + 30.0),
            10.5,
            Color::from_rgb(0.72, 0.80, 0.83),
            Horizontal::Center,
        );
    }
    draw_text(
        frame,
        state.status.as_str(),
        Point::new(rack.x + rack.width * 0.500, rack.y + rack.height * 0.810),
        10.0,
        Color::from_rgb(0.55, 0.62, 0.67),
        Horizontal::Center,
    );
}

fn truncate_nam_label(label: &str, max_chars: usize) -> String {
    let mut chars = label.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn draw_wide_combo_grille(frame: &mut Frame, origin: Point, size: Size) {
    let dark = Color::from_rgba(0.29, 0.28, 0.24, 0.34);
    let light = Color::from_rgba(0.88, 0.84, 0.72, 0.30);
    let cell = 18.0;
    let cols = (size.width / cell).ceil() as usize;
    let rows = (size.height / cell).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            let x = origin.x + col as f32 * cell;
            let y = origin.y + row as f32 * cell;
            let x1 = (x + cell).min(origin.x + size.width);
            let y1 = (y + cell).min(origin.y + size.height);
            frame.stroke(
                &Path::line(Point::new(x, y + 2.0), Point::new(x1, y1 - 2.0)),
                Stroke::default().with_color(dark).with_width(0.8),
            );
            frame.stroke(
                &Path::line(Point::new(x1, y + 2.0), Point::new(x, y1 - 2.0)),
                Stroke::default().with_color(light).with_width(0.7),
            );
        }
    }
}

fn draw_amp_status_led(frame: &mut Frame, center: Point, enabled: bool) {
    frame.fill(
        &Path::circle(center, 15.0),
        Color::from_rgba(0.0, 0.78, 0.80, if enabled { 0.22 } else { 0.06 }),
    );
    frame.fill(
        &Path::circle(center, 8.5),
        if enabled {
            Color::from_rgb(0.0, 0.78, 0.82)
        } else {
            Color::from_rgb(0.08, 0.25, 0.28)
        },
    );
    frame.fill(
        &Path::circle(Point::new(center.x - 2.5, center.y - 3.0), 2.0),
        Color::from_rgba(1.0, 1.0, 1.0, if enabled { 0.78 } else { 0.20 }),
    );
    frame.stroke(
        &Path::circle(center, 8.5),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.44))
            .with_width(1.0),
    );
}

fn draw_amp_circuit(frame: &mut Frame, size: Size, app_profile: AppProfile, model: AmpModel) {
    let Some(descriptor) = app_profile
        .amp_descriptor_for_model(model)
        .and_then(|descriptor| (descriptor.circuit)())
        .or_else(|| amp_circuit_descriptor(model.id()))
    else {
        return;
    };
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
        descriptor.label,
        Point::new(origin.x + 34.0, origin.y + 34.0),
        22.0,
        Color::from_rgb(0.90, 0.84, 0.68),
        Horizontal::Left,
    );
    draw_text(
        frame,
        circuit_descriptor_summary(descriptor),
        Point::new(origin.x + 34.0, origin.y + 62.0),
        13.0,
        Color::from_rgba(0.83, 0.78, 0.66, 0.78),
        Horizontal::Left,
    );
    draw_circuit_kind_badge(
        frame,
        Point::new(origin.x + panel_w - 92.0, origin.y + 42.0),
        descriptor.kind,
    );

    let graph_origin = Point::new(origin.x + 44.0, origin.y + 116.0);
    let graph_size = Size::new(panel_w - 88.0, panel_h - 178.0);

    let layout = layout_circuit_graph(descriptor, graph_origin, graph_size);

    for edge in descriptor.edges {
        if let (Some(from), Some(to)) = (
            circuit_placement_by_id(&layout.placements, edge.from),
            circuit_placement_by_id(&layout.placements, edge.to),
        ) {
            if edge.signal == CircuitSignalKind::RailVoltage {
                draw_amp_supply_drop(frame, from.point, to.point);
            } else {
                draw_semantic_circuit_edge(frame, from, to, layout.direction);
            }
        }
    }

    for placement in &layout.placements {
        draw_semantic_circuit_node(frame, placement.point, placement.node);
    }

    draw_text(
        frame,
        "stage-boundary graph, not a complete PCB or SPICE netlist",
        Point::new(origin.x + panel_w * 0.5, origin.y + panel_h - 30.0),
        13.0,
        Color::from_rgba(0.84, 0.76, 0.58, 0.72),
        Horizontal::Center,
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

const EQ_PANEL_X: f32 = 96.0;
const EQ_PANEL_Y: f32 = 82.0;
const EQ_PANEL_W: f32 = 1408.0;
const EQ_PANEL_H: f32 = 392.0;
const EQ_RENDER_SCALE: f32 = 0.78;
const EQ_RENDER_OFFSET_X: f32 = 15.0;
const EQ_RENDER_OFFSET_Y: f32 = 168.0;
const EQ_SLIDER_TOP: f32 = 188.0;
const EQ_SLIDER_H: f32 = 182.0;
const EQ_FIRST_SLIDER_X: f32 = 415.0;
const EQ_SLIDER_SPACING: f32 = 100.125;
const EQ_FILTER_KNOB_RADIUS: f32 = 51.0;
const EQ_FILTER_KNOB_HIT_RADIUS: f32 = 68.0;
const EQ_INK: Color = Color::from_rgb(0.12, 0.07, 0.065);

fn draw_eq_panel(frame: &mut Frame, _size: Size, eq: &EqState) {
    let origin = Point::new(EQ_PANEL_X, EQ_PANEL_Y);
    let hpf_frequency = eq_hpf_frequency_hz(eq.hpf);
    let lpf_frequency = eq_lpf_frequency_hz(eq.lpf);

    draw_text(
        frame,
        &eq_filter_readout("HPF", hpf_frequency),
        Point::new(origin.x + 100.0, origin.y + 64.0),
        18.0,
        EQ_INK,
        Horizontal::Center,
    );
    draw_text(
        frame,
        &eq_filter_readout("LPF", lpf_frequency),
        Point::new(origin.x + EQ_PANEL_W - 100.0, origin.y + 64.0),
        18.0,
        EQ_INK,
        Horizontal::Center,
    );

    draw_eq_filter_knob(
        frame,
        eq_hpf_knob_center(),
        eq.hpf,
        eq.enabled,
        hpf_frequency.is_some(),
    );
    draw_eq_filter_knob(
        frame,
        eq_lpf_knob_center(),
        eq.lpf,
        eq.enabled,
        lpf_frequency.is_some(),
    );
    for index in 0..EQ_BAND_COUNT {
        let x = EQ_FIRST_SLIDER_X + index as f32 * EQ_SLIDER_SPACING;
        let value = eq.bands[index];
        draw_eq_slider(frame, index, x, value, eq.enabled);
    }
}

fn draw_eq_filter_knob(frame: &mut Frame, center: Point, value: f32, enabled: bool, active: bool) {
    let radius = EQ_FILTER_KNOB_RADIUS;
    let alpha = if enabled { 1.0 } else { 0.62 };
    if active && enabled {
        frame.stroke(
            &Path::circle(center, radius + 14.0),
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 0.46, 0.08, 0.34))
                .with_width(7.0),
        );
        frame.fill(
            &Path::circle(center, radius + 18.0),
            Color::from_rgba(1.0, 0.40, 0.05, 0.08),
        );
    }

    let indicator_angle = eq_knob_angle(value.clamp(0.0, 1.0));
    let indicator_start = Point::new(
        center.x + indicator_angle.cos() * (radius * 0.25),
        center.y + indicator_angle.sin() * (radius * 0.25),
    );
    let indicator_end = Point::new(
        center.x + indicator_angle.cos() * (radius * 0.74),
        center.y + indicator_angle.sin() * (radius * 0.74),
    );
    frame.stroke(
        &Path::line(indicator_start, indicator_end),
        Stroke::default()
            .with_color(if active && enabled {
                Color::from_rgb(0.96, 0.66, 0.22)
            } else {
                Color::from_rgba(1.0, 0.82, 0.66, 0.96 * alpha)
            })
            .with_width(5.0),
    );
    frame.fill(
        &Path::circle(indicator_end, 3.6),
        if active && enabled {
            Color::from_rgb(1.0, 0.78, 0.24)
        } else {
            Color::from_rgba(1.0, 0.86, 0.70, 0.88 * alpha)
        },
    );
}

fn eq_knob_angle(value: f32) -> f32 {
    (135.0 + value.clamp(0.0, 1.0) * 270.0).to_radians()
}

fn draw_eq_slider(frame: &mut Frame, index: usize, x: f32, value: f32, enabled: bool) {
    let top = EQ_SLIDER_TOP;
    let bottom = EQ_SLIDER_TOP + EQ_SLIDER_H;
    let value = value.clamp(0.0, 1.0);
    let label_alpha = if enabled { 1.0 } else { 0.72 };
    draw_text(
        frame,
        &eq_band_readout(value),
        Point::new(x, top - 42.0),
        17.0,
        Color::from_rgba(EQ_INK.r, EQ_INK.g, EQ_INK.b, label_alpha),
        Horizontal::Center,
    );

    draw_text(
        frame,
        EQ_BAND_LABELS[index],
        Point::new(x, bottom + 34.0),
        17.0,
        Color::from_rgba(EQ_INK.r, EQ_INK.g, EQ_INK.b, label_alpha),
        Horizontal::Center,
    );
}

fn draw_pedal(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    app_profile: AppProfile,
    device: &DeviceState,
    color: Color,
    _selected: bool,
    circuit_view: bool,
) {
    let render_spec = device_render_spec(app_profile, device.model);
    let uses_asset = render_assets_enabled() && render_spec.asset.is_some() && !circuit_view;

    if !uses_asset {
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

        if device.model == DeviceModel::Minotaur {
            draw_minotaur_enclosure_art(frame, origin, size);
        }
    }

    if circuit_view {
        draw_pedal_circuit(frame, origin, size, app_profile, device);
        return;
    }

    if device.model == DeviceModel::CabIr {
        let knob_y = origin.y + size.height * 0.155;
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
    } else {
        draw_pedal_controls(frame, origin, size, app_profile, device);
    }

    if uses_asset {
        draw_runtime_bypass_controls(frame, origin, size, app_profile, device);
        return;
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

    let led_center = Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.69);
    if device.model == DeviceModel::Minotaur {
        draw_minotaur_jewel(frame, led_center, !device.bypassed);
    } else {
        draw_status_led(frame, led_center, 11.0, !device.bypassed);
    }

    if let Some(control) = device_render_spec(app_profile, device.model)
        .controls
        .iter()
        .find(|control| control.role == RenderControlRole::Bypass)
    {
        draw_footswitch(frame, render_control_center(control, origin, size));
    }
}

fn draw_pedal_controls(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    app_profile: AppProfile,
    device: &DeviceState,
) {
    let render_spec = device_render_spec(app_profile, device.model);
    let draw_labels = render_spec.typography == RenderTypographyPolicy::DrawnByUi;

    for control in render_spec.controls {
        let RenderControlRole::Parameter(kind) = control.role else {
            continue;
        };
        if control.widget != RenderControlWidget::Pot {
            continue;
        }
        if control.asset.is_some() {
            if draw_labels {
                let center = render_control_center(control, origin, size);
                let label_color = match device.model {
                    DeviceModel::Lumen => Color::from_rgba(0.18, 0.16, 0.12, 0.86),
                    DeviceModel::Minotaur => Color::from_rgba(0.20, 0.10, 0.02, 0.92),
                    _ => Color::from_rgba(0.86, 0.90, 0.94, 0.88),
                };
                draw_text(
                    frame,
                    control.label,
                    Point::new(center.x, center.y + control.radius + 20.0),
                    14.0,
                    label_color,
                    Horizontal::Center,
                );
            }
            continue;
        }
        draw_component_knob(
            frame,
            render_control_center(control, origin, size),
            control.radius,
            if draw_labels { control.label } else { "" },
            device.control_value(kind),
            control.skin,
        );
    }
}

fn draw_runtime_bypass_controls(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    app_profile: AppProfile,
    device: &DeviceState,
) {
    for control in device_render_spec(app_profile, device.model).controls {
        if control.role != RenderControlRole::Bypass || control.asset.is_some() {
            continue;
        }

        let center = render_control_center(control, origin, size);
        match control.widget {
            RenderControlWidget::Led => {
                draw_status_led(frame, center, control.radius, !device.bypassed);
            }
            RenderControlWidget::Footswitch => draw_footswitch(frame, center),
            _ => {}
        }
    }
}

fn draw_minotaur_enclosure_art(frame: &mut Frame, origin: Point, size: Size) {
    let inset = size.width * 0.065;
    let plate_origin = Point::new(origin.x + inset, origin.y + size.height * 0.060);
    let plate_size = Size::new(size.width - inset * 2.0, size.height * 0.855);
    let plate = rounded_rect(plate_origin, plate_size, 12.0);

    frame.fill(&plate, Color::from_rgba(0.98, 0.76, 0.30, 0.22));
    frame.stroke(
        &plate,
        Stroke::default()
            .with_color(Color::from_rgba(0.38, 0.22, 0.05, 0.38))
            .with_width(1.8),
    );

    let inner = rounded_rect(
        Point::new(plate_origin.x + 10.0, plate_origin.y + 10.0),
        Size::new(plate_size.width - 20.0, plate_size.height - 20.0),
        9.0,
    );
    frame.stroke(
        &inner,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.92, 0.56, 0.56))
            .with_width(1.3),
    );

    for index in 0..18 {
        let t = index as f32 / 17.0;
        let y = plate_origin.y + 22.0 + t * (plate_size.height - 44.0);
        let alpha = if index % 3 == 0 { 0.13 } else { 0.07 };
        frame.stroke(
            &Path::line(
                Point::new(plate_origin.x + 18.0, y),
                Point::new(plate_origin.x + plate_size.width - 18.0, y + 3.0),
            ),
            Stroke::default()
                .with_color(Color::from_rgba(0.35, 0.20, 0.06, alpha))
                .with_width(0.7),
        );
    }

    for (x, y) in [(0.12, 0.10), (0.88, 0.10), (0.12, 0.91), (0.88, 0.91)] {
        draw_minotaur_screw(
            frame,
            Point::new(
                plate_origin.x + plate_size.width * x,
                plate_origin.y + plate_size.height * y,
            ),
        );
    }

    let sun = Point::new(origin.x + size.width * 0.50, origin.y + size.height * 0.185);
    frame.fill(
        &Path::circle(sun, size.width * 0.095),
        Color::from_rgba(1.0, 0.82, 0.36, 0.28),
    );
    frame.stroke(
        &Path::circle(sun, size.width * 0.097),
        Stroke::default()
            .with_color(Color::from_rgba(0.48, 0.27, 0.04, 0.34))
            .with_width(1.2),
    );
    for ray in 0..32 {
        let angle = ray as f32 / 32.0 * std::f32::consts::TAU;
        let inner_r = size.width * 0.12;
        let outer_r = size.width * if ray % 2 == 0 { 0.34 } else { 0.27 };
        let start = Point::new(sun.x + angle.cos() * inner_r, sun.y + angle.sin() * inner_r);
        let end = Point::new(sun.x + angle.cos() * outer_r, sun.y + angle.sin() * outer_r);
        frame.stroke(
            &Path::line(start, end),
            Stroke::default()
                .with_color(Color::from_rgba(0.45, 0.25, 0.04, 0.14))
                .with_width(if ray % 2 == 0 { 1.0 } else { 0.55 }),
        );
    }

    for control in MINOTAUR_PEDAL_CONTROLS {
        let RenderControlRole::Parameter(_) = control.role else {
            continue;
        };
        let center = render_control_center(control, origin, size);
        draw_minotaur_control_bezel(frame, center, control.radius * 1.34);
    }
}

fn draw_minotaur_control_bezel(frame: &mut Frame, center: Point, radius: f32) {
    frame.fill(
        &Path::circle(Point::new(center.x + 2.0, center.y + 3.0), radius),
        Color::from_rgba(0.20, 0.10, 0.02, 0.22),
    );
    frame.fill(
        &Path::circle(center, radius),
        Color::from_rgba(1.0, 0.77, 0.30, 0.42),
    );
    frame.stroke(
        &Path::circle(center, radius),
        Stroke::default()
            .with_color(Color::from_rgba(0.34, 0.18, 0.03, 0.46))
            .with_width(1.8),
    );
    frame.stroke(
        &Path::circle(center, radius * 0.76),
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.93, 0.60, 0.55))
            .with_width(1.2),
    );
}

fn draw_minotaur_screw(frame: &mut Frame, center: Point) {
    frame.fill(
        &Path::circle(Point::new(center.x + 1.5, center.y + 2.0), 12.0),
        Color::from_rgba(0.12, 0.06, 0.01, 0.30),
    );
    frame.fill(
        &Path::circle(center, 11.5),
        Color::from_rgb(0.83, 0.58, 0.20),
    );
    frame.stroke(
        &Path::circle(center, 11.5),
        Stroke::default()
            .with_color(Color::from_rgba(0.26, 0.13, 0.02, 0.55))
            .with_width(1.1),
    );
    frame.stroke(
        &Path::line(
            Point::new(center.x - 6.0, center.y),
            Point::new(center.x + 6.0, center.y),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.18, 0.09, 0.02, 0.68))
            .with_width(2.1),
    );
    frame.stroke(
        &Path::line(
            Point::new(center.x, center.y - 6.0),
            Point::new(center.x, center.y + 6.0),
        ),
        Stroke::default()
            .with_color(Color::from_rgba(0.18, 0.09, 0.02, 0.68))
            .with_width(2.1),
    );
}

fn draw_minotaur_jewel(frame: &mut Frame, center: Point, active: bool) {
    let glow = if active { 0.32 } else { 0.10 };
    frame.fill(
        &Path::circle(center, 23.0),
        Color::from_rgba(1.0, 0.54, 0.02, glow),
    );
    frame.fill(
        &Path::circle(center, 13.0),
        if active {
            Color::from_rgb(1.0, 0.56, 0.04)
        } else {
            Color::from_rgb(0.48, 0.25, 0.05)
        },
    );
    frame.stroke(
        &Path::circle(center, 15.5),
        Stroke::default()
            .with_color(Color::from_rgba(0.10, 0.06, 0.02, 0.72))
            .with_width(2.0),
    );
    for index in 0..8 {
        let angle = index as f32 / 8.0 * std::f32::consts::TAU;
        frame.stroke(
            &Path::line(
                center,
                Point::new(center.x + angle.cos() * 11.0, center.y + angle.sin() * 11.0),
            ),
            Stroke::default()
                .with_color(Color::from_rgba(
                    1.0,
                    0.90,
                    0.42,
                    if active { 0.42 } else { 0.18 },
                ))
                .with_width(0.8),
        );
    }
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

fn draw_pedal_circuit(
    frame: &mut Frame,
    origin: Point,
    size: Size,
    app_profile: AppProfile,
    device: &DeviceState,
) {
    let Some(descriptor) = ui_circuit_descriptor(app_profile, device.model) else {
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
}

fn ui_circuit_descriptor(
    app_profile: AppProfile,
    model: DeviceModel,
) -> Option<&'static CircuitDescriptor> {
    app_profile
        .device_descriptor_for_model(model)
        .and_then(|descriptor| (descriptor.circuit)())
        .or_else(|| match model {
            DeviceModel::Lumen => device_circuit_descriptor(CoreDeviceConfig::Lumen),
            DeviceModel::Muffin => device_circuit_descriptor(CoreDeviceConfig::Muffin),
            DeviceModel::Minotaur => device_circuit_descriptor(CoreDeviceConfig::Minotaur),
            DeviceModel::Monarch => device_circuit_descriptor(CoreDeviceConfig::Monarch),
            DeviceModel::Nox30 => amp_circuit_descriptor("nox30"),
            DeviceModel::Springfield => device_circuit_descriptor(CoreDeviceConfig::Springfield),
            _ => None,
        })
}

fn circuit_descriptor_summary(descriptor: &CircuitDescriptor) -> &'static str {
    match descriptor.model_id {
        "nox30" => "input / ECC83 / top boost / phase inverter / EL84 / B+ sag / transformer",
        "lumen" => "input / sidechain / opto memory / gain cell / tube soften / mix / output",
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
        CircuitNodeKind::PhaseInverter
        | CircuitNodeKind::PowerStage
        | CircuitNodeKind::SupplyNetwork
        | CircuitNodeKind::Transformer => (54.0, 38.0),
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
        "input_volume" => "Input",
        "first_stage" => "V1",
        "cathode_follower" => "CF",
        "tone_stack" => "Tone",
        "recovery_stage" => "Recover",
        "phase_inverter" => "PI",
        "cut_presence" => "Cut",
        "power_stage" => "EL84",
        "supply_network" => "B+",
        "output_transformer" => "OT",
        "speaker_out" => "OUT",
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
        CircuitNodeKind::PhaseInverter => "split",
        CircuitNodeKind::PowerStage => "power",
        CircuitNodeKind::SupplyNetwork => "rail",
        CircuitNodeKind::Transformer => "flux",
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
        CircuitNodeKind::PhaseInverter => Color::from_rgb(0.20, 0.22, 0.18),
        CircuitNodeKind::PowerStage => Color::from_rgb(0.34, 0.18, 0.12),
        CircuitNodeKind::SupplyNetwork | CircuitNodeKind::Transformer => {
            Color::from_rgb(0.13, 0.22, 0.28)
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
        CircuitNodeKind::SupplyNetwork | CircuitNodeKind::Transformer => {
            Color::from_rgba(0.50, 0.78, 0.95, 0.58)
        }
        CircuitNodeKind::PhaseInverter | CircuitNodeKind::PowerStage => {
            Color::from_rgba(1.0, 0.66, 0.42, 0.58)
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
        "peak_reduction" => "PR",
        "emphasis" => "E",
        "gain" => "G",
        "treble" => "T",
        "output" => "O",
        "dwell" => "D",
        "tone" => "T",
        "mix" => "M",
        "volume" => "V",
        "bass" => "B",
        "cut" => "C",
        "drive" => "D",
        "presence" => "P",
        "sag" => "S",
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
    draw_footswitch_scaled(frame, center, 1.0);
}

fn draw_footswitch_scaled(frame: &mut Frame, center: Point, scale: f32) {
    let radius = |value: f32| value * scale;
    frame.fill(
        &Path::circle(
            Point::new(center.x + 4.0 * scale, center.y + 8.0 * scale),
            radius(43.0),
        ),
        Color::from_rgba(0.02, 0.015, 0.01, 0.30),
    );
    frame.fill(
        &Path::circle(center, radius(43.0)),
        Color::from_rgb(0.49, 0.34, 0.18),
    );
    frame.fill(&Path::circle(center, radius(34.0)), GOLD);
    frame.stroke(
        &Path::circle(center, radius(34.0)),
        Stroke::default()
            .with_color(Color::from_rgb(0.94, 0.82, 0.57))
            .with_width(3.0 * scale),
    );
    frame.fill(
        &Path::circle(center, radius(23.0)),
        Color::from_rgb(0.11, 0.07, 0.05),
    );
    frame.fill(
        &Path::circle(Point::new(center.x, center.y - 4.0 * scale), radius(18.0)),
        TEAL,
    );
    frame.stroke(
        &Path::circle(Point::new(center.x, center.y - 4.0 * scale), radius(18.0)),
        Stroke::default()
            .with_color(Color::from_rgb(0.92, 0.86, 0.70))
            .with_width(2.0 * scale),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amp_spine_selects_each_available_amp_in_one_click() {
        let profile = AppProfile::greybound_glass();
        let size = Size::new(DESIGN_WIDTH, 666.0);

        for (model, row) in amp_spine_layout(profile, size) {
            let center = Point::new(row.x + row.width * 0.5, row.y + row.height * 0.5);
            assert_eq!(hit_test_amp_spine(profile, size, center), Some(model));
        }
    }

    #[test]
    fn amp_spine_reduces_long_model_names_after_eight_characters() {
        let row_height = 140.0;

        assert_eq!(
            amp_spine_name_size("NOX 30", row_height),
            amp_spine_name_size("DAYBREAK", row_height)
        );
        assert!(
            amp_spine_name_size("DAYBREAKER", row_height)
                < amp_spine_name_size("DAYBREAK", row_height)
        );
    }

    #[test]
    fn auralith_footswitch_hit_test_ignores_status_led() {
        let devices = vec![BoardDeviceSlot {
            source_index: 42,
            device: DeviceState::auralith(),
        }];
        let size = Size::new(640.0, 920.0);
        let layout = board_layout(devices.len(), size);
        let origin = Point::new(layout.start_x, pedal_board_y(size, layout.pedal_h));
        let pedal_size = Size::new(layout.pedal_w, layout.pedal_h);
        let footswitch = AURALITH_PEDAL_CONTROLS
            .iter()
            .find(|control| control.widget == RenderControlWidget::Footswitch)
            .expect("Auralith must expose a clickable footswitch control");
        let center = render_control_center(footswitch, origin, pedal_size);

        assert_eq!(
            hit_test_pedal_footswitch(AppProfile::greybound_free(), &devices, size, center),
            Some(42)
        );
    }

    #[test]
    fn auralith_controls_follow_its_baked_faceplate_layout() {
        for (kind, anchor_x, anchor_y) in [
            (ControlKind::Gain, 0.246, 0.217),
            (ControlKind::Bass, 0.493, 0.217),
            (ControlKind::Cut, 0.738, 0.217),
            (ControlKind::Treble, 0.246, 0.422),
            (ControlKind::Presence, 0.493, 0.422),
            (ControlKind::Master, 0.738, 0.422),
        ] {
            let control = AURALITH_PEDAL_CONTROLS
                .iter()
                .find(|control| control.role == RenderControlRole::Parameter(kind))
                .expect("Auralith must expose every baked control");

            assert_eq!((control.anchor_x, control.anchor_y), (anchor_x, anchor_y));
        }

        let footswitch = AURALITH_PEDAL_CONTROLS
            .iter()
            .find(|control| control.widget == RenderControlWidget::Footswitch)
            .expect("Auralith must expose a footswitch");
        assert_eq!((footswitch.anchor_x, footswitch.anchor_y), (0.493, 0.804));
        assert_eq!(
            AURALITH_PEDAL_RENDER_SPEC.typography,
            RenderTypographyPolicy::BakedIntoAsset
        );
    }

    #[test]
    fn bypass_asset_value_lights_led_when_device_is_active() {
        assert_eq!(bypass_asset_value(true), 0.0);
        assert_eq!(bypass_asset_value(false), 1.0);
    }

    #[test]
    fn free_runtime_snapshot_places_bypassed_monarch_before_minotaur() {
        let snapshot = GreyboundUi::default().runtime_audio_snapshot();

        assert_eq!(snapshot.devices.len(), 6);
        assert!(matches!(
            snapshot.devices[0].controls,
            CoreDeviceControls::Lumen(_)
        ));
        assert!(snapshot.devices[0].bypassed);
        assert!(matches!(
            snapshot.devices[1].controls,
            CoreDeviceControls::Muffin(_)
        ));
        assert!(snapshot.devices[1].bypassed);
        assert!(matches!(
            snapshot.devices[2].controls,
            CoreDeviceControls::Monarch(_)
        ));
        assert!(snapshot.devices[2].bypassed);
        assert!(matches!(
            snapshot.devices[3].controls,
            CoreDeviceControls::Minotaur(_)
        ));
        assert!(!snapshot.devices[3].bypassed);
    }

    #[test]
    fn minotaur_footswitch_asset_decodes() {
        let footswitch = MINOTAUR_PEDAL_CONTROLS
            .iter()
            .find(|control| control.widget == RenderControlWidget::Footswitch)
            .expect("Minotaur must expose a footswitch");

        assert!(footswitch.asset.is_some());
        assert!(render_control_asset_handle(MINOTAUR_SILVER_FOOTSWITCH_ASSET, 0.0).is_some());
    }

    #[test]
    fn muffin_visual_asset_decodes() {
        let image = image::load_from_memory(include_bytes!("../assets/pedals/muffin@4x.png"))
            .expect("Muffin faceplate asset must decode");
        assert_eq!(image.width(), 1200);
        assert_eq!(image.height(), 2172);
        assert!(image.color().has_alpha());
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/pedals/muffin@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2172,
        })
        .is_some());
        assert!(MUFFIN_PEDAL_CONTROLS
            .iter()
            .all(|control| control.asset.is_some()));
        assert!(render_control_asset_handle(MUFFIN_BLACK_BRASS_KNOB_ASSET, 0.5).is_some());
        assert!(render_control_asset_handle(MUFFIN_JEWEL_LED_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(MUFFIN_JEWEL_LED_ASSET, 1.0).is_some());
        assert!(render_control_asset_handle(MUFFIN_FOOTSWITCH_ASSET, 0.0).is_some());
    }

    #[test]
    fn monarch_visual_asset_decodes() {
        let image = image::load_from_memory(include_bytes!("../assets/pedals/monarch@4x.png"))
            .expect("Monarch faceplate asset must decode");
        assert_eq!(image.width(), 1200);
        assert_eq!(image.height(), 2172);
        assert!(image.color().has_alpha());
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/pedals/monarch@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2172,
        })
        .is_some());
        assert_eq!(
            MONARCH_PEDAL_RENDER_SPEC.typography,
            RenderTypographyPolicy::BakedIntoAsset
        );
        assert!(MONARCH_PEDAL_CONTROLS
            .iter()
            .all(|control| control.asset.is_some()));
        assert!(render_control_asset_handle(MONARCH_BRASS_KNOB_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(MONARCH_BRASS_KNOB_ASSET, 0.5).is_some());
        assert!(render_control_asset_handle(MONARCH_JEWEL_LED_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(MONARCH_JEWEL_LED_ASSET, 1.0).is_some());
        assert!(render_control_asset_handle(MONARCH_BRASS_FOOTSWITCH_ASSET, 0.0).is_some());
    }

    #[test]
    fn lumen_render_assets_decode() {
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/pedals/lumen@4x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1200,
            pixel_height: 2172,
        })
        .is_some());
        assert!(render_control_asset_handle(LUMEN_PEARL_KNOB_ASSET, 0.5).is_some());
        assert!(render_control_asset_handle(LUMEN_JEWEL_LED_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(LUMEN_JEWEL_LED_ASSET, 1.0).is_some());
        assert!(render_control_asset_handle(LUMEN_FOOTSWITCH_ASSET, 0.0).is_some());
    }

    #[test]
    fn pedalboard_studio_backdrop_decodes() {
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/surfaces/pedalboard-studio@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1807,
            pixel_height: 870,
        })
        .is_some());
    }

    #[test]
    fn amp_studio_backdrop_decodes() {
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/surfaces/amp-studio@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1672,
            pixel_height: 941,
        })
        .is_some());
    }

    #[test]
    fn photorealistic_cab_render_asset_decodes() {
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/cabs/greybound-2x12@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1821,
            pixel_height: 864,
        })
        .is_some());
    }

    #[test]
    fn daybreaker_uses_a_transparent_base_with_separate_controls() {
        let image = image::load_from_memory(include_bytes!(
            "../assets/amps/daybreaker-50-stack-badged-v4@2x.png"
        ))
        .expect("Daybreaker base asset must decode");

        assert_eq!(image.width(), 2480);
        assert_eq!(image.height(), 1000);
        assert!(image.color().has_alpha());
        assert_eq!(DAYBREAKER_AMP_CONTROLS.len(), 8);
        assert!(DAYBREAKER_AMP_CONTROLS
            .iter()
            .all(|control| control.asset.is_some()));
        assert!(render_control_asset_handle(DAYBREAKER_BLACK_BRASS_KNOB_ASSET, 0.5).is_some());
        assert!(render_control_asset_handle(DAYBREAKER_POWER_SWITCH_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(DAYBREAKER_POWER_SWITCH_ASSET, 1.0).is_some());
        assert!(render_control_asset_handle(DAYBREAKER_JEWEL_LED_ASSET, 0.0).is_some());
        assert!(render_control_asset_handle(DAYBREAKER_JEWEL_LED_ASSET, 1.0).is_some());
        assert!(render_asset_handle(DAYBREAKER_INPUT_JACK_ASSET).is_some());
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/amps/daybreaker-50-stack-badged-v4@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 2480,
            pixel_height: 1000,
        })
        .is_some());
    }

    #[test]
    fn nam_loader_uses_a_picker_and_maps_external_gain_and_volume() {
        let size = Size::new(DESIGN_WIDTH, MAIN_VIEW_HEIGHT);
        let button = nam_loader_button_bounds(size);

        assert!(amp_knob_layout(size, AmpModel::NamLoader).is_empty());
        assert_eq!(NAM_LOADER_AMP_CONTROLS.len(), 2);
        assert!(matches!(
            NAM_LOADER_AMP_CONTROLS[0].role,
            RenderControlRole::Parameter(ControlKind::Gain)
        ));
        assert!(matches!(
            NAM_LOADER_AMP_CONTROLS[1].role,
            RenderControlRole::Parameter(ControlKind::Master)
        ));
        assert!(hit_test_nam_loader_button(
            size,
            Point::new(
                button.x + button.width * 0.5,
                button.y + button.height * 0.5,
            ),
        ));
        assert!(render_asset_handle(
            NAM_LOADER_AMP_RENDER_SPEC
                .asset
                .expect("NAM Loader must have a rack asset"),
        )
        .is_some());

        let mut ui = GreyboundUi::default();
        ui.update(Message::NamFileSelected(Some(PathBuf::from(
            "/tmp/test-model.nam",
        ))));
        assert_eq!(ui.amp_model, AmpModel::NamLoader);
        assert_eq!(ui.runtime_amp_model_id(), "nam2?path=/tmp/test-model.nam");
        assert!((ui.runtime_audio_snapshot().amp.output - 1.0).abs() < 1e-6);
        assert!((nam_loader_output_gain(0.0) - 0.125_892_53).abs() < 1e-6);
        assert!((nam_loader_output_gain(1.0) - 7.943_282).abs() < 1e-5);
    }

    #[test]
    fn daybreaker_volume_routes_to_master_not_safety_trim() {
        let mut ui = GreyboundUi::default();
        ui.update(Message::SelectAmpModel(AmpModel::Daybreaker50));

        let controls = ui.runtime_audio_snapshot().amp;
        assert!((controls.master - 0.75).abs() < 1e-6);
        assert!((controls.output - 1.0).abs() < 1e-6);
    }

    #[test]
    fn header_brushed_metal_texture_decodes() {
        assert!(render_asset_handle(RenderAssetSpec {
            path: "assets/surfaces/header-brushed-gunmetal@2x.png",
            format: RenderAssetFormat::PngRgba,
            pixel_width: 1774,
            pixel_height: 887,
        })
        .is_some());
    }

    #[test]
    fn atomic_recording_assets_decode() {
        for asset in [
            RenderAssetSpec {
                path: "assets/record/atomic/rack-chassis.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1757,
                pixel_height: 425,
            },
            RenderAssetSpec {
                path: "assets/record/atomic/vu-meter.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1331,
                pixel_height: 705,
            },
            RenderAssetSpec {
                path: "assets/record/atomic/timecode-display.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 1230,
                pixel_height: 364,
            },
            RenderAssetSpec {
                path: "assets/record/atomic/rec-button.png",
                format: RenderAssetFormat::PngRgba,
                pixel_width: 958,
                pixel_height: 965,
            },
        ] {
            assert!(render_asset_handle(asset).is_some(), "{}", asset.path);
        }
    }

    #[test]
    fn global_copper_knob_asset_is_a_standard_transparent_control_surface() {
        let image = image::load_from_memory(include_bytes!(
            "../assets/controls/knobs/global-copper@2x.png"
        ))
        .expect("global copper knob must decode");

        assert_eq!(image.width(), 512);
        assert_eq!(image.height(), 512);
        assert!(image.color().has_alpha());
    }
}
