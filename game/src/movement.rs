use crate::{
    const_defs::{GRAVITY, GROUND_TIMER, JUMP_SPEED},
    defs::PlayerInput,
    events::{SoundEvent, SoundSource},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{KinematicCharacterController, KinematicCharacterControllerOutput};

pub fn player_movement(
    mut player_input: ResMut<PlayerInput>,
    time: Res<Time<Fixed>>,
    mut player_query: Query<(
        Entity,
        &mut KinematicCharacterController,
        Option<&KinematicCharacterControllerOutput>,
    )>,
    mut commands: Commands,
    cam_q: Query<&Transform, With<Camera3d>>,
    mut vertical_movement: Local<f32>,
    mut grounded_timer: Local<f32>,
    mut was_grounded: Local<bool>,
) {
    for (player_entity, mut controller, output) in player_query.iter_mut() {
        let delta_time = time.delta_secs();
        let mut final_movement = Vec3::ZERO;
        if let Ok(camera_transform) = cam_q.single() {
            let forward = Vec3::new(
                camera_transform.forward().x,
                0.0,
                camera_transform.forward().z,
            )
            .normalize_or_zero();
            let right = Vec3::new(camera_transform.right().x, 0.0, camera_transform.right().z)
                .normalize_or_zero();

            // This is the standard physics-driven movement calculation
            let direction =
                forward * player_input.move_direction.y + right * player_input.move_direction.x;
            final_movement = direction.normalize_or_zero() * get_current_speed() * delta_time;
        }
        if output.map(|o| o.grounded).unwrap_or(false) {
            *grounded_timer = GROUND_TIMER;
            *vertical_movement = 0.0;
            if !*was_grounded {
                *was_grounded = true;
                let sound_event = SoundEvent {
                    sound_path: "sounds/bodyfallMED.wav".to_string(),
                    sound_source: SoundSource::LocalPlayer,
                };
                commands.trigger(sound_event.clone());
            }
        } else {
            player_input.requested_jump = false;
            *was_grounded = false;
        }
        // This allows for coyote time jumps.
        if *grounded_timer > 0.0 {
            *grounded_timer -= delta_time;
            if player_input.requested_jump {
                *vertical_movement = JUMP_SPEED;
                *grounded_timer = 0.0;
            }
        }

        *vertical_movement = *vertical_movement;
        // Lock max fall speed
        if *vertical_movement < -0.35 {
            *vertical_movement = -0.35;
        }
        final_movement.y = *vertical_movement;
        *vertical_movement += GRAVITY * delta_time * controller.custom_mass.unwrap_or(1.0);
        controller.translation = Some(final_movement);
        // Reset the per-frame rotation delta so we know how much we turned since the last physics frame
        player_input.mouse_movement_since_physics_frame = Vec2::ZERO;
    }
}
fn get_current_speed() -> f32 {
    // just return 1.0 for now
    4.0
}
