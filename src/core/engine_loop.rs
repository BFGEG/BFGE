//! The engine's main loop and frame handlers.

use std::collections::VecDeque;

use crate::platform::WindowSize;
use crate::scene::api::SceneApi;
use crate::scene::error::SceneError;

/// The engine's main loop.
///
/// Owns the order of frame steps: user frame handlers, state update, frame
/// build, render and present.
#[derive(Default)]
pub struct EngineLoop {
    callbacks: Vec<EngineEvent>,
    #[allow(dead_code)] // consumed by [CORE]-1 (milestone 2)
    pending: VecDeque<InternalEvent>,
    #[allow(dead_code)] // consumed by [CORE]-1 (milestone 2)
    running: bool,
}

impl EngineLoop {
    /// Registers a user frame event.
    pub(crate) fn add_event(&mut self, event: EngineEvent) {
        self.callbacks.push(event);
    }
}

/// Internal loop event, crate-private: not part of the public API.
///
/// The engine installs its own handler for each event.
#[allow(dead_code)] // produced by [CORE]-1 and [platform] (milestone 2)
#[derive(Debug)]
pub(crate) enum InternalEvent {
    /// Next frame step.
    Tick,
    /// The window was resized: recreate the surface and update the frame size.
    WindowResized(WindowSize),
    /// The window was closed: stop and move to `ShuttingDown`.
    WindowClosed,
}

/// Times passed to user frame handlers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventTime {
    /// Time since the last event, in seconds.
    pub time_since_last_event: f32,
    /// Time since the last frame, in seconds.
    pub time_since_last_frame: f32,
}

/// Signature of a user frame handler.
pub type FrameHandler = Box<dyn FnMut(&mut dyn SceneApi, &EventTime) -> Result<(), SceneError>>;

/// User frame handlers, called by the loop on every pass in registration
/// order, with access to the scene and frame times.
pub struct EngineEvent {
    /// Called before the update and frame build.
    pub frame_started: FrameHandler,
    /// Called after the frame is presented.
    pub frame_ended: FrameHandler,
}
