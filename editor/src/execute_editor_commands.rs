use bevy::{prelude::*, window::PrimaryWindow};
use rfd::FileDialog;

use crate::{
    camera::EditorCamera,
    defs::{
        CurrentObjectManipulationMode, EditorAxis, EditorGltfInstances, EditorSelected,
        IncludeInSave, InstantiatedGltfInstance, ManipulationMode, MoveState, RotationEditState,
    },
    helper_funcs::{deselect_entity, strip_assets_prefix},
    input::HighlightedMesh,
    saving_loading::save_scene_system,
};
#[derive(Debug, Message)]
pub enum EditorCommand {
    OpenFile,
    DeleteSelected,
    BeginTransform,
    SetMode(ManipulationMode),
    RestrictAxis(EditorAxis),
    Confirm,
    Cancel,
    SwapLocal,
    Save,
}
pub fn editor_command_executor(
    mut cmd_reader: MessageReader<EditorCommand>,
    mut move_state: ResMut<MoveState>,
    mut commands: Commands,
    mut global_transforms: Query<&mut GlobalTransform>,
    cam_q: Query<Entity, With<EditorCamera>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut manip_mode: ResMut<CurrentObjectManipulationMode>,
    highlighted_mesh_q: Query<Entity, With<HighlightedMesh>>,
    mut selected_entity: ResMut<EditorSelected>,
    mut rotation_edit_state: ResMut<RotationEditState>,
    mut gltf_instances: ResMut<EditorGltfInstances>,
    asset_server: Res<AssetServer>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    for cmd in cmd_reader.read() {
        match cmd {
            EditorCommand::OpenFile => {
                let Ok(cam_entity) = cam_q.single() else {
                    return;
                };
                let Ok(cam_transform) = global_transforms.get(cam_entity) else {
                    return;
                };
                open_file_dialog(
                    &mut commands,
                    &asset_server,
                    cam_transform,
                    &mut gltf_instances,
                );
            }
            EditorCommand::DeleteSelected => {
                if let Some(entity) = selected_entity.0 {
                    commands.entity(entity).despawn();
                    selected_entity.0 = None;
                    manip_mode.mode = ManipulationMode::None;
                }
            }
            EditorCommand::SetMode(mode) => {
                // Reset whatever transforming if we were in a different manipulation mode
                // since it wasn't confirmed
                if manip_mode.mode != ManipulationMode::None {
                    revert_transform(selected_entity.0, &mut global_transforms, &mut move_state);
                }
                manip_mode.axis_restriction = None;
                manip_mode.mode = *mode;
            }
            EditorCommand::Cancel => {
                if manip_mode.mode != ManipulationMode::None {
                    revert_transform(selected_entity.0, &mut global_transforms, &mut move_state);
                } else {
                    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
                    deselect_entity(
                        &mut selected_entity,
                        &mut rotation_edit_state,
                        highlighted_mesh_q,
                        &mut commands,
                        config,
                    );
                }

                manip_mode.mode = ManipulationMode::None;
                manip_mode.axis_restriction = None;
            }
            EditorCommand::Confirm => {
                if manip_mode.mode != ManipulationMode::None {
                    manip_mode.mode = ManipulationMode::None;
                    manip_mode.axis_restriction = None;
                }
            }
            EditorCommand::BeginTransform => {
                if let Some(entity) = selected_entity.0 {
                    let Ok(obj_global_transform) = global_transforms.get(entity) else {
                        continue;
                    };

                    let Ok(cam_entity) = cam_q.single() else {
                        continue;
                    };
                    let Ok(cam_transform) = global_transforms.get(cam_entity) else {
                        continue;
                    };
                    let Ok(window) = window_q.single() else {
                        continue;
                    };
                    let Some(cursor_pos) = window.cursor_position() else {
                        continue;
                    };

                    let depth = (obj_global_transform.translation() - cam_transform.translation())
                        .dot(*cam_transform.forward());

                    move_state.start_cursor = cursor_pos;
                    move_state.start_transform = *obj_global_transform;
                    move_state.start_depth = depth;
                }
            }
            EditorCommand::RestrictAxis(axis) => {
                revert_transform(selected_entity.0, &mut global_transforms, &mut move_state);
                manip_mode.axis_restriction = Some(*axis)
            }
            EditorCommand::SwapLocal => {
                manip_mode.local = !manip_mode.local;
            }
            EditorCommand::Save => {
                dbg!("run");
                commands.run_system_cached(save_scene_system);
            }
        }
    }
}
fn revert_transform(
    selected: Option<Entity>,
    transforms: &mut Query<&mut GlobalTransform>,
    move_state: &mut MoveState,
) {
    if let Some(entity) = selected {
        if let Ok(mut obj_transform) = transforms.get_mut(entity) {
            *obj_transform = move_state.start_transform;
            let mut local_trans = obj_transform.compute_transform();
            local_trans.translation.x += 1000.0;
            *obj_transform = GlobalTransform::from(local_trans);
        }
    }
}
pub fn open_file_dialog(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    cam_transform: &GlobalTransform,
    gltf_instances: &mut ResMut<EditorGltfInstances>,
) {
    if let Some(path) = FileDialog::new().pick_file() {
        dbg!(&path);
        match strip_assets_prefix(&path) {
            Some(relative_path) => {
                println!("Relative path inside assets: {}", relative_path.display());
                let gltf = asset_server.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset(relative_path.to_string_lossy().to_string()),
                );
                let gltf_entity = commands
                    .spawn((
                        Name::new("test glb"),
                        SceneRoot(gltf),
                        *cam_transform,
                        IncludeInSave,
                    ))
                    .id();
                dbg!("spawned with includeinsave");
                gltf_instances.0.push(InstantiatedGltfInstance {
                    path: path.to_string_lossy().to_string(),
                    entity: gltf_entity,
                    parent: None,
                });
            }
            None => {
                eprintln!("Selected file is not inside the assets folder!");
            }
        }
    } else {
        println!("No file selected");
    }
}
