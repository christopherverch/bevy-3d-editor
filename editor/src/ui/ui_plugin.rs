use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use egui::{Frame, Id, Ui, Widget};

use crate::{
    defs::{
        CurrentObjectManipulationMode, EditorChildOf, EditorSelected, IncludeInSave,
        ManipulationMode, RotationEditState, UiBuffers,
    },
    ui::dropdown_box::DropDownBox,
};
pub struct EditorUiPlugin;
impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, hierarchy_ui)
            .add_plugins(EguiPlugin::default())
            .add_systems(Update, draw_selection_gizmos)
            .insert_resource(UiBuffers::default());
    }
}
fn draw_selection_gizmos(
    mut gizmos: Gizmos,
    selected_entity_res: Res<EditorSelected>,
    global_transforms: Query<&GlobalTransform>,
    manip_mode: Res<CurrentObjectManipulationMode>,
) {
    if let Some(selected_entity) = selected_entity_res.0 {
        let Ok(selected_entity_transform) = global_transforms.get(selected_entity) else {
            return;
        };
        let pos = selected_entity_transform.translation();
        let length = 300.0;

        // Determine axes directions
        let (x_axis, y_axis, z_axis) = if manip_mode.local {
            let rot = selected_entity_transform.rotation();
            (rot * Vec3::X, rot * Vec3::Y, rot * Vec3::Z)
        } else {
            (Vec3::X, Vec3::Y, Vec3::Z)
        };

        gizmos.line(pos - x_axis * length, pos + x_axis * length, BLUE);
        gizmos.line(pos - y_axis * length, pos + y_axis * length, RED);
        gizmos.line(pos - z_axis * length, pos + z_axis * length, GREEN);
    }
}
fn hierarchy_ui(
    mut egui_ctx: EguiContexts,
    query: Query<
        (
            Entity,
            Option<&Name>,
            Option<&Children>,
            Option<&EditorChildOf>,
        ),
        With<IncludeInSave>,
    >,
    mut commands: Commands,
    mut ui_buffers: ResMut<UiBuffers>,
    mut selected_entity: ResMut<EditorSelected>,
    mut rotation_edit_state: ResMut<RotationEditState>,
    mut all_transforms: Query<&mut Transform>,
    mut all_global_transforms: Query<&mut GlobalTransform>,
    mut editing_name: Local<Option<Entity>>,
) {
    egui::SidePanel::right("hierarchy_panel").show(egui_ctx.ctx_mut().unwrap(), |ui| {
        ui.heading("Hierarchy");
        ui.separator();

        // Draw root-level entities
        // Determined by if they don't have "EditorChildOf" component, i.e. no (editor object) parent.
        let (_resp, root_drop) = ui.dnd_drop_zone::<Entity, ()>(
            Frame::default()
                .fill(ui.visuals().panel_fill)
                .inner_margin(0.0),
            |ui| {
                ui.set_min_width(ui.available_width());
                for (entity, name, children, _eco) in
                    query.iter().filter(|(_, _, _, eco)| eco.is_none())
                {
                    let name_str = name.map_or("no name", |n| n.as_str());
                    let input = ui.ctx().input(|i| i.clone());
                    draw_node(
                        ui,
                        &query,
                        &mut all_transforms,
                        &mut all_global_transforms,
                        entity,
                        name_str,
                        children,
                        &mut selected_entity,
                        &mut commands,
                        &mut editing_name,
                        &mut ui_buffers,
                        &input,
                    );
                }
            },
        );
        // If something was dropped, but NOT onto a node:
        if let Some(dragged_entity) = root_drop {
            // set the transform to the computed GlobalTransform, so it stays in the same spot
            if let Ok(selected_global_transform) = all_global_transforms.get(*dragged_entity) {
                if let Ok(mut selected_transform) = all_transforms.get_mut(*dragged_entity) {
                    *selected_transform = selected_global_transform.compute_transform();
                }
            }
            // Reset its hierarchy to be a Root
            commands.entity(*dragged_entity).remove::<EditorChildOf>();
            commands.entity(*dragged_entity).remove::<ChildOf>();
        }
        ui.separator();
        if let Some(selected) = selected_entity.0 {
            ui.label(format!(
                "Selected: {:?}",
                query
                    .get(selected)
                    .ok()
                    .map(|(_, n, _, _)| n.map_or("no name", |n| n.as_str()))
            ));
            if let Ok(global_transform) = all_global_transforms.get(selected) {
                if let Ok(mut transform) = all_transforms.get_mut(selected) {
                    ui.separator();
                    ui.heading("Transform");
                    // Initialize if newly selected
                    if rotation_edit_state.initial_global_transform.is_none() {
                        rotation_edit_state.initial_global_transform =
                            Some(global_transform.clone());
                        rotation_edit_state.initial_transform = Some(transform.clone());
                        let (pitch, yaw, roll) = transform.rotation.to_euler(EulerRot::XYZ);
                        rotation_edit_state.rotation_edit_euler =
                            Some((pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees()));
                        dbg!("setting from initial selection");
                        dbg!(rotation_edit_state.rotation_edit_euler);
                    }
                    let (mut delta_pitch, mut delta_yaw, mut delta_roll) =
                        rotation_edit_state.rotation_edit_euler.unwrap();

                    // --- Position ---
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.add(
                            egui::DragValue::new(&mut transform.translation.x)
                                .prefix("X: ")
                                .speed(0.1),
                        );
                        ui.add(
                            egui::DragValue::new(&mut transform.translation.y)
                                .prefix("Y: ")
                                .speed(0.1),
                        );
                        ui.add(
                            egui::DragValue::new(&mut transform.translation.z)
                                .prefix("Z: ")
                                .speed(0.1),
                        );
                    });

                    // --- Rotation ---
                    // Convert quaternion to Euler angles for editing
                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                        ui.add(
                            egui::DragValue::new(&mut delta_pitch)
                                .prefix("Pitch: ")
                                .speed(1.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut delta_yaw)
                                .prefix("Yaw: ")
                                .speed(1.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut delta_roll)
                                .prefix("Roll: ")
                                .speed(1.0),
                        );
                    });
                    apply_rotation(
                        delta_pitch,
                        delta_yaw,
                        delta_roll,
                        &mut rotation_edit_state,
                        &mut transform,
                    );

                    // --- Scale ---
                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.add(
                            egui::DragValue::new(&mut transform.scale.x)
                                .prefix("X: ")
                                .speed(0.01),
                        );
                        ui.add(
                            egui::DragValue::new(&mut transform.scale.y)
                                .prefix("Y: ")
                                .speed(0.01),
                        );
                        ui.add(
                            egui::DragValue::new(&mut transform.scale.z)
                                .prefix("Z: ")
                                .speed(0.01),
                        );
                    });
                }
            }

            // --- Parent Picker using DropDownBox ---
            let all_names: Vec<String> = query
                .iter()
                .map(|(_, name, _, _)| name.unwrap_or(&Name::new("no name")).as_str().to_owned())
                .collect();

            DropDownBox::from_iter(
                all_names.iter(),
                "parent_selector",
                &mut ui_buffers.search_buf,
                |ui, text| ui.selectable_label(false, text),
            )
            .hint_text("Select new parent...")
            .ui(ui);

            if ui.button("Set Parent").clicked() {
                if let Some(new_parent_name) =
                    all_names.iter().find(|n| **n == ui_buffers.search_buf)
                {
                    if let Some((new_parent, _, _, _)) = query.iter().find(|(_, n, _, _)| {
                        n.unwrap_or(&Name::new("no name")).as_str() == new_parent_name
                    }) {
                        if new_parent != selected {
                            commands
                                .entity(selected)
                                .insert((EditorChildOf(new_parent), ChildOf(new_parent)));
                        }
                    }
                }
            }
        }
    });
}

fn draw_node(
    ui: &mut Ui,
    query: &Query<
        (
            Entity,
            Option<&Name>,
            Option<&Children>,
            Option<&EditorChildOf>,
        ),
        With<IncludeInSave>,
    >,
    all_transforms: &mut Query<&mut Transform>,
    all_global_transforms: &mut Query<&mut GlobalTransform>,
    entity: Entity,
    name: &str,
    children: Option<&Children>,
    selected_entity: &mut EditorSelected,
    commands: &mut Commands,
    editing_name: &mut Option<Entity>,
    ui_buffers: &mut UiBuffers,
    input: &egui::InputState,
) {
    let id = Id::new(("hierarchy_node", entity));
    let frame = Frame::default().inner_margin(4.0);

    // We'll draw the entire node inside a dnd_drop_zone to accept drops.
    // The payload type is Entity.
    let (_response, dropped_payload) = ui.dnd_drop_zone::<Entity, ()>(frame, |ui| {
        // draw UI, interact, etc.
        let is_selected = selected_entity.0 == Some(entity);
        // Handle renaming
        if *editing_name == Some(entity) {
            let rename_buffer = &mut ui_buffers.rename_buf;
            let text_response = ui.text_edit_singleline(rename_buffer);
            // Commit name on Enter or unfocus
            if text_response.lost_focus() && input.key_pressed(egui::Key::Enter) {
                commands
                    .entity(entity)
                    .insert(Name::new(rename_buffer.clone()));
                rename_buffer.clear();
                *editing_name = None;
            }

            // Cancel on Escape
            if input.key_pressed(egui::Key::Escape) {
                *editing_name = None;
                rename_buffer.clear();
            }

            // Allow clicking the text edit to select as well
            if text_response.clicked() {
                selected_entity.0 = Some(entity);
            }
        } else {
            // Draw selectable label
            let label_response = ui.selectable_label(is_selected, name);

            // Single click → select
            if label_response.clicked() {
                selected_entity.0 = Some(entity);
            }

            // Double click → start renaming
            if label_response.double_clicked() {
                *editing_name = Some(entity);
            }
        }

        ui.dnd_drag_source(id, entity, |ui| {
            ui.label(format!("Moving: {}", name));
        });
    });

    // Handle drop after UI is drawn:
    if let Some(dragged_entity) = dropped_payload {
        // Prevent parenting to self or children
        if dragged_entity != entity.into() && !would_create_cycle(&query, *dragged_entity, entity) {
            set_new_relative_transform(
                all_transforms,
                all_global_transforms,
                entity,
                *dragged_entity,
            );

            commands
                .entity(*dragged_entity)
                .insert((EditorChildOf(entity), ChildOf(entity)));
        }
    }

    // Draw children recursively
    if let Some(children) = children {
        ui.indent("child_indent", |ui| {
            for child in children.iter() {
                if let Ok((e, n, c, _)) = query.get(child) {
                    let name_str = n.map_or("no name", |n| n.as_str());
                    draw_node(
                        ui,
                        query,
                        all_transforms,
                        all_global_transforms,
                        e,
                        name_str,
                        c,
                        selected_entity,
                        commands,
                        editing_name,
                        ui_buffers,
                        input,
                    );
                }
            }
        });
    }
}

fn would_create_cycle(
    query: &Query<
        (
            Entity,
            Option<&Name>,
            Option<&Children>,
            Option<&EditorChildOf>,
        ),
        With<IncludeInSave>,
    >,

    child: Entity,
    mut potential_parent: Entity,
) -> bool {
    while let Ok((_, _, _, parent_of_potential_opt)) = query.get(potential_parent) {
        if let Some(parent_of_potential) = parent_of_potential_opt {
            if parent_of_potential.0 == child {
                // Found cycle: potential_parent is a descendant of child
                return true;
            }

            potential_parent = parent_of_potential.0;
        } else {
            // No parent found; reached root without hitting child
            break;
        }
    }
    false
}
fn set_new_relative_transform(
    all_transforms: &mut Query<&mut Transform>,
    all_global_transforms: &mut Query<&mut GlobalTransform>,
    parent: Entity,
    child: Entity,
) {
    if let Ok(parent_global_transform) = all_global_transforms.get(parent) {
        if let Ok(child_global_transform) = all_global_transforms.get(child) {
            if let Ok(mut child_transform) = all_transforms.get_mut(child) {
                let new_local_matrix = parent_global_transform.to_matrix().inverse()
                    * child_global_transform.to_matrix();
                *child_transform = Transform::from_matrix(new_local_matrix);
            }
        }
    }
}
fn apply_rotation(
    new_pitch: f32,
    new_yaw: f32,
    new_roll: f32,
    rotation_edit_state: &mut RotationEditState,
    transform: &mut Transform,
) {
    let (pitch, yaw, roll) = rotation_edit_state.rotation_edit_euler.unwrap();
    let mut delta_pitch = new_pitch - pitch;
    let mut delta_yaw = new_yaw - yaw;
    let mut delta_roll = new_roll - roll;
    if delta_pitch.abs() < 0.001 {
        delta_pitch = 0.
    };
    if delta_yaw.abs() < 0.001 {
        delta_yaw = 0.
    };
    if delta_roll.abs() < 0.001 {
        delta_roll = 0.
    };
    if rotation_edit_state.rotation_edit_euler != Some((new_pitch, new_yaw, new_roll)) {
        dbg!("before rotation");
        dbg!(rotation_edit_state.rotation_edit_euler);

        let delta_rot = Quat::from_euler(
            EulerRot::XYZ,
            delta_pitch.to_radians(),
            delta_yaw.to_radians(),
            delta_roll.to_radians(),
        );
        transform.rotation = transform.rotation * delta_rot; // local
        // Set the new local Transform
        dbg!("after rotation");
        rotation_edit_state.rotation_edit_euler = Some((new_pitch, new_yaw, new_roll));
        dbg!(rotation_edit_state.rotation_edit_euler);
        let (pitch, yaw, roll) = transform.rotation.to_euler(EulerRot::XYZ);
        dbg!(pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees());
    }
}
