//! Opaque resource identifiers.

/// Opaque identifier of a geometry resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(u32);

impl MeshHandle {
    pub(crate) fn new(index: u32) -> Self {
        Self(index)
    }

    pub(crate) fn index(self) -> u32 {
        self.0
    }
}
