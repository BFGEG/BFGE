//! Engine facade and orchestrator.

use std::path::Path;
use std::time::{Duration, Instant};

use winit::event_loop::EventLoop;

use crate::core::api::EngineApi;
use crate::core::engine_loop::{EngineEvent, EngineLoop, FrameClock, InternalEvent};
use crate::core::error::EngineError;
use crate::platform::{
    PlatformAdapter, PlatformError, PlatformEvent, WindowDescriptor, WindowSize, new_event_loop,
    validate_window_size,
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
/// registration, window creation and GPU initialization. The main loop calls
/// frame handlers and presents frames until the window closes or an error occurs.
///
/// A session is the interval between [`EngineApi::initialize`] and
/// [`EngineApi::shutdown`]. The engine itself, its event loop and its scene
/// store survive shutdown; the window, the platform adapter and `scene` and
/// `resources` are recreated per session.
pub struct Engine {
    parts: EngineParts,
    event_loop: EngineLoop,
    platform: Option<PlatformAdapter>,
    winit_loop: Option<EventLoop<()>>,
}

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

impl Engine {
    /// Creates the engine; the parts are assembled by `systems::lifecycle`.
    pub fn new() -> Self {
        Self {
            parts: lifecycle::create(),
            event_loop: EngineLoop::default(),
            platform: None,
            winit_loop: None,
        }
    }

    fn initialize_platform(&mut self, platform: PlatformAdapter) -> Result<(), EngineError> {
        lifecycle::initialize(&mut self.parts, platform.window())?;
        self.platform = Some(platform);
        Ok(())
    }

    /// Returns the engine's winit event loop, creating it on first use.
    ///
    /// The loop is created once and reused by every following session; winit
    /// allows a single loop per process (see Wgpu_Backend.md).
    fn acquire_event_loop(&mut self) -> Result<&mut EventLoop<()>, EngineError> {
        if self.winit_loop.is_none() {
            self.winit_loop = Some(new_event_loop()?);
        }
        match &mut self.winit_loop {
            Some(event_loop) => Ok(event_loop),
            None => Err(EngineError::Platform(PlatformError::MissingEventLoop)),
        }
    }

    fn frame_pass(
        &mut self,
        size: WindowSize,
        clock: &mut FrameClock,
        now: &mut impl FnMut() -> Instant,
    ) -> Result<(), EngineError> {
        clock.begin_frame(now());
        for event in self.event_loop.callbacks() {
            (event.frame_started)(&mut self.parts.scene, &clock.event_time(now()))?;
        }
        let frame = self.parts.scene.build_render_frame(size);
        self.parts.renderer.render(&frame, &self.parts.resources)?;
        for event in self.event_loop.callbacks() {
            (event.frame_ended)(&mut self.parts.scene, &clock.event_time(now()))?;
        }
        Ok(())
    }

    fn run_with_events(
        &mut self,
        mut pump: impl FnMut(&mut Self, Duration) -> Result<Vec<PlatformEvent>, EngineError>,
        mut now: impl FnMut() -> Instant,
    ) -> Result<(), EngineError> {
        let size = self
            .platform
            .as_ref()
            .ok_or(EngineError::NotInitialized)?
            .window()
            .size();
        self.event_loop.stop();
        self.event_loop.set_running(true);
        let result = self.drive_loop(size, &mut pump, &mut now);
        self.event_loop.stop();
        result
    }

    fn drive_loop(
        &mut self,
        mut size: WindowSize,
        pump: &mut impl FnMut(&mut Self, Duration) -> Result<Vec<PlatformEvent>, EngineError>,
        now: &mut impl FnMut() -> Instant,
    ) -> Result<(), EngineError> {
        let mut clock = FrameClock::new(now());
        let mut timeout = Duration::ZERO;
        while self.event_loop.is_running() {
            let events = pump(self, timeout)?;
            let iteration_started = now();
            for event in events {
                self.event_loop.push_internal(match event {
                    PlatformEvent::Resized(size) => InternalEvent::WindowResized(size),
                    PlatformEvent::CloseRequested => InternalEvent::WindowClosed,
                });
            }
            self.event_loop.push_internal(InternalEvent::Tick);
            while let Some(event) = self.event_loop.next_internal() {
                match event {
                    InternalEvent::Tick => {
                        if size.width != 0 && size.height != 0 {
                            self.frame_pass(size, &mut clock, now)?;
                        }
                    }
                    InternalEvent::WindowResized(new_size) => {
                        self.parts.renderer.set_window_size(new_size)?;
                        size = new_size;
                    }
                    InternalEvent::WindowClosed => {
                        self.event_loop.stop();
                    }
                }
            }
            timeout = FRAME_INTERVAL.saturating_sub(now().duration_since(iteration_started));
        }
        Ok(())
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineApi for Engine {
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError> {
        if self.platform.is_some() {
            Err(EngineError::AlreadyInitialized)
        } else {
            validate_window_size(window.size)?;
            let event_loop = self.acquire_event_loop()?;
            let platform = PlatformAdapter::new(window, event_loop)?;
            self.initialize_platform(platform)
        }
    }

    fn run(&mut self) -> Result<(), EngineError> {
        self.run_with_events(
            |engine, timeout| {
                let platform = engine
                    .platform
                    .as_ref()
                    .ok_or(EngineError::NotInitialized)?;
                let event_loop = engine
                    .winit_loop
                    .as_mut()
                    .ok_or(EngineError::Platform(PlatformError::MissingEventLoop))?;
                Ok(platform.pump_events(event_loop, Some(timeout)))
            },
            Instant::now,
        )
    }

    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError> {
        self.event_loop.add_event(event);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        if self.platform.is_none() {
            Err(EngineError::NotInitialized)
        } else {
            self.event_loop.stop();
            lifecycle::shutdown(&mut self.parts)?;
            self.platform = None;
            Ok(())
        }
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
mod tests;
