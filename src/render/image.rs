//! Image sizes and ready scene images.

/// Requested image size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ImageSize {
    /// Creates an image size.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Ready scene image (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixels: 4 bytes per pixel.
    pub pixels: Vec<u8>,
}
