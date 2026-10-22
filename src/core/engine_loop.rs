//! The engine's main loop and frame handlers.

use std::collections::VecDeque;
use std::time::Instant;

use crate::platform::WindowSize;
use crate::scene::api::SceneApi;
use crate::scene::error::SceneError;

/// The engine's main loop.
///
/// Stores frame handlers, pending internal events and the running state.
/// The engine drives frame build, render and present in registration order.
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

    pub(super) fn stop(&mut self) {
        self.running = false;
        self.pending.clear();
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
    /// The window was closed: stop rendering until the client shuts down.
    WindowClosed,
}

pub(super) struct FrameClock {
    last_event: Instant,
    last_frame: Instant,
    frame_delta: f32,
}

impl FrameClock {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            last_event: now,
            last_frame: now,
            frame_delta: 0.0,
        }
    }

    pub(super) fn begin_frame(&mut self, now: Instant) {
        self.frame_delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
    }

    pub(super) fn event_time(&mut self, now: Instant) -> EventTime {
        let time = EventTime {
            time_since_last_event: now.duration_since(self.last_event).as_secs_f32(),
            time_since_last_frame: self.frame_delta,
        };
        self.last_event = now;
        time
    }
}

/// Times passed to user frame handlers.
///
/// Use `time_since_last_frame` for animation: it is the same for all handlers
/// of a frame. It includes time spent with a zero-sized window before the
/// first resumed frame; clamp it in the application if large steps are unwanted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventTime {
    /// Time since the previous frame handler call, in seconds.
    /// For the first call, measured from loop startup.
    pub time_since_last_event: f32,
    /// Time between starts of consecutive frames, in seconds.
    /// For the first frame, measured from loop startup.
    pub time_since_last_frame: f32,
}

/// Signature of a user frame handler.
pub type FrameHandler = Box<dyn FnMut(&mut dyn SceneApi, &EventTime) -> Result<(), SceneError>>;

/// User frame handlers, called by the loop on every pass in registration
/// order, with access to the scene and frame times.
///
/// All `frame_started` handlers run before frame assembly, then all
/// `frame_ended` handlers run after a successful render attempt. A backend
/// timeout may skip presentation and still call `frame_ended`. Any handler
/// error stops the loop and becomes [`crate::EngineError::Scene`], preserving
/// changes already made. Both phases pause for a zero-sized window.
///
/// ```
/// use BFGE::math::Quat;
/// use BFGE::{Engine, EngineApi, EngineEvent, Transform};
///
/// let mut engine = Engine::new();
/// let root = engine.scene().root_node();
/// let node = engine.scene().create_node(root)?;
/// let mut angle = 0.0;
/// engine.add_event(EngineEvent {
///     frame_started: Box::new(move |scene, time| {
///         angle = (angle + time.time_since_last_frame) % std::f32::consts::TAU;
///         scene.set_node_transform(node, Transform::from_rotation(Quat::from_rotation_y(angle)))
///     }),
///     frame_ended: Box::new(|_, _| Ok(())),
/// })?;
/// # Ok::<(), BFGE::EngineError>(())
/// ```
pub struct EngineEvent {
    /// Called before frame build.
    pub frame_started: FrameHandler,
    /// Called after a successful render attempt, even if the backend skips it.
    pub frame_ended: FrameHandler,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_events_are_consumed_in_fifo_order() {
        let mut event_loop = EngineLoop::default();
        let size = WindowSize::new(800, 600);
        event_loop.push_internal(InternalEvent::WindowResized(size));
        event_loop.push_internal(InternalEvent::Tick);
        event_loop.push_internal(InternalEvent::WindowClosed);
        assert!(
            matches!(event_loop.next_internal(), Some(InternalEvent::WindowResized(actual)) if actual == size)
        );
        assert!(matches!(
            event_loop.next_internal(),
            Some(InternalEvent::Tick)
        ));
        assert!(matches!(
            event_loop.next_internal(),
            Some(InternalEvent::WindowClosed)
        ));
        assert!(event_loop.next_internal().is_none());
    }

    #[test]
    fn stopping_discards_pending_events_but_keeps_callbacks() {
        let mut event_loop = EngineLoop::default();
        event_loop.add_event(EngineEvent {
            frame_started: Box::new(|_, _| Ok(())),
            frame_ended: Box::new(|_, _| Ok(())),
        });
        event_loop.set_running(true);
        event_loop.push_internal(InternalEvent::Tick);
        event_loop.stop();
        assert!(!event_loop.is_running());
        assert!(event_loop.next_internal().is_none());
        assert_eq!(event_loop.callbacks().len(), 1);
    }
}
