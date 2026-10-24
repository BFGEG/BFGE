//! Scene save and restore.

use crate::resources::handle::MeshHandle;
use crate::resources::manager::ResourceManager;
use crate::resources::mesh::Mesh;
use crate::scene::api::SceneApi;
use crate::scene::ids::{SceneNodeId, SceneObjectId};
use crate::scene::lighting::SceneLighting;
use crate::scene::manager::SceneManager;
use crate::scene::node::SceneNode;
use crate::scene::object::SceneObject;
use crate::scene::transform::Transform;
use crate::scene::viewpoint::ViewpointState;

/// Storage for a single saved scene (version 1 - in engine memory).
///
/// The store lives for the whole engine lifetime and survives `shutdown`;
/// it is shared between sessions because the engine lives longer than a
/// single session. The saved scene is lost only when the engine is dropped.
#[derive(Default)]
pub struct SceneStore {
    saved: Option<SavedScene>,
}

impl SceneStore {
    /// Creates an empty scene store.
    pub fn new() -> Self {
        Self::default()
    }
}

/// The full scene state captured by a save.
#[derive(Clone)]
struct SavedScene {
    nodes: Vec<SavedNode>,
    objects: Vec<SavedObject>,
    meshes: Vec<Mesh>,
    viewpoint: ViewpointState,
    lighting: SceneLighting,
}

/// A saved scene node: parent, children, attached objects and a local transform.
#[derive(Clone)]
struct SavedNode {
    parent: Option<SceneNodeId>,
    children: Vec<SceneNodeId>,
    objects: Vec<SceneObjectId>,
    local_transform: Transform,
}

impl SavedNode {
    fn from_node(node: &SceneNode) -> Self {
        Self {
            parent: node.parent,
            children: node.children.clone(),
            objects: node.objects.clone(),
            local_transform: node.local_transform,
        }
    }

    fn into_node(self) -> SceneNode {
        SceneNode {
            parent: self.parent,
            children: self.children,
            objects: self.objects,
            local_transform: self.local_transform,
        }
    }
}

/// A saved scene object: the handle of the mesh it instances.
#[derive(Clone, Copy)]
struct SavedObject {
    mesh: MeshHandle,
}

impl SavedObject {
    fn from_object(object: &SceneObject) -> Self {
        Self { mesh: object.mesh }
    }

    fn into_object(self) -> SceneObject {
        SceneObject::new(self.mesh)
    }
}

/// Visit state of a node during parent-chain cycle detection.
#[derive(Clone, Copy)]
enum VisitState {
    /// The node has not been visited yet.
    Unvisited,
    /// The node belongs to the chain currently being walked.
    InProgress,
    /// The node's chain reaches the root without a cycle.
    Done,
}

impl SavedScene {
    /// Checks the saved state for structural integrity before it is applied.
    ///
    /// Every referenced node, object and mesh must exist; the root node (index
    /// zero) must not have a parent; the node hierarchy must be a consistent
    /// tree without orphan nodes, duplicate parent links or cycles; each object
    /// must be attached to at most one node. A malformed save is rejected
    /// instead of restoring it as-is.
    fn validate(&self) -> Result<(), String> {
        let node_count = self.nodes.len();
        if node_count == 0 || self.nodes[0].parent.is_some() {
            return Err(
                "saved scene must contain a root node (index zero) without a parent".to_owned(),
            );
        }
        let object_count = self.objects.len();
        let mesh_count = self.meshes.len();
        self.lighting
            .validate()
            .map_err(|error| error.to_string())?;

        for (index, node) in self.nodes.iter().enumerate() {
            if index != 0 && node.parent.is_none() {
                return Err(format!("node {index} has no parent"));
            }
            if let Some(parent) = node.parent
                && parent.0 as usize >= node_count
            {
                return Err(format!(
                    "node {index} references unknown parent {}",
                    parent.0
                ));
            }
            if let Some(parent) = node.parent {
                let links = self.nodes[parent.0 as usize]
                    .children
                    .iter()
                    .filter(|child| **child == SceneNodeId(index as u32))
                    .count();
                if links != 1 {
                    return Err(format!(
                        "node {index} is attached to its parent {} {links} times",
                        parent.0
                    ));
                }
            }
            for child in &node.children {
                if child.0 as usize >= node_count {
                    return Err(format!("node {index} references unknown child {}", child.0));
                }
            }
            for object in &node.objects {
                if object.0 as usize >= object_count {
                    return Err(format!(
                        "node {index} references unknown object {}",
                        object.0
                    ));
                }
            }
        }

        // The parent relation must not loop: frame building walks the tree from
        // the root, and a cycle would recurse without bound. Every node except
        // the root has a parent, so an acyclic chain always ends at the root.
        let mut state = vec![VisitState::Unvisited; node_count];
        for start in 0..node_count {
            let mut chain = Vec::new();
            let mut current = start;
            loop {
                match state[current] {
                    VisitState::InProgress => {
                        return Err(format!("parent chain of node {start} contains a cycle"));
                    }
                    VisitState::Done => break,
                    VisitState::Unvisited => {
                        state[current] = VisitState::InProgress;
                        chain.push(current);
                        let Some(parent) = self.nodes[current].parent else {
                            break;
                        };
                        current = parent.0 as usize;
                    }
                }
            }
            for node in chain {
                state[node] = VisitState::Done;
            }
        }

        let mut attached = vec![false; object_count];
        for node in &self.nodes {
            for object in &node.objects {
                if attached[object.0 as usize] {
                    return Err(format!(
                        "object {} is attached to more than one node",
                        object.0
                    ));
                }
                attached[object.0 as usize] = true;
            }
        }

        for object in &self.objects {
            if object.mesh.index() as usize >= mesh_count {
                return Err(format!(
                    "object references unknown mesh index {} (have {mesh_count} meshes)",
                    object.mesh.index()
                ));
            }
        }

        Ok(())
    }

    fn to_scene_parts(&self) -> (Vec<SceneNode>, Vec<SceneObject>) {
        let nodes = self
            .nodes
            .iter()
            .cloned()
            .map(SavedNode::into_node)
            .collect();
        let objects = self.objects.iter().map(|obj| obj.into_object()).collect();
        (nodes, objects)
    }
}

/// Scene save or restore error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ScenePersistenceError {
    /// The scene is not empty: restore is only possible into an empty scene.
    #[error("scene is not empty: restore is only possible into an empty scene")]
    SceneNotEmpty,
    /// Failed to save the scene.
    #[error("failed to save the scene: {0}")]
    Save(String),
    /// Failed to restore the scene.
    #[error("failed to restore the scene: {0}")]
    Restore(String),
}

/// Saves the full scene state (replaces the previous save).
///
/// Captures the scene tree, the objects, the registered meshes and the
/// viewpoint and lighting. The previous save is replaced only after the new state has been
/// fully captured, so a failed save never destroys an earlier save. In-memory
/// saving cannot fail; [`ScenePersistenceError::Save`] is reserved for the
/// file-backed version (milestone 2).
///
/// # Errors
///
/// This function is infallible in version 1.
pub fn save_scene(
    store: &mut SceneStore,
    scene: &SceneManager,
    resources: &ResourceManager,
) -> Result<(), ScenePersistenceError> {
    let saved = SavedScene {
        nodes: scene.nodes().iter().map(SavedNode::from_node).collect(),
        objects: scene
            .objects()
            .iter()
            .map(SavedObject::from_object)
            .collect(),
        meshes: resources.meshes().to_vec(),
        viewpoint: scene.viewpoint(),
        lighting: scene.lighting(),
    };
    store.saved = Some(saved);
    Ok(())
}

/// Restores the saved scene into the empty current scene.
///
/// Restore is only possible into a scene without objects. When no save exists
/// the call succeeds and leaves the empty scene untouched. The saved state is
/// validated first; on failure nothing is modified.
///
/// # Errors
///
/// Returns [`ScenePersistenceError::Restore`] for a malformed save and
/// [`ScenePersistenceError::SceneNotEmpty`] when the current scene has objects.
pub fn restore_scene(
    store: &mut SceneStore,
    scene: &mut SceneManager,
    resources: &mut ResourceManager,
) -> Result<(), ScenePersistenceError> {
    if !scene.objects().is_empty() {
        return Err(ScenePersistenceError::SceneNotEmpty);
    }
    let Some(saved) = store.saved.as_ref() else {
        return Ok(());
    };
    saved.validate().map_err(ScenePersistenceError::Restore)?;
    let (nodes, objects) = saved.to_scene_parts();
    scene.restore(nodes, objects, saved.viewpoint, saved.lighting);
    resources.restore_meshes(saved.meshes.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::math::Vec3;
    use crate::platform::WindowSize;
    use crate::resources::library::{BuiltinMesh, MeshLibrary};
    use crate::scene::lighting::{DirectionalLight, SceneLighting};

    const TEST_WIDTH: u32 = 800;
    const TEST_HEIGHT: u32 = 600;

    #[test]
    fn save_and_restore_round_trips_scene_and_render_frame() {
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();
        let mesh = resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
        let root = scene.root_node();
        let child = scene.create_node(root).unwrap();
        scene
            .set_node_transform(
                child,
                Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            )
            .unwrap();
        scene.create_object(mesh, child).unwrap();
        scene
            .set_lighting(SceneLighting {
                ambient: Vec3::splat(0.1),
                directional: DirectionalLight {
                    direction: Vec3::X,
                    color: Vec3::splat(0.9),
                },
            })
            .unwrap();
        let mut store = SceneStore::new();

        let before = scene.build_render_frame(WindowSize::new(TEST_WIDTH, TEST_HEIGHT));
        save_scene(&mut store, &scene, &resources).unwrap();

        let mut scene_after = SceneManager::new();
        let mut resources_after = ResourceManager::new();
        restore_scene(&mut store, &mut scene_after, &mut resources_after).unwrap();

        let after = scene_after.build_render_frame(WindowSize::new(TEST_WIDTH, TEST_HEIGHT));
        assert_eq!(before.camera, after.camera);
        assert_eq!(before.lighting, after.lighting);
        assert_eq!(before.items, after.items);
        assert!(resources_after.mesh(mesh).is_some());
    }

    #[test]
    fn invalid_saved_lighting_does_not_change_the_scene() {
        let mut store = SceneStore {
            saved: Some(SavedScene {
                nodes: vec![SavedNode::from_node(&SceneNode::default())],
                objects: Vec::new(),
                meshes: Vec::new(),
                viewpoint: ViewpointState::default(),
                lighting: SceneLighting {
                    ambient: Vec3::splat(-1.0),
                    ..SceneLighting::default()
                },
            }),
        };
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        let result = restore_scene(&mut store, &mut scene, &mut resources);

        assert!(matches!(result, Err(ScenePersistenceError::Restore(_))));
        assert_eq!(scene.lighting(), SceneLighting::default());
    }

    #[test]
    fn save_captures_shared_meshes_by_content() {
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();
        let mesh = resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
        let root = scene.root_node();
        scene.create_object(mesh, root).unwrap();
        scene.create_object(mesh, root).unwrap();
        let mut store = SceneStore::new();

        save_scene(&mut store, &scene, &resources).unwrap();

        let mut scene_after = SceneManager::new();
        let mut resources_after = ResourceManager::new();
        restore_scene(&mut store, &mut scene_after, &mut resources_after).unwrap();

        assert_eq!(resources_after.meshes().len(), 1);
        assert_eq!(scene_after.objects().len(), 2);
        let expected = MeshLibrary::build(BuiltinMesh::Cube);
        assert_eq!(resources_after.mesh(mesh), Some(&expected));
    }

    #[test]
    fn restore_into_non_empty_scene_is_rejected() {
        let mut store = SceneStore::new();
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();
        let mesh = resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
        let root = scene.root_node();
        scene.create_object(mesh, root).unwrap();
        save_scene(&mut store, &scene, &resources).unwrap();

        let error = restore_scene(&mut store, &mut scene, &mut resources).unwrap_err();

        assert!(matches!(error, ScenePersistenceError::SceneNotEmpty));
    }

    #[test]
    fn restore_without_save_leaves_scene_untouched() {
        let mut store = SceneStore::new();
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        assert!(restore_scene(&mut store, &mut scene, &mut resources).is_ok());
        assert!(scene.objects().is_empty());
        assert!(scene.nodes().len() == 1);
    }

    #[test]
    fn repeated_save_replaces_previous_save() {
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();
        let mesh = resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
        let root = scene.root_node();
        scene.create_object(mesh, root).unwrap();
        let mut store = SceneStore::new();

        save_scene(&mut store, &scene, &resources).unwrap();
        assert!(store.saved.as_ref().unwrap().objects.len() == 1);

        let second_mesh = resources.add_mesh(MeshLibrary::build(BuiltinMesh::Cube));
        scene.create_object(second_mesh, root).unwrap();
        save_scene(&mut store, &scene, &resources).unwrap();

        assert!(store.saved.as_ref().unwrap().objects.len() == 2);

        let mut scene_after = SceneManager::new();
        let mut resources_after = ResourceManager::new();
        restore_scene(&mut store, &mut scene_after, &mut resources_after).unwrap();

        assert_eq!(scene_after.objects().len(), 2);
        assert_eq!(resources_after.meshes().len(), 2);
    }

    #[test]
    fn malformed_save_is_rejected_without_restoring_partially() {
        let mut store = SceneStore::new();
        store.saved = Some(SavedScene {
            nodes: vec![SavedNode {
                parent: None,
                children: Vec::new(),
                objects: vec![SceneObjectId(5)],
                local_transform: Transform::default(),
            }],
            objects: Vec::new(),
            meshes: Vec::new(),
            viewpoint: ViewpointState::default(),
            lighting: SceneLighting::default(),
        });
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        let result = restore_scene(&mut store, &mut scene, &mut resources);

        assert!(matches!(result, Err(ScenePersistenceError::Restore(_))));
        assert!(scene.objects().is_empty());
        assert!(scene.nodes().len() == 1);
        assert!(resources.meshes().is_empty());
    }

    #[test]
    fn malformed_save_with_orphan_node_is_rejected_without_changes() {
        let mut store = SceneStore::new();
        store.saved = Some(SavedScene {
            nodes: vec![
                SavedNode {
                    parent: None,
                    children: vec![SceneNodeId(1)],
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
                SavedNode {
                    parent: None,
                    children: Vec::new(),
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
            ],
            objects: Vec::new(),
            meshes: Vec::new(),
            viewpoint: ViewpointState::default(),
            lighting: SceneLighting::default(),
        });
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        let result = restore_scene(&mut store, &mut scene, &mut resources);

        assert!(matches!(result, Err(ScenePersistenceError::Restore(_))));
        assert!(scene.objects().is_empty());
        assert!(scene.nodes().len() == 1);
        assert!(resources.meshes().is_empty());
    }

    #[test]
    fn malformed_save_with_duplicate_child_links_is_rejected() {
        let mut store = SceneStore::new();
        store.saved = Some(SavedScene {
            nodes: vec![
                SavedNode {
                    parent: None,
                    children: vec![SceneNodeId(1), SceneNodeId(1)],
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
                SavedNode {
                    parent: Some(SceneNodeId(0)),
                    children: Vec::new(),
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
            ],
            objects: Vec::new(),
            meshes: Vec::new(),
            viewpoint: ViewpointState::default(),
            lighting: SceneLighting::default(),
        });
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        let result = restore_scene(&mut store, &mut scene, &mut resources);

        assert!(matches!(result, Err(ScenePersistenceError::Restore(_))));
        assert!(scene.objects().is_empty());
        assert!(scene.nodes().len() == 1);
        assert!(resources.meshes().is_empty());
    }

    #[test]
    fn malformed_save_with_cyclic_hierarchy_is_rejected() {
        let mut store = SceneStore::new();
        store.saved = Some(SavedScene {
            nodes: vec![
                SavedNode {
                    parent: None,
                    children: vec![SceneNodeId(1)],
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
                SavedNode {
                    parent: Some(SceneNodeId(2)),
                    children: vec![SceneNodeId(2)],
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
                SavedNode {
                    parent: Some(SceneNodeId(1)),
                    children: vec![SceneNodeId(1)],
                    objects: Vec::new(),
                    local_transform: Transform::default(),
                },
            ],
            objects: Vec::new(),
            meshes: Vec::new(),
            viewpoint: ViewpointState::default(),
            lighting: SceneLighting::default(),
        });
        let mut scene = SceneManager::new();
        let mut resources = ResourceManager::new();

        let result = restore_scene(&mut store, &mut scene, &mut resources);

        assert!(matches!(result, Err(ScenePersistenceError::Restore(_))));
        assert!(scene.objects().is_empty());
        assert!(scene.nodes().len() == 1);
        assert!(resources.meshes().is_empty());
    }
}
