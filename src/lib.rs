//! FREAK Engine (BFGE): a desktop graphics engine with an engine-owned window.
//!
//! Start with [`Engine`] and import [`EngineApi`] to call its operations.
//! [`SceneApi`] provides node transforms, object attachment and one viewpoint.
//! Models can be loaded from OBJ, glTF 2.0 and GLB; scene snapshots live in
//! memory for the lifetime of the engine. Rendering uses wgpu with depth testing
//! and a simple model color; textures, lighting and input events are unavailable.
//!
//! # Window lifecycle
//!
//! Run window operations on the application's main thread in a desktop session
//! with a presentation-capable graphics adapter. Closing the window returns
//! from [`EngineApi::run`]; explicitly shut down even when the loop fails.
//!
//! ```no_run
//! use BFGE::{Engine, EngineApi, EngineError, WindowDescriptor};
//!
//! const WINDOW_TITLE: &str = "My BFGE application";
//!
//! fn main() -> Result<(), EngineError> {
//!     let mut engine = Engine::new();
//!     engine.initialize(WindowDescriptor::new(WINDOW_TITLE, 800, 600))?;
//!     let result = engine.run();
//!     let shutdown = engine.shutdown();
//!     result?;
//!     shutdown?;
//!     Ok(())
//! }
//! ```
//!
//! # Scene access without a window
//!
//! Scene operations, model loading, frame handler registration and snapshots
//! are available before initialization. Use the same engine across sessions:
//! winit permits only one event loop per process. Shutdown resets the scene and
//! resources but keeps the snapshot and registered frame handlers.
//!
//! ```
//! use BFGE::math::Vec3;
//! use BFGE::{Engine, EngineApi, SceneError, Transform};
//!
//! let mut engine = Engine::new();
//! let scene = engine.scene();
//! let root = scene.root_node();
//! let child = scene.create_node(root)?;
//! scene.set_node_transform(child, Transform::from_translation(Vec3::X))?;
//! # Ok::<(), SceneError>(())
//! ```
//!
//! The repository's `docs/User_Guide.md` contains the client setup and examples;
//! `docs/FREAK_Engine_Public_API.md` describes the public contract, and
//! `docs/FREAK_Engine_Architecture.md` describes the target architecture.

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
