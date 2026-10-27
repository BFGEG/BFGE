//! Frame rendering statistics.

/// Rendering counts, or zero counts when the frame is skipped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Number of submesh draw calls submitted.
    pub draw_calls: u32,
    /// Number of indexed triangles submitted.
    pub triangles: u32,
}
