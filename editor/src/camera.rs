use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window};

use crate::input::EditorInput;

pub struct CameraPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraSyncSet;
#[derive(Event)]
struct ToggleCursor;
#[derive(Resource)]
struct CameraController {
    yaw: f32,
    pitch: f32,
    distance: f32,
    focus: Vec3,
    last_mouse_position: Option<Vec2>,
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_moving_camera)
            .add_systems(Update, toggle_cursor);
    }
}
#[derive(Component)]
pub struct EditorCameraTarget;
#[derive(Component)]
pub struct EditorCamera {
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
    pub cursor_lock_keys: Vec<KeyCode>,
    /// Mouse x/y sensitivity
    /// Default is Vec2::new(1.0, 1.0)
    pub sensitivity: Vec3,
    pub move_speed: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        EditorCamera {
            cursor_lock_keys: vec![KeyCode::Backquote, KeyCode::ShiftLeft],
            cursor_lock_toggle_enabled: true,
            cursor_lock_active: false,
            sensitivity: Vec3::new(0.86, 0.86, 0.86),
            move_speed: 0.05,
        }
    }
}
pub fn handle_moving_camera(
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut cam_with_transform: Single<(&EditorCamera, &mut Transform), With<EditorCamera>>,
    input: Res<EditorInput>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    let (cam, cam_transform) = &mut *cam_with_transform;
    if cam.cursor_lock_active {
        // wasd movement
        orbit_mouse(&window_q, &cam, cam_transform, input.mouse_movement);
        move_camera_wasd(keyboard_input, cam, cam_transform)
    } else {
        // Regular mouse movement
        if mouse_buttons.pressed(MouseButton::Middle) {
            if keyboard_input.pressed(KeyCode::ShiftLeft) {
                move_camera(&window_q, &cam, cam_transform, input.mouse_movement);
            } else {
                orbit_mouse(&window_q, &cam, cam_transform, input.mouse_movement);
            }
        } else {
            scroll_camera(&cam, cam_transform, input.mouse_scroll);
        }
    }
}
pub fn toggle_cursor_condition(cam_q: Query<&EditorCamera>) -> bool {
    let Ok(cam) = cam_q.single() else {
        return true;
    };
    cam.cursor_lock_toggle_enabled
}

pub fn toggle_cursor(
    mut cam_q: Query<&mut EditorCamera>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_options: Single<&mut CursorOptions, With<Window>>,
) {
    let Ok(mut cam) = cam_q.single_mut() else {
        return;
    };
    let any_just_pressed = cam
        .cursor_lock_keys
        .iter()
        .copied()
        .any(|key| keys.just_pressed(key));
    if !any_just_pressed {
        return;
    }
    if keys.all_pressed(cam.cursor_lock_keys.iter().copied()) {
        cam.cursor_lock_active = !cam.cursor_lock_active;
    }

    if cam.cursor_lock_active {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    } else {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}
pub fn move_camera(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    cam: &EditorCamera,
    cam_transform: &mut Transform,
    mouse_delta: Vec2,
) {
    let window = window_q.single().unwrap();
    dbg!(mouse_delta);

    // Pan speed scaled by window size and sensitivity
    let pan_x = mouse_delta.x / window.width() as f32 * cam.sensitivity.x * 10.0;
    let pan_y = mouse_delta.y / window.height() as f32 * cam.sensitivity.y * 10.0;
    dbg!(pan_x);
    // Local axes
    let right = cam_transform.rotation * Vec3::X;
    let up = cam_transform.rotation * Vec3::Y;

    // Pan camera
    cam_transform.translation += -right * pan_x + up * pan_y;
    dbg!(cam_transform.translation);
}
pub fn move_camera_wasd(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    cam: &EditorCamera,
    cam_transform: &mut Transform,
) {
    // Camera movement speed, you can make this a field of EditorCamera if you want
    let mut speed = cam.move_speed;
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        speed *= 2.0
    }

    // Local axes
    let forward = cam_transform.rotation * Vec3::NEG_Z; // Assuming -Z is forward
    let right = cam_transform.rotation * Vec3::X;

    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += right;
    }

    if direction != Vec3::ZERO {
        direction = direction.normalize();
        cam_transform.translation += direction * speed;
    }
}
pub fn scroll_camera(cam: &EditorCamera, cam_transform: &mut Transform, scroll_delta: f32) {
    let forward = cam_transform.rotation * Vec3::NEG_Z;
    cam_transform.translation += forward * scroll_delta * cam.sensitivity.z;
}

// heavily referenced https://bevy-cheatbook.github.io/cookbook/pan-orbit-camera.html
pub fn orbit_mouse(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    cam: &EditorCamera,
    cam_transform: &mut Transform,
    mut mouse_delta: Vec2,
) {
    mouse_delta.x *= cam.sensitivity.x;
    mouse_delta.y *= cam.sensitivity.y;

    if mouse_delta.length_squared() > 0.0 {
        let window = window_q.single().unwrap();
        let delta_x = mouse_delta.x / window.width() * std::f32::consts::PI * cam.sensitivity.x;
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
