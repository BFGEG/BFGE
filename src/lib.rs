//! FREAK Engine (BFGE) - a Rust graphics engine.
//!
//! Public contract - `FREAK_Engine_Public_API.md`;
//! internal skeleton - `docs/FREAK_Engine_Architecture.md`.

#![warn(missing_docs)]
// `BFGE` is a product name, not a variable/module; the crate must not be renamed.
#![allow(non_snake_case)]

pub mod backend;
pub mod core;
pub mod math;
pub mod render;
pub mod resources;
pub mod scene;
pub mod systems;

pub use crate::core::api::{EngineApi, ImageListener};
pub use crate::core::engine::Engine;
pub use crate::core::error::EngineError;
pub use crate::render::image::{ImageSize, SceneImage};
pub use crate::render::stats::RenderStats;
pub use crate::render::system::RenderError;
pub use crate::scene::api::SceneApi;
pub use crate::scene::error::SceneError;
pub use crate::scene::ids::{SceneNodeId, SceneObjectId};
pub use crate::scene::transform::Transform;
pub use crate::scene::viewpoint::ViewpointState;
pub use crate::systems::loading::ModelLoadError;
pub use crate::systems::persistence::ScenePersistenceError;
