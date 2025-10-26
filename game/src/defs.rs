use std::collections::HashMap;

use bevy::{
    asset::Handle,
    ecs::{component::Component, resource::Resource},
    gltf::Gltf,
    math::Vec2,
    scene::Scene,
};

#[derive(Debug, Resource, Default)]
pub struct PlayerInput {
    pub mouse_movement_since_physics_frame: Vec2,
    pub move_direction: Vec2,
    pub running: bool,
    pub sneaking: bool,
    pub requested_jump: bool,
}
#[derive(Default, Resource)]
pub struct GameAssets {
    pub gltf_files: HashMap<String, Handle<Gltf>>,
}
#[derive(Component)]
pub struct LevelSpawner(pub Handle<Scene>);

#[derive(Component)]
pub struct AwaitingTransformPropagation;
