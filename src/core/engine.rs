//! Engine facade and orchestrator.

use std::path::Path;
use std::time::Duration;

use crate::core::api::EngineApi;
use crate::core::engine_loop::{EngineEvent, EngineLoop};
use crate::core::error::EngineError;
use crate::platform::{PlatformAdapter, WindowDescriptor, WindowSize};
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::systems::lifecycle::{self, EngineParts};
use crate::systems::loading::{self, ModelLoadError};
use crate::systems::persistence::{self, ScenePersistenceError};

/// Engine facade and orchestrator: links the scene, resources, systems,
/// renderer, window and the frame lifecycle.
pub struct Engine {
    parts: EngineParts,
    event_loop: EngineLoop,
    platform: Option<PlatformAdapter>,
}

impl Engine {
    /// Creates the engine; the parts are assembled by `systems::lifecycle`.
    pub fn new() -> Self {
        Self {
            parts: lifecycle::create(),
            event_loop: EngineLoop::default(),
            platform: None,
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
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError> {
        if self.platform.is_some() {
            Err(EngineError::AlreadyInitialized)
        } else {
            let platform = PlatformAdapter::new(window)?;
            lifecycle::initialize(&mut self.parts, platform.window())?;
            self.platform = Some(platform);
            Ok(())
        }
    }

    fn run(&mut self) -> Result<(), EngineError> {
        if self.platform.is_none() {
            Err(EngineError::NotInitialized)
        } else {
            Err(EngineError::RunUnavailable)
        }
    }

    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError> {
        self.event_loop.add_event(event);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        if self.platform.is_none() {
            Err(EngineError::NotInitialized)
        } else {
            lifecycle::shutdown(&mut self.parts)?;
            self.platform = None;
            Ok(())
        }
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

    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError> {
        let platform = self.platform.as_mut().ok_or(EngineError::NotInitialized)?;
        platform.set_window_size(size)?;
        self.parts.renderer.set_window_size(size)?;
        Ok(())
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
