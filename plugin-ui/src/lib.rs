use baseview::{WindowHandle, WindowOpenOptions, WindowScalePolicy};
use crossbeam::atomic::AtomicCell;
use iced::mouse;
use iced::{Color, Point, Rectangle, Size};
use iced_graphics::Viewport;
use iced_runtime::user_interface::{self, UserInterface};
use nih_plug::prelude::{Editor, GuiContext, ParentWindowHandle};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawWindowHandle};
use std::{
    num::{NonZeroIsize, NonZeroU32},
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PluginUiConfig {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub background: Color,
}

impl Default for PluginUiConfig {
    fn default() -> Self {
        Self {
            title: "Greybound",
            width: 1_600,
            height: 900,
            background: Color::from_rgb(0.72, 0.78, 0.91),
        }
    }
}

pub trait PluginIcedApp: Send + 'static {
    type Message: Clone + std::fmt::Debug + Send + 'static;

    fn on_frame(&mut self) {}
    fn update(&mut self, message: Self::Message);
    fn view(&self) -> iced::Element<'_, Self::Message>;
}

pub fn create_iced_editor<App>(
    config: PluginUiConfig,
    factory: impl Fn(Arc<dyn GuiContext>) -> App + Send + Sync + 'static,
) -> Option<Box<dyn Editor>>
where
    App: PluginIcedApp,
{
    Some(Box::new(IcedPluginEditor {
        config,
        factory: Arc::new(factory),
        state: Arc::new(EditorState {
            size: AtomicCell::new((config.width, config.height)),
            open: AtomicBool::new(false),
        }),
        scaling_factor: AtomicCell::new(None),
    }))
}

struct IcedPluginEditor<App: PluginIcedApp> {
    config: PluginUiConfig,
    factory: Arc<dyn Fn(Arc<dyn GuiContext>) -> App + Send + Sync>,
    state: Arc<EditorState>,
    scaling_factor: AtomicCell<Option<f32>>,
}

impl<App: PluginIcedApp> Editor for IcedPluginEditor<App> {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let (width, height) = self.state.size.load();
        let scaling_factor = self.scaling_factor.load();
        let state = self.state.clone();
        let config = self.config;
        let app = (self.factory)(context);

        let window = baseview::Window::open_parented(
            &ParentWindowHandleAdapter(parent),
            WindowOpenOptions {
                title: config.title.to_owned(),
                size: baseview::Size::new(width as f64, height as f64),
                scale: scaling_factor
                    .map(|factor| WindowScalePolicy::ScaleFactor(factor as f64))
                    .unwrap_or(WindowScalePolicy::SystemScaleFactor),
                gl_config: None,
            },
            move |window| IcedPluginWindow::new(window, state, config, app),
        );

        self.state.open.store(true, Ordering::Release);
        Box::new(EditorHandle {
            state: self.state.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        self.state.size.load()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        if self.state.open.load(Ordering::Acquire) {
            return false;
        }

        self.scaling_factor.store(Some(factor));
        true
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}

    fn param_values_changed(&self) {}
}

struct EditorState {
    size: AtomicCell<(u32, u32)>,
    open: AtomicBool,
}

struct EditorHandle {
    state: Arc<EditorState>,
    window: WindowHandle,
}

unsafe impl Send for EditorHandle {}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        self.state.open.store(false, Ordering::Release);
        self.window.close();
    }
}

struct IcedPluginWindow<App: PluginIcedApp> {
    app: App,
    state: Arc<EditorState>,
    config: PluginUiConfig,
    renderer: iced::Renderer,
    _surface_context: softbuffer::Context<SoftbufferWindowHandleAdapter>,
    surface: softbuffer::Surface<SoftbufferWindowHandleAdapter, SoftbufferWindowHandleAdapter>,
    clip_mask: tiny_skia::Mask,
    cache: user_interface::Cache,
    clipboard: iced_runtime::core::clipboard::Null,
    cursor: mouse::Cursor,
    viewport: Viewport,
    theme: iced::Theme,
}

impl<App: PluginIcedApp> IcedPluginWindow<App> {
    fn new(
        window: &mut baseview::Window<'_>,
        state: Arc<EditorState>,
        config: PluginUiConfig,
        app: App,
    ) -> Self {
        let renderer = create_renderer();
        let target = baseview_window_to_surface_target(window);
        let surface_context =
            softbuffer::Context::new(target.clone()).expect("could not create plugin UI context");
        let mut surface = softbuffer::Surface::new(&surface_context, target)
            .expect("could not create plugin UI surface");
        surface
            .resize(
                NonZeroU32::new(config.width.max(1)).unwrap(),
                NonZeroU32::new(config.height.max(1)).unwrap(),
            )
            .expect("could not resize plugin UI surface");

        let viewport = Viewport::with_physical_size(Size::new(config.width, config.height), 1.0);
        let clip_mask = tiny_skia::Mask::new(config.width.max(1), config.height.max(1))
            .expect("could not create plugin UI clip mask");

        let mut plugin_window = Self {
            app,
            state,
            config,
            renderer,
            _surface_context: surface_context,
            surface,
            clip_mask,
            cache: user_interface::Cache::new(),
            clipboard: iced_runtime::core::clipboard::Null,
            cursor: mouse::Cursor::Unavailable,
            viewport,
            theme: iced::Theme::Dark,
        };
        plugin_window.draw();
        plugin_window
    }

    fn handle_iced_events(&mut self, events: Vec<iced::Event>) {
        let mut ui = UserInterface::build(
            self.app.view(),
            self.viewport.logical_size(),
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        let mut messages = Vec::new();
        let _ = ui.update(
            &events,
            self.cursor,
            &mut self.renderer,
            &mut self.clipboard,
            &mut messages,
        );
        self.cache = ui.into_cache();

        for message in messages {
            self.app.update(message);
        }

        self.draw();
    }

    fn draw(&mut self) {
        self.app.on_frame();

        let mut ui = UserInterface::build(
            self.app.view(),
            self.viewport.logical_size(),
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        let _interaction = ui.draw(
            &mut self.renderer,
            &self.theme,
            &iced_runtime::core::renderer::Style::default(),
            self.cursor,
        );
        self.cache = ui.into_cache();

        self.present_primitives();
    }

    fn resize(
        &mut self,
        logical_width: f64,
        logical_height: f64,
        physical_width: u32,
        physical_height: u32,
    ) {
        let logical_width = logical_width.max(1.0);
        let logical_height = logical_height.max(1.0);
        let physical_width = physical_width.max(1);
        let physical_height = physical_height.max(1);
        let scale = (physical_width as f64 / logical_width)
            .max(physical_height as f64 / logical_height)
            .max(0.25);

        self.state
            .size
            .store((logical_width.round() as u32, logical_height.round() as u32));
        self.viewport =
            Viewport::with_physical_size(Size::new(physical_width, physical_height), scale);
        self.resize_surface(physical_width, physical_height);
        self.draw();
    }

    fn resize_surface(&mut self, physical_width: u32, physical_height: u32) {
        self.surface
            .resize(
                NonZeroU32::new(physical_width.max(1)).unwrap(),
                NonZeroU32::new(physical_height.max(1)).unwrap(),
            )
            .expect("could not resize plugin UI surface");
        self.clip_mask = tiny_skia::Mask::new(physical_width.max(1), physical_height.max(1))
            .expect("could not resize plugin UI clip mask");
    }

    fn present_primitives(&mut self) {
        let physical_size = self.viewport.physical_size();
        let damage = [Rectangle::with_size(Size::new(
            physical_size.width as f32,
            physical_size.height as f32,
        ))];
        let mut buffer = self
            .surface
            .buffer_mut()
            .expect("could not acquire plugin UI surface buffer");
        let pixel_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.as_mut_ptr().cast::<u8>(),
                buffer.len() * std::mem::size_of::<u32>(),
            )
        };
        let mut pixels = tiny_skia::PixmapMut::from_bytes(
            pixel_bytes,
            physical_size.width,
            physical_size.height,
        )
        .expect("could not create plugin UI pixmap");

        #[allow(unreachable_patterns)]
        match &mut self.renderer {
            iced_renderer::Renderer::TinySkia(renderer) => {
                renderer.with_primitives(|backend, primitives| {
                    backend.draw(
                        &mut pixels,
                        &mut self.clip_mask,
                        primitives,
                        &self.viewport,
                        &damage,
                        self.config.background,
                        &[] as &[&str],
                    );
                });
            }
            _ => {}
        }

        buffer
            .present()
            .expect("could not present plugin UI surface");
    }
}

impl<App: PluginIcedApp> baseview::WindowHandler for IcedPluginWindow<App> {
    fn on_frame(&mut self, _window: &mut baseview::Window) {
        self.draw();
    }

    fn on_event(
        &mut self,
        _window: &mut baseview::Window,
        event: baseview::Event,
    ) -> baseview::EventStatus {
        let iced_event = match event {
            baseview::Event::Window(baseview::WindowEvent::Resized(info)) => {
                let logical = info.logical_size();
                let physical = info.physical_size();
                self.resize(
                    logical.width,
                    logical.height,
                    physical.width,
                    physical.height,
                );
                Some(iced::Event::Window(iced::window::Event::Resized {
                    width: logical.width as u32,
                    height: logical.height as u32,
                }))
            }
            baseview::Event::Window(baseview::WindowEvent::Focused) => {
                Some(iced::Event::Window(iced::window::Event::Focused))
            }
            baseview::Event::Window(baseview::WindowEvent::Unfocused) => {
                Some(iced::Event::Window(iced::window::Event::Unfocused))
            }
            baseview::Event::Window(baseview::WindowEvent::WillClose) => {
                Some(iced::Event::Window(iced::window::Event::CloseRequested))
            }
            baseview::Event::Mouse(baseview::MouseEvent::CursorMoved { position, .. }) => {
                let position = Point::new(position.x as f32, position.y as f32);
                self.cursor = mouse::Cursor::Available(position);
                Some(iced::Event::Mouse(mouse::Event::CursorMoved { position }))
            }
            baseview::Event::Mouse(baseview::MouseEvent::CursorEntered) => None,
            baseview::Event::Mouse(baseview::MouseEvent::CursorLeft) => {
                self.cursor = mouse::Cursor::Unavailable;
                Some(iced::Event::Mouse(mouse::Event::CursorLeft))
            }
            baseview::Event::Mouse(baseview::MouseEvent::ButtonPressed { button, .. }) => {
                map_mouse_button(button)
                    .map(|button| iced::Event::Mouse(mouse::Event::ButtonPressed(button)))
            }
            baseview::Event::Mouse(baseview::MouseEvent::ButtonReleased { button, .. }) => {
                map_mouse_button(button)
                    .map(|button| iced::Event::Mouse(mouse::Event::ButtonReleased(button)))
            }
            baseview::Event::Mouse(baseview::MouseEvent::WheelScrolled { delta, .. }) => {
                Some(iced::Event::Mouse(mouse::Event::WheelScrolled {
                    delta: match delta {
                        baseview::ScrollDelta::Lines { x, y } => mouse::ScrollDelta::Lines { x, y },
                        baseview::ScrollDelta::Pixels { x, y } => {
                            mouse::ScrollDelta::Pixels { x, y }
                        }
                    },
                }))
            }
            baseview::Event::Keyboard(_) => None,
            baseview::Event::Mouse(_) => None,
        };

        if let Some(event) = iced_event {
            self.handle_iced_events(vec![event]);
        }

        baseview::EventStatus::Captured
    }
}

fn map_mouse_button(button: baseview::MouseButton) -> Option<mouse::Button> {
    match button {
        baseview::MouseButton::Left => Some(mouse::Button::Left),
        baseview::MouseButton::Right => Some(mouse::Button::Right),
        baseview::MouseButton::Middle => Some(mouse::Button::Middle),
        baseview::MouseButton::Back => Some(mouse::Button::Other(4)),
        baseview::MouseButton::Forward => Some(mouse::Button::Other(5)),
        baseview::MouseButton::Other(value) => Some(mouse::Button::Other(value.into())),
    }
}

fn create_renderer() -> iced::Renderer {
    let settings = iced_renderer::Settings::default();
    let (_, backend) =
        iced_tiny_skia::window::compositor::new::<iced::Theme>(iced_tiny_skia::Settings {
            default_font: settings.default_font,
            default_text_size: settings.default_text_size,
        });

    iced_renderer::Renderer::TinySkia(iced_tiny_skia::Renderer::new(backend))
}

#[derive(Clone)]
struct SoftbufferWindowHandleAdapter {
    raw_display_handle: raw_window_handle_06::RawDisplayHandle,
    raw_window_handle: raw_window_handle_06::RawWindowHandle,
}

impl raw_window_handle_06::HasDisplayHandle for SoftbufferWindowHandleAdapter {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle_06::DisplayHandle<'_>, raw_window_handle_06::HandleError> {
        unsafe {
            Ok(raw_window_handle_06::DisplayHandle::borrow_raw(
                self.raw_display_handle,
            ))
        }
    }
}

impl raw_window_handle_06::HasWindowHandle for SoftbufferWindowHandleAdapter {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle_06::WindowHandle<'_>, raw_window_handle_06::HandleError> {
        unsafe {
            Ok(raw_window_handle_06::WindowHandle::borrow_raw(
                self.raw_window_handle,
            ))
        }
    }
}

fn baseview_window_to_surface_target(
    window: &baseview::Window<'_>,
) -> SoftbufferWindowHandleAdapter {
    let raw_display_handle = window.raw_display_handle();
    let raw_window_handle = window.raw_window_handle();

    SoftbufferWindowHandleAdapter {
        raw_display_handle: match raw_display_handle {
            raw_window_handle::RawDisplayHandle::AppKit(_) => {
                raw_window_handle_06::RawDisplayHandle::AppKit(
                    raw_window_handle_06::AppKitDisplayHandle::new(),
                )
            }
            raw_window_handle::RawDisplayHandle::Xlib(handle) => {
                raw_window_handle_06::RawDisplayHandle::Xlib(
                    raw_window_handle_06::XlibDisplayHandle::new(
                        NonNull::new(handle.display),
                        handle.screen,
                    ),
                )
            }
            raw_window_handle::RawDisplayHandle::Xcb(handle) => {
                raw_window_handle_06::RawDisplayHandle::Xcb(
                    raw_window_handle_06::XcbDisplayHandle::new(
                        NonNull::new(handle.connection),
                        handle.screen,
                    ),
                )
            }
            raw_window_handle::RawDisplayHandle::Windows(_) => {
                raw_window_handle_06::RawDisplayHandle::Windows(
                    raw_window_handle_06::WindowsDisplayHandle::new(),
                )
            }
            _ => panic!("unsupported plugin UI display handle"),
        },
        raw_window_handle: match raw_window_handle {
            raw_window_handle::RawWindowHandle::AppKit(handle) => {
                raw_window_handle_06::RawWindowHandle::AppKit(
                    raw_window_handle_06::AppKitWindowHandle::new(
                        NonNull::new(handle.ns_view).unwrap(),
                    ),
                )
            }
            raw_window_handle::RawWindowHandle::Xlib(handle) => {
                raw_window_handle_06::RawWindowHandle::Xlib(
                    raw_window_handle_06::XlibWindowHandle::new(handle.window),
                )
            }
            raw_window_handle::RawWindowHandle::Xcb(handle) => {
                raw_window_handle_06::RawWindowHandle::Xcb(
                    raw_window_handle_06::XcbWindowHandle::new(
                        NonZeroU32::new(handle.window).unwrap(),
                    ),
                )
            }
            raw_window_handle::RawWindowHandle::Win32(handle) => {
                let mut raw_handle = raw_window_handle_06::Win32WindowHandle::new(
                    NonZeroIsize::new(handle.hwnd as isize).unwrap(),
                );
                raw_handle.hinstance = NonZeroIsize::new(handle.hinstance as isize);

                raw_window_handle_06::RawWindowHandle::Win32(raw_handle)
            }
            _ => panic!("unsupported plugin UI window handle"),
        },
    }
}

struct ParentWindowHandleAdapter(ParentWindowHandle);

unsafe impl HasRawWindowHandle for ParentWindowHandleAdapter {
    fn raw_window_handle(&self) -> RawWindowHandle {
        match self.0 {
            ParentWindowHandle::X11Window(window) => {
                let mut handle = raw_window_handle::XcbWindowHandle::empty();
                handle.window = window;
                RawWindowHandle::Xcb(handle)
            }
            ParentWindowHandle::AppKitNsView(ns_view) => {
                let mut handle = raw_window_handle::AppKitWindowHandle::empty();
                handle.ns_view = ns_view;
                RawWindowHandle::AppKit(handle)
            }
            ParentWindowHandle::Win32Hwnd(hwnd) => {
                let mut handle = raw_window_handle::Win32WindowHandle::empty();
                handle.hwnd = hwnd;
                RawWindowHandle::Win32(handle)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_primary_mouse_buttons() {
        assert_eq!(
            map_mouse_button(baseview::MouseButton::Left),
            Some(mouse::Button::Left)
        );
        assert_eq!(
            map_mouse_button(baseview::MouseButton::Right),
            Some(mouse::Button::Right)
        );
    }
}
