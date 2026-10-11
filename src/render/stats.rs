//! Frame rendering statistics.

/// Frame rendering statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Number of draw calls.
    pub draw_calls: u32,
    /// Number of triangles.
    pub triangles: u32,
}
