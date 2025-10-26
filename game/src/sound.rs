use crate::events::{SoundEvent, SoundSource};
use bevy::prelude::*;
use bevy_kira_audio::{
    Audio, AudioControl, AudioInstance, EmitterSettings, PlaybackState, SpatialAudioEmitter,
};
use kira::Easing;
#[derive(Component)]
pub struct DespawnAfterAudio {
    instance: Handle<AudioInstance>,
}

/// Reacts to SoundEvent triggers and spawns a sound either attached to the player, an entity or detached
pub fn generate_sound(
    trigger: Trigger<SoundEvent>,
    mut commands: Commands,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    spatial_emitters_q: Query<(&SpatialAudioEmitter, &EmitterSettings)>,
) {
    // Set the sound emitting entity to either the one given by the trigger, or a new entity we
    // spawn with the sound trigger's translation
    let mut spawned_temporary_sound_entity = false;
    let sound_source_entity = match trigger.sound_source {
        SoundSource::Attached(entity) => Some(entity),
        SoundSource::Located(translation) => {
            let detached_entity = commands
                .spawn((Transform {
                    translation,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::splat(1.0),
                },))
                .id();
            spawned_temporary_sound_entity = true;
            Some(detached_entity)
        }
        // Only play the audio spatially if it's not the local player making the sound
        SoundSource::LocalPlayer => None,
    };
    if let Some(sound_entity) = sound_source_entity {
        // If the attached entity doesn't have spatial audio components, add them
        if spatial_emitters_q.get(sound_entity).is_err() {
            commands.entity(sound_entity).insert((
                SpatialAudioEmitter::default(),
                EmitterSettings {
                    distances: (2.0..=14.0).into(),
                    attenuation_function: Easing::Linear,
                },
            ));
        }
        let instance_handle = audio
            .play(asset_server.load(trigger.sound_path.clone()))
            .with_emitter(sound_entity)
            .with_volume(-10.0)
            .handle();
        if spawned_temporary_sound_entity {
            commands.entity(sound_entity).insert(DespawnAfterAudio {
                instance: instance_handle,
            });
        }
    } else {
        audio
            .play(asset_server.load(trigger.sound_path.clone()))
            .with_volume(-10.0);
    }
}
pub fn cleanup_finished_audio(
    mut commands: Commands,
    audio_instances: Res<Assets<AudioInstance>>,
    query: Query<(Entity, &DespawnAfterAudio)>,
) {
    for (entity, despawn) in query.iter() {
        if let Some(instance) = audio_instances.get(&despawn.instance) {
            if instance.state() == PlaybackState::Stopped {
                commands.entity(entity).despawn();
            }
        }
    }
}
