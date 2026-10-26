
use bevy::animation::graph::AnimationGraph;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use crate::*;

#[derive(Resource)]
pub(crate) struct SceneRenderTarget(pub(crate) Handle<Image>);

pub(crate) fn setup_common(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    _graphs: ResMut<Assets<AnimationGraph>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    player_model_tuning: Res<PlayerModelTuning>,
    render_resolution_scale: Res<RenderResolutionScale>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let (window_width, window_height) = windows
        .single()
        .map(|window| (window.resolution.physical_width(), window.resolution.physical_height()))
        .unwrap_or((1280, 720));
    let mut scene_image = Image::new_fill(
        scaled_scene_extent(window_width, window_height, render_resolution_scale.0),
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    scene_image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let scene_image_handle = images.add(scene_image);
    commands.insert_resource(SceneRenderTarget(scene_image_handle.clone()));

    commands.spawn((Camera2d, IsDefaultUiCamera));
    // MAP_ID means a measurement or a debug run, which goes straight into the
    // map. Two seconds of branding in front of that is two seconds of waiting
    // on every single run.
    if game_flow::map_id_override().is_none() {
        spawn_splash_screen(&mut commands, &asset_server);
    }

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::new(scene_image_handle),
        Pickable::IGNORE,
    ));

    commands.insert_resource(HitMarkerAssets {
        mesh: meshes.add(Sphere::new(0.03)),
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(4.0, 0.3, 0.2),
            unlit: true,
            ..default()
        }),
    });

    // One handle per weapon, in WeaponId order, so a weapon that gets its own
    // recording only has to say so in the table.
    commands.insert_resource(GunshotSound(
        WeaponId::ALL.iter().map(|w| asset_server.load(w.def().sound.fire)).collect(),
    ));
    commands.insert_resource(ReloadSound(
        WeaponId::ALL.iter().map(|w| asset_server.load(w.def().sound.reload)).collect(),
    ));
    commands.insert_resource(FootstepSounds {
        player: (0..PLAYER_FOOTSTEP_SAMPLES)
            .map(|i| asset_server.load(format!("audio/footsteps/{PLAYER_FOOTSTEP_SURFACE}/{i}.ogg")))
            .collect(),
        zombie: (0..ZOMBIE_FOOTSTEP_SAMPLES)
            .map(|i| asset_server.load(format!("audio/footsteps/{ZOMBIE_FOOTSTEP_SURFACE}/{i}.ogg")))
            .collect(),
    });

    commands.insert_resource(MuzzleFxAssets {
        flash_mesh: meshes.add(Rectangle::new(FLASH_QUAD_SIZE, FLASH_QUAD_SIZE)),
        ember_mesh: meshes.add(Rectangle::new(MUZZLE_EMBER_SIZE, MUZZLE_EMBER_SIZE)),
        smoke_mesh: meshes.add(Rectangle::new(0.25, 0.25)),
        smoke_atlas: asset_server.load(SMOKE_ATLAS_PATH),
    });

    commands.insert_resource(TracerAssets {
        mesh: meshes.add(spindle_mesh(TRACER_SEGMENTS)),
        fade_steps: (0..TRACER_FADE_STEPS)
            .map(|step| {
                let fade = 1.0 - step as f32 / TRACER_FADE_STEPS as f32;
                materials.add(StandardMaterial {
                    base_color: Color::LinearRgba(LinearRgba::new(10.0, 6.0, 1.0, fade)),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
    });

    let barrel_ribbon = meshes.add(empty_ribbon_mesh());
    commands.insert_resource(BarrelSmokeRibbon { mesh: barrel_ribbon.clone() });
    commands.spawn((
        Name::new("BarrelSmokeRibbon"),
        Mesh3d(barrel_ribbon),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.70, 0.69, 0.67),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
    ));

    commands.insert_resource(build_bullet_hole_assets(&asset_server, &mut meshes, &mut materials));
    commands.insert_resource(build_casing_assets(&asset_server, &mut meshes, &mut materials));
    commands.insert_resource(build_gore_assets(&asset_server));

    commands.insert_resource(BloodAssets {
        particle_mesh: meshes.add(Rectangle::new(BLOOD_PARTICLE_SIZE, BLOOD_PARTICLE_SIZE)),
        particle_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.05, 0.04),
            base_color_texture: Some(images.add(droplet_image(BLOOD_DROPLET_TEXTURE_SIZE))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        stain_mesh: meshes.add(Rectangle::new(BLOOD_STAIN_SIZE, BLOOD_STAIN_SIZE)),
        floor_stain_mesh: meshes.add(Rectangle::new(BLOOD_FLOOR_STAIN_SIZE, BLOOD_FLOOR_STAIN_SIZE)),
        stain_materials: (0..BLOOD_STAIN_VARIANTS)
            .map(|i| {
                materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.42, 0.38, 0.92),
                    base_color_texture: Some(asset_server.load(format!("textures/blood/splatter_{i}.png"))),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: 0.12,
                    metallic: 0.0,
                    reflectance: 0.75,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
        wall_stain_mesh: meshes.add(Rectangle::new(BLOOD_WALL_STAIN_SIZE, BLOOD_WALL_STAIN_SIZE)),
        burst_mesh: meshes.add(Rectangle::new(BLOOD_BURST_SIZE, BLOOD_BURST_SIZE)),
        burst_materials: (0..BLOOD_STAIN_VARIANTS)
            .map(|i| {
                let base = StandardMaterial {
                    base_color: Color::srgba(1.0, 0.16, 0.12, 1.0),
                    base_color_texture: Some(asset_server.load(format!("textures/blood/splatter_{i}.png"))),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                };
                fade_ladder(&mut materials, base, BLOOD_BURST_FADE_STEPS)
            })
            .collect(),
    });

    commands.insert_resource(SparkAssets {
        mesh: meshes.add(Rectangle::new(SPARK_PARTICLE_SIZE, SPARK_PARTICLE_SIZE)),
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(25.0, 12.0, 1.5),
            base_color_texture: Some(images.add(droplet_image(BLOOD_DROPLET_TEXTURE_SIZE))),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });

    commands.insert_resource(load_zombie_anim_gltfs(&asset_server, &player_model_tuning));
    commands.insert_resource(GameOver(false));
    // Everything now goes through the loading gate; it remembers where it was
    // headed.
    commands.insert_resource(AfterLoading(if game_flow::map_id_override().is_some() {
        AppState::InGame
    } else {
        AppState::Menu
    }));
    next_state.set(AppState::Loading);
}

fn review_yaw_pitch() -> (f32, f32) {
    (0.0, 0.0)
}



fn push_bar(positions: &mut Vec<[f32; 3]>, indices: &mut Vec<u32>, cx: f32, cy: f32, hw: f32, hh: f32) {
    let base = positions.len() as u32;
    positions.extend_from_slice(&[
        [cx - hw, cy - hh, 0.0],
        [cx + hw, cy - hh, 0.0],
        [cx + hw, cy + hh, 0.0],
        [cx - hw, cy + hh, 0.0],
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Four posts thickening toward the edge, a hairline cross in the middle and
/// ranging dots along it: a duplex, the mark an M24 carries.
fn duplex_reticle_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let reach = SCOPE_RETICLE_REACH;
    let taper = reach * SCOPE_RETICLE_TAPER;
    let gap = reach * SCOPE_RETICLE_CENTRE_GAP;
    let post = SCOPE_RETICLE_POST_WIDTH / 2.0;
    let line = SCOPE_RETICLE_LINE_WIDTH / 2.0;
    let dot = SCOPE_RETICLE_DOT_SIZE / 2.0;

    for side in [-1.0f32, 1.0] {
        let post_mid = side * (taper + reach) / 2.0;
        let post_half = (reach - taper) / 2.0;
        let line_mid = side * (gap + taper) / 2.0;
        let line_half = (taper - gap) / 2.0;

        push_bar(&mut positions, &mut indices, post_mid, 0.0, post_half, post);
        push_bar(&mut positions, &mut indices, 0.0, post_mid, post, post_half);
        push_bar(&mut positions, &mut indices, line_mid, 0.0, line_half, line);
        push_bar(&mut positions, &mut indices, 0.0, line_mid, line, line_half);

        // Spaced along the fine section only, as on the real glass: dots out
        // on the thick posts would be swallowed by them.
        let step = (taper - gap) / (SCOPE_RETICLE_DOTS_PER_ARM + 1) as f32;
        for i in 1..=SCOPE_RETICLE_DOTS_PER_ARM {
            let at = side * (gap + step * i as f32);
            push_bar(&mut positions, &mut indices, at, 0.0, dot, dot);
            push_bar(&mut positions, &mut indices, 0.0, at, dot, dot);
        }
    }

    let count = positions.len();
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

pub(crate) fn setup_gameplay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut overlay_materials: ResMut<Assets<DamageOverlayMaterial>>,
    touch_enabled: Res<TouchControlsEnabled>,
    scene_target: Option<Res<SceneRenderTarget>>,
    mut pending: ResMut<MapSelectionPending>,
    active: Res<ActiveWeapon>,
    cascades: Res<ShadowCascades>,
) {
    let Some(scene_target) = scene_target else {
        return;
    };
    pending.0 = false;
    let scenario = current_scenario();
    let player_spawn = scenario.player_spawn();

    commands.spawn((
        Name::new(scenario.title()),
        crate::map::MapRoot,
        // The map keeps its CPU meshes: Avian builds every trimesh collider from
        // them, and world_ready is the signal that it has finished.
        WorldAssetRoot(load_gltf_scene(&asset_server, scenario.require_gltf_scene_path(), RenderAssetUsages::default())),
        Transform::IDENTITY,
        shared::gltf_scene_colliders(),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: if dynamic_lights_enabled() { MAIN_SUN_ILLUMINANCE } else { 0.0 },
            shadow_maps_enabled: light_shadows_enabled(),
            ..default()
        },
        cascade_config(&cascades),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.5, 0.0)),
        MainSun,
    ));

    commands.spawn((
        Name::new("Player"),
        Transform::from_translation(player_spawn),
        Player {
            horizontal_velocity: Vec3::ZERO,
            ground_normal: Vec3::Y,
            vertical_velocity: 0.0,
            grounded: false,
        },
        Health(PLAYER_MAX_HEALTH),
        PlayerStance::default(),
        crate::sim::PlayerThreat::default(),
    ));

    let (review_yaw, review_pitch) = review_yaw_pitch();
    let camera = commands
        .spawn((
            Name::new("FpsCamera (first-person, main render camera)"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: BASE_FOV,
                ..default()
            }),
            Transform::from_translation(player_spawn + Vec3::Y * EYE_HEIGHT_OFFSET),
            bevy_seedling::prelude::SpatialListener3D,
            FpsCamera { yaw: review_yaw, pitch: review_pitch },
            ViewRecoil::default(),
            Lean::default(),
            AmbientLight {
                color: Color::WHITE,
                brightness: ambient_brightness(),
                ..default()
            },
            (
                Bloom::default(),
                Msaa::Off,
                bevy::core_pipeline::prepass::DepthPrepass,
            ),
            RenderLayers::from_layers(&[WORLD_LAYER, VIEWMODEL_LAYER]),
            RenderTarget::from(scene_target.0.clone()),
        ))
        .id();

    let gun_entity = commands
        .spawn((
            Name::new("Gun (first-person viewmodel)"),
            Transform::from_translation(active.def().rest_offset),
            Visibility::default(),
            Gun {
                optical_fix: Quat::IDENTITY,
                ads_from_ready: 0.0,
                ads_wanted: false,
                base_offset: active.def().rest_offset,
                muzzle_local: Transform::IDENTITY,
                eject_local: Transform::IDENTITY,
                settle_jitter: Vec3::ZERO,
                settle_current: Vec3::ZERO,
                settle_twist_jitter: 0.0,
                settle_twist_current: 0.0,
                settle_rolled: false,
                recoil: 0.0,
                recoil_target: 0.0,
                ammo: active.def().magazine,
                reload_remaining: 0.0,
                ads_blend: 0.0,
                ads_velocity: 0.0,
                ads_settle_error: 0.0,
                wall_raise: 0.0,
                wall_near: false,
                shoulder_blend: 0.0,
                ready_blend: 0.0,
                ready_armed: false,
                sprint_blend: 0.0,
                since_shot: f32::MAX,
                idle_elapsed: 0.0,
                inspect_t: 0.0,
                inspect_blend: 0.0,
                inspect_done: 0,
                inspect_offset: Vec3::ZERO,
                inspect_offset_vel: Vec3::ZERO,
                inspect_angles: Vec3::ZERO,
                inspect_angles_vel: Vec3::ZERO,
                recoil_jitter: Vec2::ZERO,
                // Off the wall clock, or every session sprays its first
                // magazine the same way.
                recoil_rng: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0x9E37_79B9, |since| since.subsec_nanos())
                    | 1,
                look_sway: Vec2::ZERO,
                step_lean: Vec2::ZERO,
                look_sway_velocity: Vec2::ZERO,
            },
            ChildOf(camera),
        ))
        .id();
    spawn_gun_mesh_parts(&mut commands, &asset_server, &mut materials, gun_entity, active.def().mesh_path);

    // Unlit and biased forward, so the dot stays a mark on the world rather
    // than a lit object standing in it, and nothing the weapon carries can
    // land in front of it.
    let mark_material = |colour: Color| StandardMaterial {
        base_color: colour,
        unlit: true,
        depth_bias: 1000.0,
        cull_mode: None,
        ..default()
    };
    // Where the left hand is sent while the weapon is being reloaded. Off the
    // camera, so the waypoints `drive_reload_hand` writes are already in the
    // frame the solver wants.
    commands.spawn((
        Name::new("ReloadHandTarget"),
        Transform::default(),
        ReloadHandTarget,
        ChildOf(camera),
    ));

    // The holographic dot, on the main view rather than the scope's. A 1x
    // sight has nothing to magnify, so it earns neither a second camera nor a
    // render target -- it is one quad hung off the eye and aimed down the
    // barrel, the same way the scope mark is, and `place_holo_reticle` turns
    // it off when the eye leaves the window.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(HOLO_DOT_RADIANS).mesh())),
        MeshMaterial3d(materials.add(mark_material(Color::linear_rgb(28.0, 0.6, 0.2)))),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(VIEWMODEL_LAYER),
        Reticle(ReticleKind::Dot),
        ChildOf(camera),
    ));

    // The scope's own mark, hung off the eye exactly like the dot.
    commands.spawn((
        Mesh3d(meshes.add(duplex_reticle_mesh())),
        MeshMaterial3d(materials.add(mark_material(Color::BLACK))),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(VIEWMODEL_LAYER),
        Reticle(ReticleKind::Duplex),
        ChildOf(camera),
        // The red centre sits in the duplex's gap. A child, so it is carried
        // and hidden with the cross; render layers do not propagate, hence it
        // names its own.
        children![(
            Mesh3d(meshes.add(Circle::new(SCOPE_RETICLE_CENTRE_DOT_SIZE / 2.0).mesh())),
            MeshMaterial3d(materials.add(mark_material(Color::linear_rgb(35.0, 0.0, 0.0)))),
            Transform::default(),
            RenderLayers::layer(VIEWMODEL_LAYER),
        )],
    ));

    spawn_crosshair(&mut commands);
    spawn_health_ui(&mut commands);
    spawn_kill_ui(&mut commands);
    spawn_ammo_ui(&mut commands);
    spawn_compass_ui(&mut commands);
    spawn_radar_ui(&mut commands);
    spawn_damage_overlay(&mut commands, &mut overlay_materials);
    spawn_settings_menu(&mut commands);
    spawn_settings_toggle_button(&mut commands);
    spawn_mouse_ui_lock_indicator(&mut commands);
    if touch_enabled.0 {
        spawn_touch_controls(&mut commands);
    }
}

