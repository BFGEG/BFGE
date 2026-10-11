//! Window and platform adapter owned by the engine.

const WINDOW_CREATION_UNAVAILABLE: &str = "window creation is not implemented";

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
/// Owned by [`PlatformAdapter`]; the backend uses it to create a render
/// surface. Backed by the windowing library in the next milestone.
#[derive(Debug)]
pub struct Window;

/// Owns the engine's window and delivers its events (close, resize).
///
/// The client does not create or manage the window; it only supplies a
/// [`WindowDescriptor`]. Event delivery is backed by the windowing library
/// in the next milestone.
#[derive(Debug)]
pub struct PlatformAdapter {
    window: Window,
}

impl PlatformAdapter {
    /// Creates the window and the platform adapter.
    pub(crate) fn new(_descriptor: WindowDescriptor) -> Result<Self, PlatformError> {
        Err(PlatformError::WindowCreation(
            WINDOW_CREATION_UNAVAILABLE.to_owned(),
        ))
    }

    /// Returns the engine's window.
    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    /// Resizes the window.
    pub(crate) fn set_window_size(&mut self, _size: WindowSize) -> Result<(), PlatformError> {
        todo!("window resize: milestone 2")
    }
}

/// Platform (window) error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// Failed to create the window.
    #[error("failed to create the window: {0}")]
    WindowCreation(String),
}
