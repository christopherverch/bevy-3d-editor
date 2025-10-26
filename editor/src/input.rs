use bevy::{
    color::palettes::css::{BLUE, CRIMSON, GREEN, RED},
    input::mouse::{MouseMotion, MouseWheel},
    math::Affine3A,
    pbr::wireframe::Wireframe,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use crate::{
    camera::{EditorCamera, toggle_cursor_condition},
    defs::{
        CurrentObjectManipulationMode, EditorAxis, EditorChildOf, EditorMaterials, EditorSelected,
        GltfEntityRoot, ManipulationMode, MoveState, RotationEditState,
    },
    execute_editor_commands::{EditorCommand, editor_command_executor},
    helper_funcs::{deselect_entity, scale_increment_falloff},
};
pub struct EditorInputPlugin;

impl Plugin for EditorInputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EditorInput::default())
            // Only check the hotkeys if we are not in freecam mode
            .add_message::<EditorCommand>()
            .add_systems(
                PostUpdate,
                resolve_hotkey_intents.run_if(toggle_cursor_condition),
            )
            .add_systems(Update, transform_using_manipulation_mode)
            .add_systems(EguiPrimaryContextPass, editor_command_executor)
            .add_systems(PreUpdate, store_mouse_deltas);
    }
}

#[derive(Debug, Resource, Default)]
pub struct EditorInput {
    pub mouse_movement: Vec2,
    pub mouse_scroll: f32,
}
fn store_mouse_deltas(
    mut input: ResMut<EditorInput>,
    mut mouse_evr: MessageReader<MouseMotion>,
    mut wheel_events: MessageReader<MouseWheel>,
) {
    let mut rotation = Vec2::ZERO;
    for ev in mouse_evr.read() {
        rotation += ev.delta;
    }
    input.mouse_movement = rotation;
    let mut scroll_delta = 0.0;
    for wheel_ev in wheel_events.read() {
        scroll_delta += wheel_ev.y;
    }
    input.mouse_scroll = scroll_delta;
}
fn resolve_hotkey_intents(
    selected_entity: Res<EditorSelected>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cmd_writer: MessageWriter<EditorCommand>,
    manip_mode: Res<CurrentObjectManipulationMode>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut egui_ctx: EguiContexts,
) {
    let ctx = egui_ctx.ctx_mut().unwrap();
    if ctx.wants_keyboard_input() || ctx.wants_pointer_input() {
        return;
    }
    if mouse_buttons.pressed(MouseButton::Left) {
        cmd_writer.write(EditorCommand::Confirm);
    }
    if keyboard_input.just_pressed(KeyCode::KeyS) && keyboard_input.pressed(KeyCode::ControlLeft) {
        cmd_writer.write(EditorCommand::Save);
        return;
    }
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        cmd_writer.write(EditorCommand::SwapLocal);
    }
    if keyboard_input.just_pressed(KeyCode::KeyO) {
        cmd_writer.write(EditorCommand::OpenFile);
    }
    if selected_entity.0.is_some() {
        if keyboard_input.just_pressed(KeyCode::Delete) {
            cmd_writer.write(EditorCommand::DeleteSelected);
        }
        if keyboard_input.just_pressed(KeyCode::KeyG) {
            cmd_writer.write(EditorCommand::SetMode(ManipulationMode::Move));
            cmd_writer.write(EditorCommand::BeginTransform);
        }
        if keyboard_input.just_pressed(KeyCode::KeyR) {
            cmd_writer.write(EditorCommand::SetMode(ManipulationMode::Rotate));
            cmd_writer.write(EditorCommand::BeginTransform);
        }
        if keyboard_input.just_pressed(KeyCode::KeyS) {
            cmd_writer.write(EditorCommand::SetMode(ManipulationMode::Scale));
            cmd_writer.write(EditorCommand::BeginTransform);
        }
        if keyboard_input.just_pressed(KeyCode::Escape) {
            cmd_writer.write(EditorCommand::Cancel);
        }

        if manip_mode.mode != ManipulationMode::None {
            if keyboard_input.just_pressed(KeyCode::KeyX) {
                cmd_writer.write(EditorCommand::RestrictAxis(EditorAxis::X));
            }
            if keyboard_input.just_pressed(KeyCode::KeyY) {
                cmd_writer.write(EditorCommand::RestrictAxis(EditorAxis::Y));
            }
            if keyboard_input.just_pressed(KeyCode::KeyZ) {
                cmd_writer.write(EditorCommand::RestrictAxis(EditorAxis::Z));
            }
        }
    }
}

#[derive(Component)]
pub struct HighlightedMesh;
pub fn change_selected_entity(
    event: On<Pointer<Press>>,
    mut selected_entity: ResMut<EditorSelected>,
    mut rotation_edit_state: ResMut<RotationEditState>,
    highlighted_mesh_q: Query<Entity, With<HighlightedMesh>>,
    root_q: Query<&GltfEntityRoot>,
    entities_with_children: Query<
        (&GlobalTransform, Option<&Mesh3d>, Option<&Children>),
        Without<HighlightedMesh>,
    >,
    editor_materials: Res<EditorMaterials>,
    manip_mode: Res<CurrentObjectManipulationMode>,
    meshes: Res<Assets<Mesh>>,
    editor_childof_query: Query<&EditorChildOf>,
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    dbg!("pressed");
    // Don't select another object if we're trying to move/rotate an object already
    if manip_mode.mode != ManipulationMode::None {
        return;
    }
    // Don't allow selecting the highlights...
    if highlighted_mesh_q.get(event.entity).is_ok() {
        dbg!("selected highlight");
    }
    deselect_entity(
        &mut selected_entity,
        &mut rotation_edit_state,
        highlighted_mesh_q,
        &mut commands,
        config,
    );
    config.enabled = true;
    let entity = event.event_target();

    // Add new highlight entities to each mesh component
    if let Ok(root_entity) = root_q.get(entity) {
        if let Ok((root_global_transform, _, _)) = entities_with_children.get(root_entity.0) {
            let root_inverse = root_global_transform.affine().inverse();
            spawn_highlight_for_mesh_recursive(
                &entities_with_children,
                root_entity.0,
                &mut commands,
                &meshes,
                &editor_materials,
                editor_childof_query,
                root_entity.0,
                root_inverse,
            );
            selected_entity.0 = Some(root_entity.0);
        }
    }
}

fn spawn_highlight_for_mesh_recursive(
    entities_with_children: &Query<
        (&GlobalTransform, Option<&Mesh3d>, Option<&Children>),
        Without<HighlightedMesh>,
    >,
    current_entity: Entity,
    commands: &mut Commands,
    meshes: &Res<Assets<Mesh>>,
    editor_materials: &EditorMaterials,
    editor_childof_query: Query<&EditorChildOf>,
    mut root_entity: Entity,
    mut root_inverse: Affine3A,
) {
    if let Ok((transform, mesh_opt, children_opt)) = entities_with_children.get(current_entity) {
        // If it has editorchildof component, that means it's a separate object, not just a child,
        // and we should treat it as such
        if editor_childof_query.get(current_entity).is_ok() {
            root_entity = current_entity;
            root_inverse = transform.affine().inverse();
        }
        // Create a new entity with a highlight material for each mesh3d component we find
        if let Some(mesh) = mesh_opt {
            let highlight_affine = root_inverse * transform.affine();
            let highlight_global_transform = GlobalTransform::from(highlight_affine);
            let mut new_transform = highlight_global_transform.compute_transform();
            let avg_scale =
                (new_transform.scale.x + new_transform.scale.y + new_transform.scale.z) / 3.0;
            let scale_increment = scale_increment_falloff(avg_scale);
            new_transform.scale += Vec3::splat(scale_increment);

            let highlight_entity = commands
                .spawn((
                    new_transform,
                    Mesh3d(mesh.0.clone()),
                    MeshMaterial3d(editor_materials.pressed_matl.clone()),
                    HighlightedMesh,
                ))
                .id();
            commands.entity(highlight_entity).insert(Wireframe);
            commands
                .entity(highlight_entity)
                .observe(change_selected_entity);
            commands
                .entity(highlight_entity)
                .insert(GltfEntityRoot(root_entity));

            commands
                .entity(highlight_entity)
                .insert(ChildOf(root_entity));
        }
        // Recurse into children
        if let Some(children) = children_opt {
            for child in children.iter() {
                spawn_highlight_for_mesh_recursive(
                    entities_with_children,
                    child,
                    commands,
                    meshes,
                    editor_materials,
                    editor_childof_query,
                    root_entity,
                    root_inverse,
                );
            }
        }
    }
}
fn transform_using_manipulation_mode(
    window_q: Query<&Window, With<PrimaryWindow>>,
    cam_q: Query<(Entity, &Camera), With<EditorCamera>>,
    mut transforms: Query<(&GlobalTransform, &mut Transform)>,
    move_state: Res<MoveState>,
    selected_entity: Res<EditorSelected>,
    manip_mode: Res<CurrentObjectManipulationMode>,
    mut rotation_edit_state: ResMut<RotationEditState>,
) {
    if manip_mode.mode == ManipulationMode::None {
        return;
    }
    dbg!(manip_mode.mode);
    let Ok(window) = window_q.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((cam_entity, camera)) = cam_q.single() else {
        return;
    };
    let Ok((cam_transform, _cam_local_transform)) = transforms.get(cam_entity) else {
        return;
    };
    let cam_transform = cam_transform.clone();
    let Some(entity) = selected_entity.0 else {
        return;
    };
    let Ok((global_transform, mut transform)) = transforms.get_mut(entity) else {
        return;
    };
    let Ok(current_ray) = camera.viewport_to_world(&cam_transform, cursor_position) else {
        return;
    };
    match manip_mode.mode {
        ManipulationMode::Move => move_object(
            global_transform,
            &mut transform,
            &move_state,
            &cam_transform,
            current_ray,
            camera,
            &manip_mode,
        ),
        ManipulationMode::Rotate => {
            rotate_object(
                &mut transform,
                global_transform,
                &move_state,
                &cam_transform,
                window,
                cursor_position,
                &manip_mode,
                &mut rotation_edit_state,
            );
        }
        ManipulationMode::Scale => {
            scale_object(
                &mut transform,
                &move_state,
                &cam_transform,
                current_ray,
                camera,
                &manip_mode,
            );
        }
        ManipulationMode::None => {}
    }
}
fn move_object(
    global_transform: &GlobalTransform,
    transform: &mut Transform,
    move_state: &MoveState,
    cam_transform: &GlobalTransform,
    current_ray: Ray3d,
    camera: &Camera,
    manip_mode: &CurrentObjectManipulationMode,
) {
    let child_local = transform.clone();
    let child_global = global_transform;
    let child_affine = child_local.compute_affine();
    let global_affine = child_global.affine();
    let parent_affine = global_affine * child_affine.inverse();

    let plane_origin = move_state.start_transform.translation();
    let plane_normal = cam_transform.forward();

    let Ok(start_ray) = camera.viewport_to_world(cam_transform, move_state.start_cursor) else {
        return;
    };
    let Some(start_world_pos) = intersect_ray_with_plane(start_ray, plane_origin, *plane_normal)
    else {
        return;
    };
    let Some(current_world_pos) =
        intersect_ray_with_plane(current_ray, plane_origin, *plane_normal)
    else {
        return;
    };

    let mut movement_delta = current_world_pos - start_world_pos;
    if let Some(axis) = manip_mode.axis_restriction {
        // Translate in either local or global space
        if manip_mode.local {
            let local_rotation = move_state.start_transform.rotation();
            let local_x = local_rotation * Vec3::X;
            let local_y = local_rotation * Vec3::Y;
            let local_z = local_rotation * Vec3::Z;
            // Transform delta into local space
            movement_delta = match axis {
                EditorAxis::X => local_x * movement_delta.dot(local_x),
                EditorAxis::Y => local_y * movement_delta.dot(local_y),
                EditorAxis::Z => local_z * movement_delta.dot(local_z),
            };
        } else {
            // Axis restriction in world space (your current logic)
            movement_delta = match axis {
                EditorAxis::X => Vec3::new(movement_delta.x, 0.0, 0.0),
                EditorAxis::Y => Vec3::new(0.0, movement_delta.y, 0.0),
                EditorAxis::Z => Vec3::new(0.0, 0.0, movement_delta.z),
            };
        }
    }
    // Desired new global translation
    let desired_global_translation = move_state.start_transform.translation() + movement_delta;

    // Convert desired global translation into local space relative to parent
    // Construct a translation matrix for desired global position
    let desired_global_affine = Mat4::from_translation(desired_global_translation);

    // Calculate local affine by removing parent's transform
    let local_affine = parent_affine.inverse() * desired_global_affine;

    // Extract translation from local affine and set it as new local translation
    transform.translation = local_affine.to_scale_rotation_translation().2;
}
fn rotate_object(
    transform: &mut Transform,
    global_transform: &GlobalTransform,
    move_state: &MoveState,
    cam_transform: &GlobalTransform,
    window: &Window,
    current_cursor: Vec2,
    manipulation_mode: &CurrentObjectManipulationMode,
    rotation_edit_state: &mut RotationEditState,
) {
    let window_size = Vec2::new(window.width(), window.height());
    // If no axis lock, do full trackball rotation
    if manipulation_mode.axis_restriction.is_none() {
        // Normalize screen positions to [-1, 1] (NDC space)
        let to_ndc = |cursor: Vec2| {
            let mut ndc = (cursor / window_size) * 2.0 - Vec2::ONE;
            ndc.y = -ndc.y;
            ndc
        };

        let start_ndc = to_ndc(move_state.start_cursor);
        let current_ndc = to_ndc(current_cursor);

        // Map to virtual trackball sphere
        fn project_to_sphere(ndc: Vec2) -> Vec3 {
            let x = ndc.x;
            let y = ndc.y;
            let z_squared = 1.0 - x * x - y * y;
            let z = if z_squared > 0.0 {
                z_squared.sqrt()
            } else {
                0.0
            };
            Vec3::new(x, y, z).normalize()
        }

        let v0 = project_to_sphere(start_ndc);
        let v1 = project_to_sphere(current_ndc);

        // Axis of rotation is the cross product of the two vectors
        let axis = v0.cross(v1);
        if axis.length_squared() < 1e-6 {
            return; // No meaningful rotation
        }

        let angle = v0.angle_between(v1);
        let world_axis = cam_transform.rotation() * axis;

        let rotation = Quat::from_axis_angle(world_axis, angle);
        transform.rotation = rotation * move_state.start_transform.rotation();
    } else {
        // Axis lock rotation
        // Extract the locked axis in world space or local space
        let locked_axis = match manipulation_mode.axis_restriction.unwrap() {
            EditorAxis::X => {
                if manipulation_mode.local {
                    move_state.start_transform.rotation() * Vec3::X
                } else {
                    Vec3::X
                }
            }
            EditorAxis::Y => {
                if manipulation_mode.local {
                    move_state.start_transform.rotation() * Vec3::Y
                } else {
                    Vec3::Y
                }
            }
            EditorAxis::Z => {
                if manipulation_mode.local {
                    move_state.start_transform.rotation() * Vec3::Z
                } else {
                    Vec3::Z
                }
            }
        };
        // Use horizontal cursor delta as rotation input
        let cursor_delta = current_cursor - move_state.start_cursor;
        let rotation_speed = 0.01;
        let angle = cursor_delta.x * rotation_speed;

        // Build rotation quaternion around locked axis
        let rotation = Quat::from_axis_angle(locked_axis, angle);

        // Apply rotation relative to original rotation
        transform.rotation = rotation * move_state.start_transform.rotation();
    }
    let (pitch, yaw, roll) = transform.rotation.to_euler(EulerRot::XYZ);
    rotation_edit_state.rotation_edit_euler =
        Some((pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees()));
    rotation_edit_state.initial_global_transform = Some(global_transform.clone());
    rotation_edit_state.initial_transform = Some(transform.clone());
}
fn scale_object(
    transform: &mut Transform,
    move_state: &MoveState,
    cam_transform: &GlobalTransform,
    current_ray: Ray3d,
    camera: &Camera,
    manip_mode: &CurrentObjectManipulationMode,
) {
    let plane_origin = move_state.start_transform.translation();
    let plane_normal = cam_transform.forward();

    let Ok(start_ray) = camera.viewport_to_world(cam_transform, move_state.start_cursor) else {
        return;
    };
    let Some(start_world_pos) = intersect_ray_with_plane(start_ray, plane_origin, *plane_normal)
    else {
        return;
    };
    let Some(current_world_pos) =
        intersect_ray_with_plane(current_ray, plane_origin, *plane_normal)
    else {
        return;
    };

    // Distances from object center (plane_origin)
    let start_dist = (start_world_pos - plane_origin).length();
    let current_dist = (current_world_pos - plane_origin).length();

    if start_dist == 0.0 {
        return; // avoid division by zero
    }

    // Calculate uniform scale factor: ratio of current distance to start distance
    let mut scale_factor = current_dist / start_dist;

    // Clamp scale factor to avoid zero or negative scaling
    scale_factor = scale_factor.max(0.01);

    // Prepare final scale vector
    let final_scale;

    if let Some(axis) = manip_mode.axis_restriction {
        // Scale only along locked axis, keep others 1.0
        final_scale = match axis {
            EditorAxis::X => Vec3::new(scale_factor, 1.0, 1.0),
            EditorAxis::Y => Vec3::new(1.0, scale_factor, 1.0),
            EditorAxis::Z => Vec3::new(1.0, 1.0, scale_factor),
        };
    } else {
        // Uniform scale on all axes
        final_scale = Vec3::splat(scale_factor);
    }

    // Apply relative to starting scale
    transform.scale = move_state.start_transform.scale() * final_scale;
}
fn intersect_ray_with_plane(ray: Ray3d, plane_origin: Vec3, plane_normal: Vec3) -> Option<Vec3> {
    let denom = ray.direction.dot(plane_normal);
    if denom.abs() > f32::EPSILON {
        let t = (plane_origin - ray.origin).dot(plane_normal) / denom;
        if t >= 0.0 {
            Some(ray.origin + t * ray.direction)
        } else {
            None
        }
    } else {
        None
    }
}
