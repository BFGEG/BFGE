//! BFGE is a Rust graphics engine for rendering static 3D scenes in a desktop window.
//!
//! The repository README introduces the engine. The `docs` directory describes its
//! architecture, public API, rendering backend, and benchmark methodology.

#![warn(missing_docs)]
// `BFGE` is a product name, not a variable/module; the crate must not be renamed.
#![allow(non_snake_case)]

pub mod backend;
pub mod core;
pub mod math;
pub mod platform;
pub mod render;
pub mod resources;
pub mod scene;
#[cfg(feature = "stress-benchmark")]
#[path = "../benchmark/stress.rs"]
pub mod stress;
pub mod systems;

pub use crate::core::api::EngineApi;
pub use crate::core::engine::Engine;
pub use crate::core::engine_loop::{EngineEvent, EventTime};
pub use crate::core::error::EngineError;
pub use crate::platform::{WindowDescriptor, WindowSize};
pub use crate::render::stats::RenderStats;
pub use crate::render::system::RenderError;
pub use crate::scene::api::SceneApi;
pub use crate::scene::error::SceneError;
pub use crate::scene::ids::{SceneNodeId, SceneObjectId};
pub use crate::scene::lighting::{DirectionalLight, SceneLighting};
pub use crate::scene::transform::Transform;
pub use crate::scene::viewpoint::ViewpointState;
pub use crate::systems::loading::ModelLoadError;
pub use crate::systems::persistence::ScenePersistenceError;
