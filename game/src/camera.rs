use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow, Window};
use bevy_rapier3d::plugin::PhysicsSet;

use crate::const_defs::FIRST_PERSON_CAMERA_HEIGHT_OFFSET;

pub struct PlayerCameraPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraSyncSet;
#[derive(Event)]
pub struct ToggleCameraMode;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ToggleCameraMode>()
            .configure_sets(PostUpdate, CameraSyncSet.after(PhysicsSet::StepSimulation)) // DO THIS!
            .add_systems(Update, toggle_cursor.run_if(toggle_cursor_condition))
            .add_systems(
                PostUpdate,
                sync_camera_and_player
                    .before(TransformSystem::TransformPropagate)
                    .in_set(CameraSyncSet),
            );
    }
}
#[derive(Component)]
pub struct PlayerCameraTarget;
#[derive(Component)]
pub struct PlayerCamera {
    /// Flag to indicate if the cursor lock toggle functionality is turned on.
    /// When enabled and the cursor lock is NOT active, the mouse can freely move about the window without the camera's transform changing.
    /// Example usage: Browsing a character inventory without moving the camera.
    /// Default is true
    pub cursor_lock_toggle_enabled: bool,
    /// Flag to indicate if the cursor is in a locked state or not.
    /// Default is true
    pub cursor_lock_active: bool,
    /// The cursor lock toggle key binding.
    /// Default is KeyCode::Space
    pub cursor_lock_key: KeyCode,
    /// Mouse x/y sensitivity
    /// Default is Vec2::new(1.0, 1.0)
    pub sensitivity: Vec2,
}

impl Default for PlayerCamera {
    fn default() -> Self {
        PlayerCamera {
            cursor_lock_key: KeyCode::Escape,
            cursor_lock_toggle_enabled: true,
            cursor_lock_active: true,
            sensitivity: Vec2::new(0.86, 0.86),
        }
    }
}
// Set the camera's translation to the player's translation every frame.
// We do this instead of parenting so the rotation doesn't get messed up
// if the player is rotated
fn sync_camera_and_player(
    player_q: Query<&Transform, With<PlayerCameraTarget>>,
    mut cam_q: Query<&mut Transform, (With<PlayerCamera>, Without<PlayerCameraTarget>)>,
) {
    let Ok(player_transform) = player_q.single() else {
        return;
    };
    let Ok(mut cam_transform) = cam_q.single_mut() else {
        return;
    };
    cam_transform.translation =
        player_transform.translation + Vec3::new(0.0, FIRST_PERSON_CAMERA_HEIGHT_OFFSET, 0.0);
}

// heavily referenced https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera.html
pub fn orbit_mouse(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    cam_q: &mut Query<(&PlayerCamera, &mut Transform), With<PlayerCamera>>,
    mut mouse_delta: Vec2,
) {
    let Ok((cam, mut cam_transform)) = cam_q.single_mut() else {
        return;
    };

    mouse_delta *= cam.sensitivity;

    if mouse_delta.length_squared() > 0.0 {
        let window = window_q.single().unwrap();
        let delta_x = mouse_delta.x / window.width() * PI * cam.sensitivity.x;
        let delta_y = mouse_delta.y / window.height() * PI * cam.sensitivity.y;
        let yaw = Quat::from_rotation_y(-delta_x);

        cam_transform.rotation = yaw * cam_transform.rotation;

        let pitch = Quat::from_rotation_x(-delta_y);
        let new_rotation = cam_transform.rotation * pitch;

        let up_vector = new_rotation * Vec3::Y;
        if up_vector.y < 0.2 {
            // Calculate the desired pitch to clamp up_vector.y at 0.2
            let current_up = cam_transform.rotation * Vec3::Y;
            let target_up = Vec3::new(current_up.x, 0.2, current_up.z).normalize(); //clamp y to 0.2

            // Use from_rotation_arc instead of from_two_vectors
            let rotation_to_target = Quat::from_rotation_arc(current_up, target_up);

            // Apply only the necessary pitch to reach the target
            cam_transform.rotation = rotation_to_target * cam_transform.rotation;
        } else {
            cam_transform.rotation = new_rotation; // Apply the original new rotation
        }
    }
}
// only run the orbit system if the cursor lock is disabled
pub fn orbit_condition(cam_q: Query<&PlayerCamera>) -> bool {
    let Ok(cam) = cam_q.single() else {
        return true;
    };
    return cam.cursor_lock_active;
}

fn toggle_cursor(
    mut cam_q: Query<&mut PlayerCamera>,
    keys: Res<ButtonInput<KeyCode>>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut cam) = cam_q.single_mut() else {
        return;
    };

    if keys.just_pressed(cam.cursor_lock_key) {
        cam.cursor_lock_active = !cam.cursor_lock_active;
    }

    if let Ok(mut window) = window_q.single_mut() {
        if cam.cursor_lock_active {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

// checks if the toggle cursor functionality is enabled
fn toggle_cursor_condition(cam_q: Query<&PlayerCamera>) -> bool {
    let Ok(cam) = cam_q.single() else {
        return true;
    };
    cam.cursor_lock_toggle_enabled
}
