use bevy::{
    ecs::{entity::Entity, event::Event},
    math::Vec3,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SoundSource {
    Attached(Entity),
    Located(Vec3),
    LocalPlayer,
}
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct SoundEvent {
    pub sound_source: SoundSource,
    pub sound_path: String,
}
