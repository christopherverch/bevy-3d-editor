use bevy::{
    color::palettes::tailwind::*,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass},
    light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver},
    pbr::wireframe::Wireframe,
    prelude::*,
    render::view::Hdr,
    scene::SceneInstanceReady,
    window::{CursorGrabMode, CursorOptions},
};
use std::f32::consts::FRAC_PI_4;

use crate::{
    camera::EditorCamera,
    defs::{EditorChildOf, EditorEntityLink, EditorGltfInstances, EditorMaterials, GltfEntityRoot},
    input::change_selected_entity,
    outline_material::OutlineMaterial,
    saving_loading::{load_gltf_instances, spawn_gltf_instances},
};

pub fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cursor_options: Single<&mut CursorOptions, With<Window>>,
    gltf_instances_res: ResMut<EditorGltfInstances>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 0.4;
    cursor_options.grab_mode = CursorGrabMode::None;
    commands.spawn((
        Camera3d::default(),
        EditorCamera::default(),
        Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        // MSAA needs to be off for Deferred rendering
        Msaa::Off,
        DistanceFog {
            color: Color::srgb_u8(43, 44, 47),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 2000.0,
            },
            ..default()
        },
        Hdr,
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        },
        DepthPrepass,
        MotionVectorPrepass,
        DeferredPrepass,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.0, -FRAC_PI_4)),
    ));
    let instances = load_gltf_instances("test.ron");
    spawn_gltf_instances(&mut commands, instances, gltf_instances_res, asset_server);
    //test glb
    //let test_glb = asset_server
    //    .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    //let test_glb = asset_server.load(GltfAssetLabel::Scene(0).from_asset("test2.glb"));
    //  commands.spawn((
    //      Name::new("test glb"),
    //      SceneRoot(test_glb.clone()),
    //      Transform::from_xyz(-1.0, 0.2, -1.0),
    //  ));
    //    commands.spawn((
    //        Name::new("test glb"),
    //        SceneRoot(test_glb),
    //        Transform::from_xyz(-2.0, 0.2, -1.0),
    //    ));
    //
    let sphere_h = meshes.add(Sphere::new(0.125).mesh().uv(32, 18));

    let sphere_color = Color::srgb(10.0, 4.0, 1.0);
    let sphere_pos = Transform::from_xyz(0.4, 0.5, -0.8);
    // Emissive sphere
    let mut unlit_mat: StandardMaterial = sphere_color.into();
    unlit_mat.unlit = true;
    commands.spawn((
        Mesh3d(sphere_h.clone()),
        MeshMaterial3d(materials.add(unlit_mat)),
        sphere_pos,
        NotShadowCaster,
    ));
    // Light
    commands.spawn((
        PointLight {
            intensity: 800.0,
            radius: 0.125,
            shadows_enabled: true,
            color: sphere_color,
            ..default()
        },
        sphere_pos,
    ));

    // sky
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("888888").unwrap().into(),
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(1_000_000.0)),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // Example instructions
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

pub fn detect_gltf_children(
    trigger: On<SceneInstanceReady>,
    editor_entity_link_q: Query<&EditorEntityLink>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut outline_materials: ResMut<Assets<OutlineMaterial>>,
    entities_with_children: Query<(Entity, Option<&Mesh3d>, Option<&Children>)>,
    child_gltfs: Query<Entity, With<EditorChildOf>>,
) {
    let clicked_entity = trigger.entity;
    let Ok(editor_entity_link) = editor_entity_link_q.get(clicked_entity) else {
        return;
    };
    let white_matl = materials.add(Color::WHITE);
    let hover_matl = materials.add(Color::from(CYAN_300));
    let material = OutlineMaterial {
        color: LinearRgba::new(1.0, 0.61, 0.0, 0.0),
        alpha_mode: AlphaMode::Blend,
    };
    let pressed_matl = outline_materials.add(material);
    commands.insert_resource(EditorMaterials {
        pressed_matl,
        hover_matl,
        white_matl,
    });
    let main_editor_entity = editor_entity_link.0;
    observe_entity_clicked_recursive(
        main_editor_entity,
        &entities_with_children,
        trigger.entity,
        child_gltfs,
        &mut commands,
    );
}
fn observe_entity_clicked_recursive(
    main_editor_entity: Entity,
    entities_with_children: &Query<(Entity, Option<&Mesh3d>, Option<&Children>)>,
    entity: Entity,
    child_gltfs: Query<Entity, With<EditorChildOf>>,
    commands: &mut Commands,
) {
    // Don't run for gltfs that were added as children, they will run their own observing etc setup
    if main_editor_entity != entity && child_gltfs.get(entity).is_ok() {
        return;
    }
    if let Ok((entity, mesh_opt, children_opt)) = entities_with_children.get(entity) {
        // Apply to this entity if it has a Mesh3d
        if mesh_opt.is_some() {
            commands.entity(entity).observe(change_selected_entity);
            commands
                .entity(entity)
                .insert(GltfEntityRoot(main_editor_entity));
        }

        // Recurse into children
        if let Some(children) = children_opt {
            for child in children.iter() {
                observe_entity_clicked_recursive(
                    main_editor_entity,
                    entities_with_children,
                    child,
                    child_gltfs,
                    commands,
                );
            }
        }
    }
}
