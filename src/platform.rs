//! Desktop window and platform adapter owned by the engine.

use std::sync::Arc;
use std::time::{Duration, Instant};

use std::collections::HashSet;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::platform::pump_events::EventLoopExtPumpEvents;
#[cfg(test)]
use winit::window::Fullscreen;
use winit::window::{Window as NativeWindow, WindowId};

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

/// Module-wide `Result` with [`PlatformError`] as the error type.
type Result<T> = core::result::Result<T, PlatformError>;

const WINDOW_NOT_RESUMED: &str = "the window event loop did not resume during initialization";
const WINDOW_BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(test)]
const TEST_WINDOW_UNAVAILABLE: &str = "the test window has no native window handle";

/// Window size in physical pixels.
///
/// A zero dimension can represent a minimized window. Explicit sizes passed to
/// engine initialization and resizing must have nonzero dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Input collected from window events before the current frame.
///
/// The engine updates the state from window events before frame handlers run
/// and resets mouse motion deltas after the end-of-frame handlers. Pressed keys
/// and buttons stay pressed until the matching release event arrives or the
/// window loses focus.
///
/// Read the state inside frame handlers through
/// [`crate::FrameContext::input`].
#[derive(Debug, Default)]
pub struct InputState {
    focused: bool,
    pressed_key: HashSet<KeyCode>,
    pressed_mouse: HashSet<MouseButton>,
    cursor_pos: PhysicalPosition<f64>,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
}

impl InputState {
    pub(crate) fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => self.set_key(*code, *state),
            WindowEvent::MouseInput { state, button, .. } => self.set_mouse_button(*button, *state),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = *position;
            }
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                self.end_frame();
                if !focused {
                    self.pressed_key.clear();
                    self.pressed_mouse.clear();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn set_key(&mut self, code: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.pressed_key.insert(code);
            }
            ElementState::Released => {
                self.pressed_key.remove(&code);
            }
        }
    }

    pub(crate) fn set_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.pressed_mouse.insert(button);
            }
            ElementState::Released => {
                self.pressed_mouse.remove(&button);
            }
        }
    }

    pub(crate) fn handle_device_state(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && self.focused
        {
            self.mouse_delta_x += *dx;
            self.mouse_delta_y += *dy;
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
    }

    /// Returns whether the engine window has keyboard focus.
    ///
    /// A default input state is unfocused. Window creation initializes this
    /// value from the native window; subsequent focus events update it before
    /// frame callbacks. The application can use it to pause its simulation.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns whether `key` is currently held down.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_key.contains(&key)
    }

    /// Returns whether mouse `button` is currently held down.
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse.contains(&button)
    }

    /// Returns the cursor position in physical pixels.
    pub fn cursor_position(&self) -> (f64, f64) {
        (self.cursor_pos.x, self.cursor_pos.y)
    }

    /// Returns the mouse motion accumulated since the previous frame.
    ///
    /// Focus changes clear accumulated motion; motion while unfocused is ignored.
    pub fn mouse_delta(&self) -> (f64, f64) {
        (self.mouse_delta_x, self.mouse_delta_y)
    }
}

impl WindowSize {
    /// Creates a window size in physical pixels.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Title and initial physical dimensions for the engine window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowDescriptor {
    /// Window title.
    pub title: String,
    /// Initial window size.
    pub size: WindowSize,
}

impl WindowDescriptor {
    /// Creates a window descriptor from a title and physical dimensions.
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        Self {
            title: title.to_owned(),
            size: WindowSize::new(width, height),
        }
    }
}

/// The engine's window.
///
/// Owned by [`PlatformAdapter`]; the backend shares the native window's
/// lifetime when creating a render surface. Native handles are crate-private.
#[derive(Debug)]
pub struct Window {
    inner: WindowInner,
}

#[derive(Debug)]
enum WindowInner {
    Native(Arc<NativeWindow>),
    #[cfg(test)]
    Test(WindowSize),
}

impl Window {
    pub(crate) fn native(&self) -> Result<Arc<NativeWindow>> {
        match &self.inner {
            WindowInner::Native(window) => Ok(Arc::clone(window)),
            #[cfg(test)]
            WindowInner::Test(_) => Err(PlatformError::WindowCreation(
                TEST_WINDOW_UNAVAILABLE.to_owned(),
            )),
        }
    }

    pub(crate) fn size(&self) -> WindowSize {
        match &self.inner {
            WindowInner::Native(window) => {
                let size = window.inner_size();
                WindowSize::new(size.width, size.height)
            }
            #[cfg(test)]
            WindowInner::Test(size) => *size,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn hide_cursor(&self) -> Result<()> {
        match &self.inner {
            WindowInner::Native(window) => {
                window.set_cursor_visible(false);
                Ok(())
            }
            #[cfg(test)]
            WindowInner::Test(_) => Err(PlatformError::WindowCreation(
                TEST_WINDOW_UNAVAILABLE.to_owned(),
            )),
        }
    }

    pub(crate) fn show_cursor(&self) -> Result<()> {
        match &self.inner {
            WindowInner::Native(window) => {
                window.set_cursor_visible(true);
                Ok(())
            }
            #[cfg(test)]
            WindowInner::Test(_) => Err(PlatformError::WindowCreation(
                TEST_WINDOW_UNAVAILABLE.to_owned(),
            )),
        }
    }
}

/// Owns the engine's window active for one session.
///
/// The client does not create or manage the window; it only supplies a
/// [`WindowDescriptor`]. Creation must run on the application's main thread.
/// The event loop is owned by the engine and reused across sessions; the
/// window itself lives only for the current session (see issue #15).
#[derive(Debug)]
pub struct PlatformAdapter {
    window: Window,
    input_state: InputState,
}

impl PlatformAdapter {
    /// Creates the window inside the given event loop.
    pub(crate) fn new(
        descriptor: WindowDescriptor,
        event_loop: &mut EventLoop<()>,
    ) -> Result<Self> {
        validate_window_size(descriptor.size)?;
        Self::create_window(descriptor, event_loop)
    }

    fn create_window(descriptor: WindowDescriptor, event_loop: &mut EventLoop<()>) -> Result<Self> {
        let mut bootstrap = WindowBootstrap {
            descriptor,
            result: None,
        };
        let deadline = Instant::now() + WINDOW_BOOTSTRAP_TIMEOUT;
        while bootstrap.result.is_none() && Instant::now() < deadline {
            event_loop.pump_app_events(
                Some(deadline.saturating_duration_since(Instant::now())),
                &mut bootstrap,
            );
        }
        let native = bootstrap
            .result
            .ok_or_else(|| PlatformError::WindowCreation(WINDOW_NOT_RESUMED.to_owned()))??;
        let focused = native.has_focus();
        Ok(Self {
            window: Window {
                inner: WindowInner::Native(native),
            },
            input_state: InputState {
                focused,
                ..InputState::default()
            },
        })
    }

    /// Returns the engine's window.
    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    /// Resizes the window.
    pub(crate) fn set_window_size(&mut self, size: WindowSize) -> Result<()> {
        validate_window_size(size)?;
        match &mut self.window.inner {
            WindowInner::Native(window) => {
                let _actual_size =
                    window.request_inner_size(PhysicalSize::new(size.width, size.height));
            }
            #[cfg(test)]
            WindowInner::Test(current) => *current = size,
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn for_test(size: WindowSize) -> Self {
        Self {
            window: Window {
                inner: WindowInner::Test(size),
            },
            input_state: Default::default(),
        }
    }

    /// Returns the input collected by the last [`Self::pump_events`] call.
    pub(crate) fn input(&self) -> &InputState {
        &self.input_state
    }

    #[cfg(test)]
    pub(crate) fn input_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }

    /// Drops the per-frame mouse motion accumulated for the current frame.
    pub(crate) fn end_input_frame(&mut self) {
        self.input_state.end_frame();
    }

    pub(crate) fn pump_events(
        &mut self,
        event_loop: &mut EventLoop<()>,
        timeout: Option<Duration>,
    ) -> Vec<PlatformEvent> {
        let window_id = match &self.window.inner {
            WindowInner::Native(window) => Some(window.id()),
            #[cfg(test)]
            WindowInner::Test(_) => None,
        };
        let mut recorder = EventRecorder {
            window_id,
            events: Vec::new(),
            input_state: &mut self.input_state,
        };
        event_loop.pump_app_events(timeout, &mut recorder);
        recorder.events
    }

    /// Toggles borderless fullscreen on the current monitor.
    ///
    /// Test-only: lets an interactive smoke test fill the screen. Uses a
    /// borderless mode (no video-mode switch), which is the least intrusive and
    /// works across X11/Wayland/Windows.
    #[cfg(test)]
    pub(crate) fn set_fullscreen(&self, fullscreen: bool) {
        match &self.window.inner {
            WindowInner::Native(window) => {
                let mode = fullscreen.then(|| Fullscreen::Borderless(None));
                window.set_fullscreen(mode);
            }
            #[cfg(test)]
            WindowInner::Test(_) => {}
        }
    }
}

/// A window event delivered to the engine loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformEvent {
    /// The window's close button was pressed (or close was requested).
    CloseRequested,
    /// The window was resized to the given size.
    Resized(WindowSize),
}

struct EventRecorder<'a> {
    window_id: Option<WindowId>,
    events: Vec<PlatformEvent>,
    input_state: &'a mut InputState,
}

impl<'a> EventRecorder<'a> {
    fn record(&mut self, window_id: WindowId, window_event: WindowEvent) {
        if self.window_id == Some(window_id) {
            self.input_state.handle_window_event(&window_event);
            match window_event {
                WindowEvent::CloseRequested => self.events.push(PlatformEvent::CloseRequested),
                WindowEvent::Resized(size) => self.events.push(PlatformEvent::Resized(
                    WindowSize::new(size.width, size.height),
                )),
                _ => {}
            }
        }
    }
}

impl ApplicationHandler for EventRecorder<'_> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.record(window_id, event);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.input_state.handle_device_state(&event);
    }
}

/// Creates the native event loop for the engine.
///
/// The event loop is created once and reused for every following window session.
/// See `docs/graphics.md` for the window and backend lifecycle.
pub(crate) fn new_event_loop() -> Result<EventLoop<()>> {
    EventLoop::new().map_err(|error| PlatformError::EventLoopCreation(error.to_string()))
}

pub(crate) fn validate_window_size(size: WindowSize) -> Result<()> {
    if size.width == 0 || size.height == 0 {
        Err(PlatformError::InvalidWindowSize(size))
    } else {
        Ok(())
    }
}

struct WindowBootstrap {
    descriptor: WindowDescriptor,
    result: Option<Result<Arc<NativeWindow>>>,
}

impl WindowBootstrap {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.result.is_none() {
            let attributes = NativeWindow::default_attributes()
                .with_title(&self.descriptor.title)
                .with_inner_size(PhysicalSize::new(
                    self.descriptor.size.width,
                    self.descriptor.size.height,
                ));
            self.result = Some(
                event_loop
                    .create_window(attributes)
                    .map(Arc::new)
                    .map_err(|error| PlatformError::WindowCreation(error.to_string())),
            );
        }
    }
}

impl ApplicationHandler for WindowBootstrap {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }
}

/// Platform (window) error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// An explicit window size must have nonzero width and height.
    #[error("invalid window size: {0:?}; width and height must be nonzero")]
    InvalidWindowSize(WindowSize),
    /// Failed to create the event loop.
    #[error("failed to create the event loop: {0}")]
    EventLoopCreation(String),
    /// No event loop is available for window creation.
    #[error("no event loop available for window creation")]
    MissingEventLoop,
    /// Failed to create the window.
    #[error("failed to create the window: {0}")]
    WindowCreation(String),
    /// Failed to resize the window.
    #[error("failed to resize the window: {0}")]
    WindowResize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_WINDOW_TITLE: &str = "BFGE wgpu smoke test";

    #[test]
    fn zero_height_resize_preserves_current_window_size() {
        let size = WindowSize::new(800, 600);
        let mut platform = PlatformAdapter::for_test(size);

        assert!(matches!(
            platform.set_window_size(WindowSize::new(800, 0)),
            Err(PlatformError::InvalidWindowSize(_))
        ));
        assert_eq!(platform.window().size(), size);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a desktop display and a presentation-capable GPU; opens a window"]
    fn native_surface_initializes_resizes_presents_and_reinitializes()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "linux")]
        use winit::platform::x11::EventLoopBuilderExtX11;

        use crate::backend::wgpu::backend::WgpuRenderSystem;
        use crate::math::Mat4;
        use crate::render::frame::{RenderCamera, RenderFrame};
        use crate::render::stats::RenderStats;
        use crate::render::system::RenderSystem;
        use crate::resources::manager::ResourceManager;
        use crate::scene::lighting::SceneLighting;

        let mut event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let mut platform = PlatformAdapter::create_window(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            &mut event_loop,
        )?;
        let mut backend = WgpuRenderSystem::new();
        let resources = ResourceManager::new();
        let frame = RenderFrame {
            camera: RenderCamera {
                view: Mat4::IDENTITY,
                projection: Mat4::IDENTITY,
            },
            lighting: SceneLighting::default(),
            items: Vec::new(),
        };

        backend.initialize(platform.window())?;
        assert!(backend.initialize(platform.window()).is_err());
        assert_eq!(backend.render(&frame, &resources)?, RenderStats::default());
        backend.set_window_size(WindowSize::new(640, 480))?;
        platform.set_window_size(WindowSize::new(640, 480))?;
        backend.render(&frame, &resources)?;
        assert!(
            backend
                .set_window_size(WindowSize::new(u32::MAX, 480))
                .is_err()
        );
        backend.render(&frame, &resources)?;
        backend.set_window_size(WindowSize::new(0, 0))?;
        backend.render(&frame, &resources)?;
        backend.set_window_size(WindowSize::new(640, 480))?;
        backend.render(&frame, &resources)?;
        backend.shutdown();
        assert!(backend.render(&frame, &resources).is_err());
        backend.initialize(platform.window())?;
        backend.render(&frame, &resources)?;
        backend.shutdown();
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a desktop display; opens two windows"]
    fn native_window_can_be_created_twice_on_one_event_loop()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "linux")]
        use winit::platform::x11::EventLoopBuilderExtX11;

        let mut event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let first = PlatformAdapter::new(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            &mut event_loop,
        )?;
        assert_eq!(first.window().size(), WindowSize::new(320, 240));
        drop(first);
        let second = PlatformAdapter::new(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            &mut event_loop,
        )?;
        assert_eq!(second.window().size(), WindowSize::new(320, 240));
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "requires a desktop display and a presentation-capable GPU; opens a window and spins until closed"]
    fn native_surface_draws_indexed_meshes() -> core::result::Result<(), Box<dyn std::error::Error>>
    {
        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "linux")]
        use winit::platform::x11::EventLoopBuilderExtX11;

        use std::f32::consts::TAU;

        use crate::backend::wgpu::backend::WgpuRenderSystem;
        use crate::math::{self, Mat4, Quat, Vec3};
        use crate::render::frame::{RenderCamera, RenderFrame, RenderItem};
        use crate::render::system::RenderSystem;
        use crate::resources::library::BuiltinMesh;
        use crate::resources::manager::ResourceManager;
        use crate::scene::lighting::SceneLighting;

        let mut event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let mut platform = PlatformAdapter::create_window(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            &mut event_loop,
        )?;
        platform.set_fullscreen(true);
        let mut backend = WgpuRenderSystem::new();

        let mut resources = ResourceManager::new();
        let cube = resources.builtin(BuiltinMesh::Cube);

        let view = math::view(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 0.0);
        let items = [(-1.5_f32, Vec3::Y), (1.5, Vec3::X)];

        backend.initialize(platform.window())?;

        let mut size = platform.window().size();
        let mut angle = 0.0_f32;
        let mut running = true;
        while running {
            for event in platform.pump_events(&mut event_loop, Some(Duration::from_millis(16))) {
                if let PlatformEvent::CloseRequested = event {
                    running = false;
                }
            }
            if !running {
                break;
            }

            // Poll the window's real size each frame: X11 may not deliver a
            // Resized event for the fullscreen transition, or may deliver the
            // pre-fullscreen size, which would leave the surface too small.
            let current = platform.window().size();
            if current != size && current.width != 0 && current.height != 0 {
                size = current;
                backend.set_window_size(current)?;
            }

            let aspect = if size.height == 0 {
                1.0
            } else {
                size.width as f32 / size.height as f32
            };
            let camera = RenderCamera {
                view,
                projection: math::perspective(60.0_f32.to_radians(), aspect),
            };

            angle = (angle + 0.02) % TAU;
            let frame = RenderFrame {
                camera,
                lighting: SceneLighting::default(),
                items: items
                    .iter()
                    .map(|(offset, axis)| RenderItem {
                        color_override: None,
                        world_transform: Mat4::from_translation(Vec3::new(*offset, 0.0, 0.0))
                            * Mat4::from_quat(Quat::from_axis_angle(*axis, angle)),
                        mesh: cube,
                    })
                    .collect(),
            };
            let stats = backend.render(&frame, &resources)?;
            assert_eq!(stats.draw_calls, 2);
            assert_eq!(stats.triangles, 24);
        }

        backend.shutdown();
        Ok(())
    }

    #[test]
    fn rendering_an_unknown_mesh_is_reported_not_panicked() {
        // A handle that was never registered must surface as an error rather
        // than reaching GPU validation. Uses a real device when available and
        // is skipped otherwise.
        use crate::backend::wgpu::buffer::MeshCache;
        use crate::resources::handle::MeshHandle;
        use crate::resources::manager::ResourceManager;

        let device = match test_device() {
            Some(device) => device,
            None => return,
        };
        let resources = ResourceManager::new();
        let mut cache = MeshCache::default();

        let result = cache.get_mesh(&device, &resources, MeshHandle::new(7));

        assert!(matches!(
            result,
            Err(crate::render::system::RenderError::Render(message)) if message.contains("unknown mesh handle")
        ));
    }

    /// Requests a headless adapter for CPU-side buffer tests.
    ///
    /// Returns `None` when no adapter is available (typical in CI), so tests
    /// that need one stay green without a GPU.
    fn test_device() -> Option<wgpu::Device> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some(device)
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;

    #[test]
    fn recorder_preserves_resize_and_close_order() {
        let window_id = WindowId::from(1);
        let mut input_state = InputState::default();
        let mut recorder = EventRecorder {
            window_id: Some(window_id),
            events: Vec::new(),
            input_state: &mut input_state,
        };
        recorder.record(window_id, WindowEvent::Resized(PhysicalSize::new(0, 600)));
        recorder.record(window_id, WindowEvent::CloseRequested);
        assert_eq!(
            recorder.events,
            [
                PlatformEvent::Resized(WindowSize::new(0, 600)),
                PlatformEvent::CloseRequested
            ]
        );
    }

    #[test]
    fn recorder_ignores_events_from_other_sessions() {
        let mut input_state = InputState::default();
        let mut recorder = EventRecorder {
            window_id: Some(WindowId::from(1)),
            events: Vec::new(),
            input_state: &mut input_state,
        };
        recorder.record(WindowId::from(2), WindowEvent::CloseRequested);
        recorder.record(
            WindowId::from(2),
            WindowEvent::Resized(PhysicalSize::new(800, 600)),
        );
        assert!(recorder.events.is_empty());
    }

    #[test]
    fn recorder_ignores_input_and_redraw_events() {
        let window_id = WindowId::from(1);
        let mut input_state = InputState::default();
        let mut recorder = EventRecorder {
            window_id: Some(window_id),
            events: Vec::new(),
            input_state: &mut input_state,
        };
        recorder.record(window_id, WindowEvent::Focused(true));
        recorder.record(window_id, WindowEvent::RedrawRequested);
        assert!(recorder.events.is_empty());
    }

    fn mouse_input(state: ElementState, button: MouseButton) -> WindowEvent {
        WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state,
            button,
        }
    }

    #[test]
    fn input_state_tracks_key_press_and_release() {
        let mut input_state = InputState::default();
        input_state.set_key(KeyCode::KeyW, ElementState::Pressed);
        assert!(input_state.is_key_pressed(KeyCode::KeyW));
        assert!(!input_state.is_key_pressed(KeyCode::KeyA));
        input_state.set_key(KeyCode::KeyW, ElementState::Released);
        assert!(!input_state.is_key_pressed(KeyCode::KeyW));
    }

    #[test]
    fn input_state_tracks_mouse_button_press_and_release() {
        let mut input_state = InputState::default();
        input_state.handle_window_event(&mouse_input(ElementState::Pressed, MouseButton::Left));
        assert!(input_state.is_mouse_pressed(MouseButton::Left));
        assert!(!input_state.is_mouse_pressed(MouseButton::Right));
        input_state.handle_window_event(&mouse_input(ElementState::Released, MouseButton::Left));
        assert!(!input_state.is_mouse_pressed(MouseButton::Left));
    }

    #[test]
    fn input_state_clears_pressed_input_on_focus_loss() {
        let mut input_state = InputState::default();
        input_state.set_key(KeyCode::KeyW, ElementState::Pressed);
        input_state.handle_window_event(&mouse_input(ElementState::Pressed, MouseButton::Left));
        input_state.handle_window_event(&WindowEvent::Focused(false));
        assert!(!input_state.is_key_pressed(KeyCode::KeyW));
        assert!(!input_state.is_mouse_pressed(MouseButton::Left));
    }

    #[test]
    fn input_state_accumulates_mouse_motion_and_resets_it_per_frame() {
        let mut input_state = InputState::default();
        input_state.handle_window_event(&WindowEvent::Focused(true));
        input_state.handle_device_state(&DeviceEvent::MouseMotion { delta: (2.0, -3.0) });
        input_state.handle_device_state(&DeviceEvent::MouseMotion { delta: (1.0, 1.0) });
        assert_eq!(input_state.mouse_delta(), (3.0, -2.0));
        input_state.end_frame();
        assert_eq!(input_state.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn input_state_reports_focus_and_keeps_it_across_frames() {
        let mut input_state = InputState::default();
        assert!(!input_state.is_focused());
        input_state.handle_window_event(&WindowEvent::Focused(true));
        input_state.end_frame();
        assert!(input_state.is_focused());
        input_state.handle_window_event(&WindowEvent::Focused(false));
        assert!(!input_state.is_focused());
        input_state.handle_window_event(&WindowEvent::Focused(true));
        assert!(input_state.is_focused());
    }

    #[test]
    fn focus_loss_discards_mouse_motion_until_focus_returns() {
        let mut input_state = InputState::default();
        input_state.handle_window_event(&WindowEvent::Focused(true));
        input_state.handle_device_state(&DeviceEvent::MouseMotion { delta: (3.0, 4.0) });
        input_state.handle_window_event(&WindowEvent::Focused(false));
        assert_eq!(input_state.mouse_delta(), (0.0, 0.0));
        input_state.handle_device_state(&DeviceEvent::MouseMotion { delta: (8.0, 9.0) });
        input_state.handle_window_event(&WindowEvent::Focused(true));
        assert_eq!(input_state.mouse_delta(), (0.0, 0.0));
        input_state.handle_device_state(&DeviceEvent::MouseMotion { delta: (1.0, 2.0) });
        assert_eq!(input_state.mouse_delta(), (1.0, 2.0));
    }

    #[test]
    fn focus_from_another_window_does_not_change_input() {
        let mut input_state = InputState::default();
        let mut recorder = EventRecorder {
            window_id: Some(WindowId::from(1)),
            events: Vec::new(),
            input_state: &mut input_state,
        };
        recorder.record(WindowId::from(2), WindowEvent::Focused(true));
        assert!(!recorder.input_state.is_focused());
        recorder.record(WindowId::from(1), WindowEvent::Focused(true));
        assert!(recorder.input_state.is_focused());
        recorder.record(WindowId::from(2), WindowEvent::Focused(false));
        assert!(recorder.input_state.is_focused());
    }

    #[test]
    fn input_state_tracks_cursor_position() {
        let mut input_state = InputState::default();
        input_state.handle_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(12.0, 34.0),
        });
        assert_eq!(input_state.cursor_position(), (12.0, 34.0));
    }
}
