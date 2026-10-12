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
    pending: VecDeque<InternalEvent>,
    running: bool,
}

impl EngineLoop {
    /// Registers a user frame event.
    pub(crate) fn add_event(&mut self, event: EngineEvent) {
        self.callbacks.push(event);
    }

    /// Returns all registered frame handlers in registration order.
    pub(super) fn callbacks(&mut self) -> &mut [EngineEvent] {
        &mut self.callbacks
    }

    /// Sets the running state of the engine loop.
    pub(super) fn set_running(&mut self, status: bool) {
        self.running = status;
    }

    /// Returns whether the engine loop should keep running.
    pub(super) fn is_running(&self) -> bool {
        self.running
    }

    /// Queues an internal loop event.
    pub(super) fn push_internal(&mut self, event: InternalEvent) {
        self.pending.push_back(event);
    }

    /// Pops the next internal loop event, if any.
    pub(super) fn next_internal(&mut self) -> Option<InternalEvent> {
        self.pending.pop_front()
    }
}

/// Internal loop event, crate-private: not part of the public API.
///
/// The engine installs its own handler for each event.
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
