use bevy::prelude::*;
use std::path::{Path, PathBuf};

use crate::{
    defs::{EditorSelected, RotationEditState},
    input::HighlightedMesh,
};
pub fn deselect_entity(
    selected_entity: &mut EditorSelected,
    rotation_edit_state: &mut RotationEditState,
    highlighted_mesh_q: Query<Entity, With<HighlightedMesh>>,
    commands: &mut Commands,
    config: &mut GizmoConfig,
) {
    config.enabled = false;
    selected_entity.0 = None;
    rotation_edit_state.initial_global_transform = None;
    rotation_edit_state.initial_transform = None;
    rotation_edit_state.rotation_edit_euler = None;
    // delete the old highlight entities
    for highlighted_mesh_entity in highlighted_mesh_q {
        commands.entity(highlighted_mesh_entity).despawn();
    }
}
pub fn strip_assets_prefix(path: &Path) -> Option<PathBuf> {
    let mut found_assets = false;

    let mut components = path.components();

    // Skip components until we find "assets"
    while let Some(c) = components.next() {
        if c.as_os_str() == "assets" {
            found_assets = true;
            break;
        }
    }

    if found_assets {
        // Collect the rest of the path components after "assets"
        Some(components.collect())
    } else {
        None
    }
}
// falloff = C / (scale + k) + min_value
// where C and k control shape, min_value ensures minimal size increase
pub fn scale_increment_falloff(scale: f32) -> f32 {
    let k = 0.1;
    let c = 0.011;
    let min_increment = 0.001; // minimum increment for very large scale

    (c / (scale + k)).max(min_increment)
}
