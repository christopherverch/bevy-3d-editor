use bevy::{input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};

use crate::{
    camera::{PlayerCamera, orbit_condition, orbit_mouse},
    defs::PlayerInput,
};
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, handle_mouse_input.run_if(orbit_condition))
            .add_systems(PreUpdate, handle_keyboard_input)
            .insert_resource(PlayerInput::default());
    }
}
pub fn handle_mouse_input(
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut cam_q: Query<(&PlayerCamera, &mut Transform), With<PlayerCamera>>,
    mut player_input: ResMut<PlayerInput>,
    mut mouse_evr: EventReader<MouseMotion>,
) {
    let mut rotation = Vec2::ZERO;
    for ev in mouse_evr.read() {
        rotation = ev.delta;
    }
    // This is to be reset to 0,0 every physics frame
    player_input.mouse_movement_since_physics_frame += rotation;
    orbit_mouse(&window_q, &mut cam_q, rotation);
}
fn handle_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_input: ResMut<PlayerInput>,
) {
    let mut move_direction = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        move_direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        move_direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        move_direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        move_direction.x += 1.0;
    }
    if keyboard_input.just_pressed(KeyCode::CapsLock) {
        player_input.running = !player_input.running;
    }
    player_input.move_direction = move_direction.normalize_or_zero();
    if keyboard_input.just_pressed(KeyCode::ControlLeft) {
        player_input.sneaking = !player_input.sneaking;
    }
    // do this since physics runs at 60fps but player input checks run faster
    if !player_input.requested_jump {
        player_input.requested_jump = keyboard_input.just_pressed(KeyCode::Space);
    }
}
