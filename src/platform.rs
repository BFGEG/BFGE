//! Desktop window and platform adapter owned by the engine.

use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
#[cfg(test)]
use winit::window::Fullscreen;
use winit::window::{Window as NativeWindow, WindowId};

/// Module-wide `Result` with [`PlatformError`] as the error type.
type Result<T> = core::result::Result<T, PlatformError>;

const WINDOW_NOT_RESUMED: &str = "the window event loop did not resume during initialization";
#[cfg(test)]
const TEST_WINDOW_UNAVAILABLE: &str = "the test window has no native window handle";

/// Window size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl WindowSize {
    /// Creates a window size.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Parameters of the window created by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowDescriptor {
    /// Window title.
    pub title: String,
    /// Initial window size.
    pub size: WindowSize,
}

impl WindowDescriptor {
    /// Creates a window descriptor.
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
        event_loop.pump_app_events(Some(Duration::ZERO), &mut bootstrap);
        let native = bootstrap
            .result
            .ok_or_else(|| PlatformError::WindowCreation(WINDOW_NOT_RESUMED.to_owned()))??;
        Ok(Self {
            window: Window {
                inner: WindowInner::Native(native),
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
        }
    }

    /// Pumps pending window events, returning the ones the caller cares about.
    ///
    /// Test-only: drives a manual render loop so an interactive smoke test can
    /// keep drawing until the window is closed. Returns an empty list when the
    /// adapter was built without a real event loop (e.g. [`Self::for_test`]).
    #[cfg(test)]
    pub(crate) fn pump_once(&mut self, timeout: Option<Duration>) -> Vec<PlatformEvent> {
        let Some(event_loop) = self._event_loop.as_mut() else {
            return Vec::new();
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut recorder = EventRecorder {
            events: Rc::clone(&events),
        };
        event_loop.pump_app_events(timeout, &mut recorder);
        Rc::try_unwrap(events)
            .map(RefCell::into_inner)
            .unwrap_or_default()
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

/// A platform event surfaced by [`PlatformAdapter::pump_once`] for tests.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformEvent {
    /// The window's close button was pressed (or close was requested).
    CloseRequested,
    /// The window was resized to the given size.
    Resized(WindowSize),
}

/// Collects the window events a manual test loop reacts to.
#[cfg(test)]
struct EventRecorder {
    events: Rc<RefCell<Vec<PlatformEvent>>>,
}

#[cfg(test)]
impl ApplicationHandler for EventRecorder {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.events.borrow_mut().push(PlatformEvent::CloseRequested);
            }
            WindowEvent::Resized(size) => {
                self.events
                    .borrow_mut()
                    .push(PlatformEvent::Resized(WindowSize::new(
                        size.width,
                        size.height,
                    )));
            }
            _ => {}
        }
    }
}

/// Creates a new event loop for the engine.
///
/// The event loop is created once and reused for every window in following
/// sessions; winit allows a single loop per process (see Wgpu_Backend.md).
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

impl ApplicationHandler for WindowBootstrap {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
    #[ignore = "requires a desktop display and a presentation-capable GPU; opens a window and spins until closed"]
    fn native_surface_draws_indexed_meshes() -> Result<(), Box<dyn std::error::Error>> {
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

        let event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let mut platform = PlatformAdapter::create_window(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            event_loop,
        )?;
        platform.set_fullscreen(true);
        let mut backend = WgpuRenderSystem::new();

        let mut resources = ResourceManager::new();
        let cube = resources.builtin(BuiltinMesh::Cube);

        let view = math::view(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let items = [(-1.5_f32, Vec3::Y), (1.5, Vec3::X)];

        backend.initialize(platform.window())?;

        let mut size = platform.window().size();
        let mut angle = 0.0_f32;
        let mut running = true;
        while running {
            for event in platform.pump_once(Some(Duration::from_millis(16))) {
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
                items: items
                    .iter()
                    .map(|(offset, axis)| RenderItem {
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
        use crate::backend::wgpu::buffer::MeshBufferCache;
        use crate::resources::handle::MeshHandle;
        use crate::resources::manager::ResourceManager;

        let device = match test_device() {
            Some(device) => device,
            None => return,
        };
        let resources = ResourceManager::new();
        let mut cache = MeshBufferCache::default();

        let result = cache.get(&device, &resources, MeshHandle::new(7));

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
