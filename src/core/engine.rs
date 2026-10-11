//! Engine facade and orchestrator.

use std::path::Path;
use std::time::Duration;

use crate::core::api::{EngineApi, ImageListener};
use crate::core::error::EngineError;
use crate::render::image::ImageSize;
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::systems::lifecycle::{self, EngineParts};
use crate::systems::loading::{self, ModelLoadError};
use crate::systems::persistence::{self, ScenePersistenceError};

/// Engine facade and orchestrator: links the scene, resources, systems,
/// renderer and the frame lifecycle.
pub struct Engine {
    parts: EngineParts,
    #[allow(dead_code)] // consumed by EngineLoop (milestone 2)
    image_listener: Option<Box<dyn ImageListener>>,
    image_size: Option<ImageSize>,
}

impl Engine {
    /// Creates the engine; the parts are assembled by `systems::lifecycle`.
    pub fn new() -> Self {
        Self {
            parts: lifecycle::create(),
            image_listener: None,
            image_size: None,
        }
    }

    #[allow(dead_code)] // driven by EngineLoop (milestone 2)
    fn update(&mut self, _dt: Duration) -> Result<(), EngineError> {
        todo!("milestone 2")
    }

    #[allow(dead_code)] // driven by EngineLoop (milestone 2)
    fn render(&mut self) -> Result<(), EngineError> {
        todo!("milestone 2")
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineApi for Engine {
    fn initialize(&mut self, initial_image_size: ImageSize) -> Result<(), EngineError> {
        if self.image_size.is_some() {
            return Err(EngineError::AlreadyInitialized);
        }
        lifecycle::initialize(&mut self.parts, initial_image_size)?;
        self.image_size = Some(initial_image_size);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        if self.image_size.is_none() {
            return Err(EngineError::NotInitialized);
        }
        lifecycle::shutdown(&mut self.parts)?;
        self.image_size = None;
        Ok(())
    }

    fn scene(&mut self) -> &mut dyn SceneApi {
        &mut self.parts.scene
    }

    fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError> {
        loading::load_model(
            path,
            &mut self.parts.resources,
            &mut self.parts.scene,
            parent,
        )
    }

    fn set_image_size(&mut self, size: ImageSize) -> Result<(), EngineError> {
        if self.image_size.is_none() {
            return Err(EngineError::NotInitialized);
        }
        self.parts.renderer.resize(size)?;
        self.image_size = Some(size);
        Ok(())
    }

    fn set_image_listener(&mut self, listener: Box<dyn ImageListener>) {
        self.image_listener = Some(listener);
    }

    fn save_scene(&mut self) -> Result<(), ScenePersistenceError> {
        persistence::save_scene(
            &mut self.parts.store,
            &self.parts.scene,
            &self.parts.resources,
        )
    }

    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError> {
        persistence::restore_scene(
            &mut self.parts.store,
            &mut self.parts.scene,
            &mut self.parts.resources,
        )
    }
}
