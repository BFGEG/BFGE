//! Render surface (swapchain and present).

/// A render surface bound to the engine's window.
///
/// Owns the swapchain and presents rendered frames. Backed by wgpu in the
/// next milestone.
#[derive(Debug)]
pub struct RenderSurface;
