use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use bevy::{asset::ron, prelude::*, tasks::IoTaskPool};

use crate::{
    defs::{
        EditorChildOf, EditorEntityLink, EditorGltfInstance, EditorGltfInstances,
        FinishedGltfRefLoading, GltfRef, IncludeInSave, InstantiatedGltfInstance,
    },
    helper_funcs::strip_assets_prefix,
};
pub fn save_scene_system(world: &mut World) {
    let type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = type_registry.read();
    let entities_to_save: Vec<Entity> = world
        .query::<(Entity, &IncludeInSave)>()
        .iter(world)
        .map(|(entity, _)| entity)
        .collect();
    // 2. Create the DynamicSceneBuilder
    let scene_builder =
        DynamicSceneBuilder::from_world(world).extract_entities(entities_to_save.into_iter());

    // 4. Optionally: Add a component filter (see C below)

    let scene = scene_builder.build();

    // 5. Serialize and save to a file
    match scene.serialize(&type_registry) {
        Ok(ron) => {
            // In a real editor, you'd save this string to a file (e.g., using a non-blocking task)
            println!("Serialized Scene:\n{}", ron);
            IoTaskPool::get()
                .spawn(async move {
                    // Write the scene RON data to file
                    File::create(format!("assets/{}", SCENE_FILE_PATH))
                        .and_then(|mut file| file.write(ron.as_bytes()))
                        .expect("Error while writing scene to file");
                })
                .detach();
        }
        Err(e) => {
            eprintln!("Error serializing scene: {}", e);
        }
    }
}
const SCENE_FILE_PATH: &str = "test_saving_dynamicscene.scn.ron";

pub fn load_scene_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DynamicSceneRoot(asset_server.load(SCENE_FILE_PATH)));
}
pub fn finish_loading_scene(
    unloaded_gltf_refs: Query<(Entity, &GltfRef), Without<FinishedGltfRefLoading>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (editor_entity, gltf_ref) in unloaded_gltf_refs {
        let gltf =
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(gltf_ref.asset_path.clone()));
        let _gltf_entity = commands
            .spawn((
                SceneRoot(gltf),
                EditorEntityLink(editor_entity),
                ChildOf(editor_entity),
            ))
            .id();
        commands
            .entity(editor_entity)
            .insert(FinishedGltfRefLoading);
    }
}

pub fn load_gltf_instances(ron_path: &str) -> Vec<EditorGltfInstance> {
    let file_path = format!("{}/assets/{}", env!("CARGO_MANIFEST_DIR"), ron_path);
    let data = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read RON file {}: {}", file_path, e));

    ron::from_str(&data).expect("Failed to parse RON file")
}
pub fn spawn_gltf_instances(
    commands: &mut Commands,
    instances: Vec<EditorGltfInstance>,
    mut gltf_instances: ResMut<EditorGltfInstances>,
    asset_server: Res<AssetServer>,
) {
    for (index, instance) in instances.into_iter().enumerate() {
        let EditorGltfInstance {
            path,
            transform,
            parent,
        } = instance;
        let path = Path::new(&path);
        match strip_assets_prefix(path) {
            Some(relative_path) => {
                println!("Relative path inside assets: {}", relative_path.display());
                let editor_entity = commands
                    .spawn((
                        Name::new(format!("test glb {}", index)),
                        GltfRef {
                            asset_path: relative_path.to_string_lossy().to_string(),
                            label: None,
                        },
                        Transform::from(transform),
                        Visibility::Visible,
                        IncludeInSave,
                    ))
                    .id();

                gltf_instances.0.push(InstantiatedGltfInstance {
                    path: path.to_string_lossy().to_string(),
                    entity: editor_entity,
                    parent,
                });
            }
            None => {
                eprintln!("Selected file is not inside the assets folder!");
            }
        }
    }
    apply_gltf_hierarchy(commands, gltf_instances);
}
pub fn apply_gltf_hierarchy(commands: &mut Commands, gltf_instances: ResMut<EditorGltfInstances>) {
    for (i, instance) in gltf_instances.0.iter().enumerate() {
        if let Some(parent_index) = instance.parent {
            dbg!(parent_index);
            let child = instance.entity;
            dbg!(child);
            if let Some(parent_instance) = gltf_instances.0.get(parent_index) {
                dbg!(parent_instance.entity);
                let parent = parent_instance.entity;
                commands.entity(parent).add_child(child);
                commands.entity(child).insert(EditorChildOf(parent));
            } else {
                eprintln!("Invalid parent index {} for instance {}", parent_index, i);
            }
        }
    }
}
