//! Engine facade and orchestrator.

use std::path::Path;
use std::time::Instant;

use crate::core::api::EngineApi;
use crate::core::engine_loop::{EngineEvent, EngineLoop, EventTime, InternalEvent};
use crate::core::error::EngineError;
use crate::platform::{
    PlatformAdapter, PlatformEvent, WindowDescriptor, WindowSize, validate_window_size,
};
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::systems::lifecycle::{self, EngineParts};
use crate::systems::loading::{self, ModelLoadError};
use crate::systems::persistence::{self, ScenePersistenceError};

/// Engine facade and orchestrator: links the scene, resources, systems,
/// renderer, window and the frame lifecycle.
///
/// The current skeleton supports construction, scene access and frame handler
/// registration, window creation and GPU initialization. The main loop drives
/// frame handlers, frame build, render and present until the window is closed.
pub struct Engine {
    parts: EngineParts,
    event_loop: EngineLoop,
    platform: Option<PlatformAdapter>,
}

impl Engine {
    /// Creates the engine; the parts are assembled by `systems::lifecycle`.
    pub fn new() -> Self {
        Self {
            parts: lifecycle::create(),
            event_loop: EngineLoop::default(),
            platform: None,
        }
    }

    fn initialize_platform(&mut self, platform: PlatformAdapter) -> Result<(), EngineError> {
        lifecycle::initialize(&mut self.parts, platform.window())?;
        self.platform = Some(platform);
        Ok(())
    }

    /// One frame step: builds the frame from the current window size and
    /// presents it through the renderer.
    fn render(&mut self) -> Result<(), EngineError> {
        let platform = self.platform.as_ref().ok_or(EngineError::NotInitialized)?;
        let window_size = platform.window().size();
        let frame = self.parts.scene.build_render_frame(window_size);
        self.parts.renderer.render(&frame, &self.parts.resources)?;
        Ok(())
    }

    /// Runs one pass: user frame handlers around the render step.
    fn frame_pass(&mut self, event_time: &EventTime) -> Result<(), EngineError> {
        process_frame_started_events(&mut self.parts, &mut self.event_loop, event_time)?;
        self.render()?;
        process_frame_ended_events(&mut self.parts, &mut self.event_loop, event_time)?;
        Ok(())
    }
}

fn process_frame_started_events(
    parts: &mut EngineParts,
    event_loop: &mut EngineLoop,
    event_time: &EventTime,
) -> Result<(), EngineError> {
    for event in event_loop.callbacks() {
        let result = (event.frame_started)(&mut parts.scene, event_time);
        if result.is_err() {
            return Err(EngineError::EventError);
        }
    }
    Ok(())
}

fn process_frame_ended_events(
    parts: &mut EngineParts,
    event_loop: &mut EngineLoop,
    event_time: &EventTime,
) -> Result<(), EngineError> {
    for event in event_loop.callbacks() {
        let result = (event.frame_ended)(&mut parts.scene, event_time);
        if result.is_err() {
            return Err(EngineError::EventError);
        }
    }
    Ok(())
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineApi for Engine {
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError> {
        if self.platform.is_some() {
            return Err(EngineError::AlreadyInitialized);
        }
        let platform = PlatformAdapter::new(window)?;
        self.initialize_platform(platform)
    }

    fn run(&mut self) -> Result<(), EngineError> {
        if self.platform.is_none() {
            return Err(EngineError::NotInitialized);
        }

        self.event_loop.set_running(true);
        self.event_loop.push_internal(InternalEvent::Tick);

        let mut last_frame = Instant::now();

        while self.event_loop.is_running() {
            let events = if let Some(platform) = self.platform.as_mut() {
                platform.pump_events()
            } else {
                Vec::new()
            };
            for event in events {
                match event {
                    PlatformEvent::Resized(size) => {
                        self.event_loop
                            .push_internal(InternalEvent::WindowResized(size));
                    }
                    PlatformEvent::CloseRequested => {
                        self.event_loop.push_internal(InternalEvent::WindowClosed);
                    }
                }
            }

            while self.event_loop.is_running() {
                let Some(event) = self.event_loop.next_internal() else {
                    break;
                };
                match event {
                    InternalEvent::Tick => {
                        let now = Instant::now();
                        let delta = now.duration_since(last_frame).as_secs_f32();
                        last_frame = now;
                        let event_time = EventTime {
                            time_since_last_event: delta,
                            time_since_last_frame: delta,
                        };
                        self.frame_pass(&event_time)?;
                    }
                    InternalEvent::WindowResized(size) => {
                        self.parts.renderer.set_window_size(size)?;
                    }
                    InternalEvent::WindowClosed => {
                        self.event_loop.set_running(false);
                        break;
                    }
                }
            }

            if self.event_loop.is_running() {
                self.event_loop.push_internal(InternalEvent::Tick);
            }
        }

        self.event_loop.set_running(false);
        Ok(())
    }

    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError> {
        self.event_loop.add_event(event);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        if self.platform.is_none() {
            return Err(EngineError::NotInitialized);
        }
        lifecycle::shutdown(&mut self.parts)?;
        self.platform = None;
        Ok(())
    }

    fn scene(&mut self) -> &mut dyn SceneApi {
        &mut self.parts.scene
    }

    fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError> {
        loading::load_model(
            path,
            &mut self.parts.resources,
            &mut self.parts.scene,
            parent,
        )
    }

    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError> {
        let platform = self.platform.as_mut().ok_or(EngineError::NotInitialized)?;
        validate_window_size(size)?;
        self.parts.renderer.set_window_size(size)?;
        platform.set_window_size(size)?;
        Ok(())
    }

    fn save_scene(&mut self) -> Result<(), ScenePersistenceError> {
        persistence::save_scene(
            &mut self.parts.store,
            &self.parts.scene,
            &self.parts.resources,
        )
    }

    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError> {
        persistence::restore_scene(
            &mut self.parts.store,
            &mut self.parts.scene,
            &mut self.parts.resources,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::platform::Window;
    use crate::render::frame::RenderFrame;
    use crate::render::renderer::Renderer;
    use crate::render::stats::RenderStats;
    use crate::render::system::{RenderError, RenderSystem};
    use crate::resources::manager::ResourceManager;

    use super::*;

    const BACKEND_FAILURE: &str = "test backend failure";
    const TEST_WINDOW_TITLE: &str = "Engine lifecycle test";
    const TEST_SIZE: WindowSize = WindowSize {
        width: 800,
        height: 600,
    };

    #[derive(Default)]
    struct BackendState {
        initialized: bool,
        fail_initialize: bool,
        fail_resize: bool,
        size: Option<WindowSize>,
        shutdown_count: usize,
    }

    struct TestRenderSystem(Rc<RefCell<BackendState>>);

    impl RenderSystem for TestRenderSystem {
        fn initialize(&mut self, _window: &Window) -> Result<(), RenderError> {
            let mut state = self.0.borrow_mut();
            if state.fail_initialize {
                Err(RenderError::Initialization(BACKEND_FAILURE.to_owned()))
            } else {
                state.initialized = true;
                Ok(())
            }
        }

        fn set_window_size(&mut self, size: WindowSize) -> Result<(), RenderError> {
            let mut state = self.0.borrow_mut();
            if state.fail_resize {
                return Err(RenderError::Render(BACKEND_FAILURE.to_owned()));
            }
            state.size = Some(size);
            Ok(())
        }

        fn render(
            &mut self,
            _frame: &RenderFrame,
            _resources: &ResourceManager,
        ) -> Result<RenderStats, RenderError> {
            Ok(RenderStats::default())
        }

        fn shutdown(&mut self) {
            self.0.borrow_mut().shutdown_count += 1;
        }
    }

    fn engine_with_backend() -> (Engine, Rc<RefCell<BackendState>>) {
        let state = Rc::new(RefCell::new(BackendState::default()));
        let mut engine = Engine::new();
        engine.parts.renderer = Renderer::new(Box::new(TestRenderSystem(Rc::clone(&state))));
        (engine, state)
    }

    #[test]
    fn initialization_delegates_to_backend_before_committing_platform() {
        let (mut engine, state) = engine_with_backend();

        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");

        assert!(state.borrow().initialized);
        assert!(engine.platform.is_some());
    }

    #[test]
    fn backend_initialization_failure_preserves_uninitialized_state_and_cause() {
        let (mut engine, state) = engine_with_backend();
        state.borrow_mut().fail_initialize = true;

        let result = engine.initialize_platform(PlatformAdapter::for_test(TEST_SIZE));

        assert!(matches!(result,
            Err(EngineError::Lifecycle(lifecycle::LifecycleError::Render(
                RenderError::Initialization(message)
            ))) if message == BACKEND_FAILURE
        ));
        assert!(engine.platform.is_none());

        state.borrow_mut().fail_initialize = false;
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("a failed initialization leaves the engine available for retry");
    }

    #[test]
    fn initialized_engine_rejects_reinitialization() {
        let (mut engine, _) = engine_with_backend();
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");

        assert!(matches!(
            engine.initialize(WindowDescriptor::new(TEST_WINDOW_TITLE, 800, 600)),
            Err(EngineError::AlreadyInitialized)
        ));
    }

    #[test]
    fn resize_delegates_requested_size_to_backend() {
        let (mut engine, state) = engine_with_backend();
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");
        let size = WindowSize::new(1280, 720);

        engine
            .set_window_size(size)
            .expect("the test platform and backend accept resizing");

        assert_eq!(state.borrow().size, Some(size));
    }

    #[test]
    fn resize_preserves_backend_error_cause() {
        let (mut engine, state) = engine_with_backend();
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");
        state.borrow_mut().fail_resize = true;

        assert!(matches!(engine.set_window_size(WindowSize::new(1280, 720)),
            Err(EngineError::Render(RenderError::Render(message))) if message == BACKEND_FAILURE
        ));
        assert_eq!(
            engine
                .platform
                .as_ref()
                .map(|platform| platform.window().size()),
            Some(TEST_SIZE)
        );
    }

    #[test]
    fn invalid_resize_does_not_change_backend_or_window() {
        let (mut engine, state) = engine_with_backend();
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");

        assert!(matches!(
            engine.set_window_size(WindowSize::new(0, 600)),
            Err(EngineError::Platform(
                crate::platform::PlatformError::InvalidWindowSize(_)
            ))
        ));
        assert_eq!(state.borrow().size, None);
        assert_eq!(
            engine
                .platform
                .as_ref()
                .map(|platform| platform.window().size()),
            Some(TEST_SIZE)
        );
    }

    #[test]
    fn shutdown_releases_platform_and_calls_backend_once() {
        let (mut engine, state) = engine_with_backend();
        engine
            .initialize_platform(PlatformAdapter::for_test(TEST_SIZE))
            .expect("the test backend accepts initialization");

        engine
            .shutdown()
            .expect("the test backend supports shutdown");

        assert!(engine.platform.is_none());
        assert!(matches!(
            engine.shutdown(),
            Err(EngineError::NotInitialized)
        ));
        assert_eq!(state.borrow().shutdown_count, 1);
    }

    #[test]
    fn event_registration_retains_handlers_without_calling_them() {
        let mut engine = Engine::new();
        let calls = Rc::new(RefCell::new(Vec::new()));

        for index in 0..2 {
            let started = Rc::clone(&calls);
            let ended = Rc::clone(&calls);
            engine
                .add_event(EngineEvent {
                    frame_started: Box::new(move |_, _| {
                        started.borrow_mut().push((index, true));
                        Ok(())
                    }),
                    frame_ended: Box::new(move |_, _| {
                        ended.borrow_mut().push((index, false));
                        Ok(())
                    }),
                })
                .expect("frame handlers can be registered before initialization");
        }

        assert!(calls.borrow().is_empty());
        let time = crate::core::engine_loop::EventTime {
            time_since_last_event: 0.0,
            time_since_last_frame: 0.0,
        };
        for event in engine.event_loop.callbacks() {
            (event.frame_started)(&mut engine.parts.scene, &time)
                .expect("the registered test handler succeeds");
            (event.frame_ended)(&mut engine.parts.scene, &time)
                .expect("the registered test handler succeeds");
        }
        assert_eq!(
            *calls.borrow(),
            [(0, true), (0, false), (1, true), (1, false)]
        );
    }
}
