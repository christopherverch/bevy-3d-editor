use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::outline_material::OutlineMaterial;

#[derive(Resource)]
pub struct EditorMaterials {
    pub pressed_matl: Handle<OutlineMaterial>,
    pub hover_matl: Handle<StandardMaterial>,
    pub white_matl: Handle<StandardMaterial>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManipulationMode {
    #[default]
    None,
    Move,
    Rotate,
    Scale,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAxis {
    X,
    Y,
    Z,
}
#[derive(Resource, Default)]
pub struct CurrentObjectManipulationMode {
    pub mode: ManipulationMode,
    pub axis_restriction: Option<EditorAxis>,
    pub local: bool,
}
#[derive(Component, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Default)]
pub struct IncludeInSave;
#[derive(Component, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct GltfRef {
    // The path to the original glTF file
    pub asset_path: String,
    // The ID of the scene/node within the glTF file, e.g., "#Scene0"
    pub label: Option<String>,
}

#[derive(Component)]
pub struct FinishedGltfRefLoading;
#[derive(Debug, Resource, Default)]
pub struct MoveState {
    pub start_cursor: Vec2,
    pub start_transform: GlobalTransform,
    pub start_local_transform: Transform,
    pub start_depth: f32,
}
#[derive(Component)]
pub struct EditorEntityLink(pub Entity);
#[derive(Component)]
pub struct GltfEntityRoot(pub Entity);
// New type so we can serialize the transform
#[derive(Serialize, Deserialize, Debug)]
pub struct EditorGltfInstance {
    pub path: String, // Path to the GLTF or scene asset
    pub transform: TransformData,
    pub parent: Option<usize>, // Index in the Vec during save/load
}
#[derive(Serialize, Deserialize, Debug)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
impl From<Transform> for TransformData {
    fn from(t: Transform) -> Self {
        Self {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}
impl From<TransformData> for Transform {
    fn from(t: TransformData) -> Self {
        Transform {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}
#[derive(Resource, Default)]
pub struct EditorGltfInstances(pub Vec<InstantiatedGltfInstance>);

pub struct InstantiatedGltfInstance {
    pub path: String,
    pub entity: Entity,
    pub parent: Option<usize>,
}
#[derive(Component)]
pub struct EditorChildOf(pub Entity);
#[derive(Resource, Default)]
pub struct EditorSelected(pub Option<Entity>);
#[derive(Resource, Default)]
pub struct UiBuffers {
    pub search_buf: String,
    pub rename_buf: String,
}

#[derive(Resource, Default)]
pub struct RotationEditState {
    pub initial_transform: Option<Transform>,
    pub initial_global_transform: Option<GlobalTransform>,
    pub rotation_edit_euler: Option<(f32, f32, f32)>,
}
