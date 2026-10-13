//! Desktop window and platform adapter owned by the engine.

use std::sync::Arc;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window as NativeWindow, WindowId};

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
    pub(crate) fn native(&self) -> Result<Arc<NativeWindow>, PlatformError> {
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

/// Owns the engine's window and delivers its events (close, resize).
///
/// The client does not create or manage the window; it only supplies a
/// [`WindowDescriptor`]. Creation must run on the application's main thread.
/// The event loop is retained for the frame driver implemented in issue #15.
#[derive(Debug)]
pub struct PlatformAdapter {
    window: Window,
    _event_loop: Option<EventLoop<()>>,
}

impl PlatformAdapter {
    /// Creates the window and the platform adapter.
    pub(crate) fn new(descriptor: WindowDescriptor) -> Result<Self, PlatformError> {
        validate_window_size(descriptor.size)?;
        let event_loop =
            EventLoop::new().map_err(|error| PlatformError::WindowCreation(error.to_string()))?;
        Self::create_window(descriptor, event_loop)
    }

    fn create_window(
        descriptor: WindowDescriptor,
        mut event_loop: EventLoop<()>,
    ) -> Result<Self, PlatformError> {
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
            _event_loop: Some(event_loop),
        })
    }

    /// Returns the engine's window.
    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    /// Resizes the window.
    pub(crate) fn set_window_size(&mut self, size: WindowSize) -> Result<(), PlatformError> {
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
            _event_loop: None,
        }
    }
}

pub(crate) fn validate_window_size(size: WindowSize) -> Result<(), PlatformError> {
    if size.width == 0 || size.height == 0 {
        Err(PlatformError::InvalidWindowSize(size))
    } else {
        Ok(())
    }
}

struct WindowBootstrap {
    descriptor: WindowDescriptor,
    result: Option<Result<Arc<NativeWindow>, PlatformError>>,
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
    const PREVIEW_WINDOW_TITLE: &str = "BFGE cube preview";

    #[test]
    fn zero_width_is_rejected_before_creating_event_loop() {
        assert!(matches!(
            PlatformAdapter::new(WindowDescriptor::new(TEST_WINDOW_TITLE, 0, 600)),
            Err(PlatformError::InvalidWindowSize(_))
        ));
    }

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
    -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "linux")]
        use winit::platform::x11::EventLoopBuilderExtX11;

        use crate::backend::wgpu::backend::WgpuRenderSystem;
        use crate::math::{Mat4, Vec3};
        use crate::render::frame::{RenderCamera, RenderFrame, RenderItem};
        use crate::render::stats::RenderStats;
        use crate::render::system::RenderSystem;
        use crate::resources::library::BuiltinMesh;
        use crate::resources::manager::ResourceManager;

        let event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let mut platform = PlatformAdapter::create_window(
            WindowDescriptor::new(TEST_WINDOW_TITLE, 320, 240),
            event_loop,
        )?;
        let mut backend = WgpuRenderSystem::new();
        let mut resources = ResourceManager::new();
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
        let cube = resources.builtin(BuiltinMesh::Cube);
        let scene_frame = RenderFrame {
            camera: RenderCamera {
                view: crate::math::view(Vec3::new(0.0, 0.0, 5.0), Vec3::NEG_Z),
                projection: crate::math::perspective(60.0_f32.to_radians(), 320.0 / 240.0),
            },
            items: [-0.8, 0.8]
                .into_iter()
                .map(|x| RenderItem {
                    world_transform: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)),
                    mesh: cube,
                })
                .collect(),
        };
        let scene_stats = RenderStats {
            draw_calls: 2,
            triangles: 24,
        };
        assert_eq!(backend.render(&scene_frame, &resources)?, scene_stats);
        backend.set_window_size(WindowSize::new(640, 480))?;
        platform.set_window_size(WindowSize::new(640, 480))?;
        assert_eq!(backend.render(&scene_frame, &resources)?, scene_stats);
        assert!(
            backend
                .set_window_size(WindowSize::new(u32::MAX, 480))
                .is_err()
        );
        backend.render(&frame, &resources)?;
        backend.set_window_size(WindowSize::new(0, 0))?;
        assert_eq!(
            backend.render(&scene_frame, &resources)?,
            RenderStats::default()
        );
        backend.set_window_size(WindowSize::new(640, 480))?;
        assert_eq!(backend.render(&scene_frame, &resources)?, scene_stats);
        backend.shutdown();
        assert!(backend.render(&frame, &resources).is_err());
        backend.initialize(platform.window())?;
        assert_eq!(backend.render(&scene_frame, &resources)?, scene_stats);
        backend.shutdown();
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    #[ignore = "opens a GPU window until it is closed manually"]
    fn native_cube_preview_renders_until_closed() -> Result<(), Box<dyn std::error::Error>> {
        const REAR_X: f32 = 1.5;
        const FRONT_Z: f32 = 1.8;

        #[cfg(target_os = "windows")]
        use winit::platform::windows::EventLoopBuilderExtWindows;
        #[cfg(target_os = "linux")]
        use winit::platform::x11::EventLoopBuilderExtX11;

        use crate::backend::wgpu::backend::WgpuRenderSystem;
        use crate::math::{self, Mat4, Vec3};
        use crate::render::frame::{RenderCamera, RenderFrame, RenderItem};
        use crate::render::system::RenderSystem;
        use crate::resources::library::BuiltinMesh;
        use crate::resources::manager::ResourceManager;

        #[derive(Default)]
        struct PreviewEvents {
            close_requested: bool,
        }

        impl ApplicationHandler for PreviewEvents {
            fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

            fn window_event(
                &mut self,
                _event_loop: &ActiveEventLoop,
                _window_id: WindowId,
                event: WindowEvent,
            ) {
                if let WindowEvent::CloseRequested = event {
                    self.close_requested = true;
                }
            }
        }

        let event_loop = EventLoop::builder().with_any_thread(true).build()?;
        let mut platform = PlatformAdapter::create_window(
            WindowDescriptor::new(PREVIEW_WINDOW_TITLE, 800, 600),
            event_loop,
        )?;
        let mut backend = WgpuRenderSystem::new();
        backend.initialize(platform.window())?;

        let mut resources = ResourceManager::new();
        let cube = resources.builtin(BuiltinMesh::Cube);
        let mut red_cube = resources
            .mesh(cube)
            .expect("builtin cube is registered in the resource manager")
            .clone();
        red_cube.color = [0.9, 0.2, 0.1, 1.0];
        let red_cube = resources.add_mesh(red_cube);
        let rotation = Mat4::from_rotation_y(0.5) * Mat4::from_rotation_x(0.3);
        let mut frame = RenderFrame {
            camera: RenderCamera {
                view: math::view(Vec3::new(0.0, 0.0, 4.5), Vec3::NEG_Z),
                projection: Mat4::IDENTITY,
            },
            items: [
                (0.0, FRONT_Z, red_cube),
                (-REAR_X, 0.0, cube),
                (REAR_X, 0.0, cube),
            ]
            .into_iter()
            .map(|(x, z, mesh)| RenderItem {
                world_transform: Mat4::from_translation(Vec3::new(x, 0.0, z)) * rotation,
                mesh,
            })
            .collect(),
        };
        let mut events = PreviewEvents::default();
        let mut surface_size = platform.window().size();
        let mut resized = false;
        let mut presented = false;
        let mut presented_after_resize = false;
        println!("Resize the cube preview window, then close it to finish the test.");

        loop {
            platform
                ._event_loop
                .as_mut()
                .expect("the native platform owns an event loop")
                .pump_app_events(Some(Duration::from_millis(16)), &mut events);
            if events.close_requested {
                break;
            }

            let actual_size = platform.window().size();
            if actual_size != surface_size {
                backend.set_window_size(actual_size)?;
                surface_size = actual_size;
                resized = true;
            }
            let aspect_ratio = if actual_size.width == 0 || actual_size.height == 0 {
                1.0
            } else {
                actual_size.width as f32 / actual_size.height as f32
            };
            frame.camera.projection = math::perspective(60.0_f32.to_radians(), aspect_ratio);
            let stats = backend.render(&frame, &resources)?;
            if stats.draw_calls != 0 {
                assert_eq!(stats.draw_calls, 3);
                assert_eq!(stats.triangles, 36);
                if !presented || (resized && !presented_after_resize) {
                    println!(
                        "Rendered {}x{}: {} draw calls, {} triangles",
                        actual_size.width, actual_size.height, stats.draw_calls, stats.triangles
                    );
                }
                presented = true;
                if resized {
                    presented_after_resize = true;
                }
            }
        }

        backend.shutdown();
        assert!(presented, "no cube frame was presented");
        assert!(
            presented_after_resize,
            "resize the cube preview window before closing it"
        );
        Ok(())
    }
}
