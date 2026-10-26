
use bevy::prelude::*;
use bevy::animation::{AnimatedBy, AnimationTargetId};
use crate::*;

#[derive(Resource, Default)]
pub(crate) struct ZombieAnimDisabled(pub(crate) bool);

#[derive(Resource, Default)]
pub(crate) struct ZombieHealthBarsHidden(pub(crate) bool);

#[derive(Component)]
pub(crate) struct PlayerModelAnimRoot;

#[derive(Component)]
pub(crate) struct PlayerModelAnimationPlayer(pub(crate) Entity);

#[derive(Component, Default)]
pub(crate) struct PlayerModelSkeletonBones {
    pub(crate) left_upper_leg: Option<Entity>,
    pub(crate) right_upper_leg: Option<Entity>,
    pub(crate) right_arm: Option<Entity>,
    pub(crate) right_fore_arm: Option<Entity>,
    pub(crate) right_hand: Option<Entity>,
    pub(crate) left_arm: Option<Entity>,
    pub(crate) left_fore_arm: Option<Entity>,
    pub(crate) left_hand: Option<Entity>,
    pub(crate) head: Option<Entity>,
    pub(crate) hips: Option<Entity>,
}

#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub(crate) enum PlayerModelPose {
    #[default]
    Idle,
    Walk,
    Run,
}


#[derive(Component)]
pub(crate) struct ZombieHealthBar {
    zombie: Entity,
}

#[derive(Component)]
pub(crate) struct ZombieHealthBarFill;

#[derive(Component)]
pub(crate) struct ZombieHealthBarText;

#[derive(Component)]
pub(crate) struct Gun {
    /// Turns the scope's own optical axis onto the camera's, read from the two
    /// points the mesh marks along the tube rather than assumed. Identity for
    /// a scope mounted true, and for a weapon with no glass at all.
    pub(crate) optical_fix: Quat,
    /// How ready the weapon was when the sights started coming up, latched for
    /// the whole of that raise, and whether they were being asked for last
    /// frame -- which is what says a raise is starting.
    pub(crate) ads_from_ready: f32,
    pub(crate) ads_wanted: bool,
    pub(crate) base_offset: Vec3,
    pub(crate) recoil: f32,
    /// Where the kick is headed. The shot sets this; recoil chases it, so the
    /// blow has a travel instead of arriving whole in one frame.
    pub(crate) recoil_target: f32,
    pub(crate) ammo: u32,

    pub(crate) reload_remaining: f32,

    pub(crate) ads_blend: f32,
    /// How fast the sights are rising, in blend per second. A spring needs to
    /// remember its momentum; that is the whole of what makes it overshoot.
    pub(crate) ads_velocity: f32,
    /// Spring velocity normalised by this weapon's rate. Drives how far off
    /// the sight line the weapon is pointing while it settles.
    pub(crate) ads_settle_error: f32,

    pub(crate) wall_raise: f32,
    /// Seconds spent up against that wall without a break. Drives the weapon
    /// to the ready on its own, so a player backed into cover is not left
    /// holding the muzzle in the brickwork.
    pub(crate) wall_hold: f32,

    pub(crate) shoulder_blend: f32,

    pub(crate) ready_blend: f32,
    pub(crate) ready_armed: bool,

    /// Yaw and roll of the kick, drawn fresh each shot. Two numbers rather
    /// than one scaling both: sharing a scalar fixes the direction of the
    /// kick and only varies how far along it the weapon goes.
    /// How far across the body the weapon is carried, and how long since the
    /// last shot. The clock is what keeps a player firing on the move in a
    /// pose they can fire from.
    pub(crate) sprint_blend: f32,
    pub(crate) since_shot: f32,
    /// How long the weapon has had nothing asked of it, how far through the
    /// look-over it is, how much of that pose is showing, and how many times
    /// it has already been done -- each round of idling waits longer than the
    /// last.
    pub(crate) idle_elapsed: f32,
    pub(crate) inspect_t: f32,
    pub(crate) inspect_blend: f32,
    pub(crate) inspect_done: u32,
    /// Where the look-over has actually got to, and how fast. The table is
    /// only the target; three kilos of rifle arrive at it late and go a
    /// little past.
    pub(crate) inspect_offset: Vec3,
    pub(crate) inspect_offset_vel: Vec3,
    pub(crate) inspect_angles: Vec3,
    pub(crate) inspect_angles_vel: Vec3,
    pub(crate) recoil_jitter: Vec2,
    /// Advanced once per draw, so the sequence depends on shots fired rather
    /// than on when they were fired.
    pub(crate) recoil_rng: u32,


    /// How far the weapon is still turned away from where the view now points,
    /// as yaw then pitch in degrees. Chases zero, so it is only non-zero while
    /// the view is moving or has just stopped.
    pub(crate) look_sway: Vec2,
    /// The sway's own velocity. Carrying it is what lets the weapon pass the
    /// mark and come back instead of easing onto it.
    pub(crate) look_sway_velocity: Vec2,

    /// Which way the step is leaning the weapon, each -1 to 1: `x` cants it
    /// towards the side being stepped, `y` drops the muzzle going forward and
    /// lifts it going back. Smoothed here rather than read straight off the
    /// stick, which would snap the weapon level the instant a key came up.
    pub(crate) step_lean: Vec2,

    /// Socket_Muzzle in the weapon's own frame, kept here so anything leaving
    /// the barrel can be placed from the current weapon transform without
    /// reaching for the socket entity or carrying a stale world transform.
    pub(crate) muzzle_local: Transform,
    /// Where the brass leaves, in the weapon.s own space. Cached beside the
    /// muzzle and for the same reason: the socket never moves inside the
    /// weapon, so the propagated copy is exact rather than a frame late.
    pub(crate) eject_local: Transform,

    pub(crate) settle_jitter: Vec3,
    pub(crate) settle_current: Vec3,
    pub(crate) settle_twist_jitter: f32,
    pub(crate) settle_twist_current: f32,
    pub(crate) settle_rolled: bool,
}

#[derive(Component)]
pub(crate) struct GunMesh;

#[derive(Component)]
pub(crate) struct GunMeshPart;

#[derive(Resource)]
pub(crate) struct ZombieKindAnims {
    classic: (Handle<AnimationGraph>, Vec<AnimationNodeIndex>),
    soldier: (Handle<AnimationGraph>, Vec<AnimationNodeIndex>),
    female: (Handle<AnimationGraph>, Vec<AnimationNodeIndex>),
}

impl ZombieKindAnims {
    fn slot(&self, kind: ZombieKind) -> &(Handle<AnimationGraph>, Vec<AnimationNodeIndex>) {
        match kind {
            ZombieKind::Classic => &self.classic,
            ZombieKind::Soldier => &self.soldier,
            ZombieKind::Female => &self.female,
        }
    }
}

#[derive(Resource)]
pub(crate) struct ZombieAnimGltfs {
    /// None for a kind ZOMBIE_KIND_WEIGHTS never spawns. Loading one costs
    /// its whole texture set: zombie_female alone carries three 4096s, 192 MB
    /// decoded, for a kind that never appears.
    kinds: [Option<Handle<bevy::gltf::Gltf>>; 3],
    player: Handle<bevy::gltf::Gltf>,
}

impl ZombieAnimGltfs {
    fn of(&self, kind: ZombieKind) -> Option<&Handle<bevy::gltf::Gltf>> {
        self.kinds[kind as usize].as_ref()
    }
}

pub(crate) fn load_zombie_anim_gltfs(
    asset_server: &AssetServer,
    tuning: &PlayerModelTuning,
) -> ZombieAnimGltfs {
    let load = |kind: ZombieKind| {
        (ZOMBIE_KIND_WEIGHTS[kind as usize] > 0.0)
            .then(|| asset_server.load(zombie_kind_anim_path(kind)))
    };
    ZombieAnimGltfs {
        kinds: [
            load(ZombieKind::Classic),
            load(ZombieKind::Soldier),
            load(ZombieKind::Female),
        ],
        player: asset_server.load(tuning.anim_source_or_mesh().to_string()),
    }
}

pub(crate) fn build_zombie_anims_when_loaded(
    mut commands: Commands,
    pending: Option<Res<ZombieAnimGltfs>>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    tuning: Res<PlayerModelTuning>,
    asset_server: Res<AssetServer>,
) {
    let Some(pending) = pending else { return };

    // Settled, not loaded: a glTF that fails must not hold the game in the
    // loading screen for ever. It degrades to a kind with no clips.
    let settled = |handle: &Handle<bevy::gltf::Gltf>| {
        matches!(
            asset_server.load_state(handle),
            bevy::asset::LoadState::Loaded | bevy::asset::LoadState::Failed(_)
        )
    };
    let wanted: Vec<&Handle<bevy::gltf::Gltf>> = pending
        .kinds
        .iter()
        .flatten()
        .chain(std::iter::once(&pending.player))
        .collect();
    if !wanted.iter().all(|handle| settled(handle)) {
        return;
    }
    for handle in &wanted {
        if gltfs.get(*handle).is_none() {
            warn!("[anim] a glTF failed to load; that kind will have no clips");
        }
    }

    let clips_of = |handle: Option<&Handle<bevy::gltf::Gltf>>| -> &[Handle<AnimationClip>] {
        handle.and_then(|h| gltfs.get(h)).map_or(&[][..], |gltf| &gltf.animations)
    };
    let classic = clips_of(pending.of(ZombieKind::Classic));
    let soldier = clips_of(pending.of(ZombieKind::Soldier));
    let female = clips_of(pending.of(ZombieKind::Female));
    let player = clips_of(Some(&pending.player));

    let all = |kind: ZombieKind| -> &[Handle<AnimationClip>] {
        match kind {
            ZombieKind::Classic => classic,
            ZombieKind::Soldier => soldier,
            ZombieKind::Female => female,
        }
    };

    commands.insert_resource(ZombieAnim::build(&mut graphs, |kind, index| {
        all(kind).get(index).cloned().unwrap_or_else(|| {
            // A kind with zero spawn weight is not loaded at all; its graph
            // is built empty and never played.
            if pending.of(kind).is_some() {
                warn!("[zombie] {kind:?} has no clip {index}, wanted {}", all(kind).len());
            }
            Handle::default()
        })
    }));
    commands.insert_resource(build_zombie_kind_anims(
        &mut graphs,
        classic,
        soldier,
        female,
    ));
    commands.insert_resource(build_player_model_anim(&mut graphs, &tuning, player));
    commands.remove_resource::<ZombieAnimGltfs>();
}

fn build_zombie_kind_anims(
    graphs: &mut Assets<AnimationGraph>,
    classic: &[Handle<AnimationClip>],
    soldier: &[Handle<AnimationClip>],
    female: &[Handle<AnimationClip>],
) -> ZombieKindAnims {
    let mut build = |kind: ZombieKind, all: &[Handle<AnimationClip>]| {
        let clips: Vec<Handle<AnimationClip>> = (0..zombie_kind_clip_count(kind))
            .filter_map(|i| all.get(i).cloned())
            .collect();
        let (graph, nodes) = AnimationGraph::from_clips(clips);
        (graphs.add(graph), nodes)
    };
    ZombieKindAnims {
        classic: build(ZombieKind::Classic, classic),
        soldier: build(ZombieKind::Soldier, soldier),
        female: build(ZombieKind::Female, female),
    }
}

#[derive(Resource)]
pub(crate) struct PlayerModelAnim {
    pub(crate) graph: Handle<AnimationGraph>,
    pub(crate) idle: AnimationNodeIndex,
    pub(crate) walk: AnimationNodeIndex,
    pub(crate) run: AnimationNodeIndex,

    pub(crate) run_back: AnimationNodeIndex,
}

fn build_player_model_anim(
    graphs: &mut Assets<AnimationGraph>,
    tuning: &PlayerModelTuning,
    all: &[Handle<AnimationClip>],
) -> PlayerModelAnim {
    let clips: Vec<Handle<AnimationClip>> = [
        tuning.animations.idle,
        tuning.animations.walk,
        tuning.animations.run,
        tuning.animations.run_back,
    ]
    .into_iter()
    .map(|index| {
        all.get(index).cloned().unwrap_or_else(|| {
            warn!("[player] no clip {index}, model has {}", all.len());
            Handle::default()
        })
    })
    .collect();
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    PlayerModelAnim {
        graph: graphs.add(graph),
        idle: nodes[0],
        walk: nodes[1],
        run: nodes[2],
        run_back: nodes[3],
    }
}

pub(crate) fn on_zombie_replicated(
    trigger: On<Add, Zombie>,
    mut commands: Commands,

    showcase: Query<(), With<ZombieShowcaseClip>>,
) {
    let entity = trigger.event().entity;
    if showcase.contains(entity) {
        return;
    }
    commands.entity(entity).insert(zombie_cosmetic_bundle());
    spawn_zombie_health_bar(&mut commands, entity);
    spawn_radar_blip(&mut commands, entity);
}

pub(crate) fn on_zombie_kind_replicated(
    trigger: On<Add, ZombieKind>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    kinds: Query<&ZombieKind>,

    showcase: Query<(), With<ZombieShowcaseClip>>,
) {
    let entity = trigger.event().entity;
    if showcase.contains(entity) {
        return;
    }
    let Ok(&kind) = kinds.get(entity) else { return };
    commands.entity(entity).with_child(zombie_mesh_bundle(&asset_server, kind));
}

pub(crate) fn spawn_local_zombie(commands: &mut Commands, asset_server: &AssetServer, spawn_pos: Vec3) {
    let zombie_entity = commands
        .spawn((
            Transform::from_translation(spawn_pos),
            Zombie,

            Health(ZOMBIE_MAX_HEALTH),
            zombie_cosmetic_bundle(),
        ))
        .with_child(zombie_mesh_bundle(asset_server, ZombieKind::Classic))
        .id();
    spawn_zombie_health_bar(commands, zombie_entity);
    spawn_radar_blip(commands, zombie_entity);
}

pub(crate) fn zombie_cosmetic_bundle() -> impl Bundle {
    (
        Visibility::default(),
        ZombieHitboxBones::default(),
        KnockbackVelocity::default(),
        ZombieAttackTimer::default(),
        ZombiePose::default(),
        ZombieWalkVariant::default(),
    )
}

pub(crate) fn zombie_mesh_bundle(asset_server: &AssetServer, kind: ZombieKind) -> impl Bundle {
    (
        WorldAssetRoot(load_gltf_scene(&asset_server, zombie_kind_mesh_path(kind), RenderAssetUsages::RENDER_WORLD)),
        Transform::IDENTITY,
    )
}

pub(crate) fn spawn_zombie_health_bar(commands: &mut Commands, zombie: Entity) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(ZOMBIE_HEALTH_BAR_WIDTH_PX),
                height: Val::Px(ZOMBIE_HEALTH_BAR_HEIGHT_PX),

                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZombieHealthBar { zombie },
            NativeUiRoot,
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.9, 0.2)),
                ZombieHealthBarFill,
            ));

            bar.spawn((
                Text::new(format!("{ZOMBIE_MAX_HEALTH}/{ZOMBIE_MAX_HEALTH}")),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(-14.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ZombieHealthBarText,
            ));
        });
}

pub(crate) fn update_zombie_health_bars(
    mut commands: Commands,
    mut timings: ResMut<PerfTimings>,
    hidden: Res<ZombieHealthBarsHidden>,
    camera: Query<(&Camera, &GlobalTransform), With<FpsCamera>>,
    zombies: Query<(&Transform, &Health), With<Zombie>>,
    mut bars: Query<(Entity, &ZombieHealthBar, &mut Node, &Children)>,
    mut fills: Query<&mut Node, (With<ZombieHealthBarFill>, Without<ZombieHealthBar>)>,
    mut texts: Query<&mut Text, With<ZombieHealthBarText>>,
) {
    let perf_start = std::time::Instant::now();
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };

    if hidden.0 {
        for (_, _, mut node, _) in &mut bars {
            node.display = Display::None;
        }
        timings.health_bars_ms = perf_start.elapsed().as_secs_f32() * 1000.0;
        return;
    }

    for (bar_entity, bar, mut node, children) in &mut bars {
        let Ok((zombie_transform, health)) = zombies.get(bar.zombie) else {

            commands.entity(bar_entity).try_despawn();
            continue;
        };

        let head_pos = zombie_transform.translation + Vec3::Y * ZOMBIE_HEALTH_BAR_HEIGHT;
        let Ok(screen_pos) = camera.world_to_viewport(camera_transform, head_pos) else {

            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = Val::Px(screen_pos.x - ZOMBIE_HEALTH_BAR_WIDTH_PX / 2.0);
        node.top = Val::Px(screen_pos.y);

        let fraction = (health.0 / ZOMBIE_MAX_HEALTH).clamp(0.0, 1.0);
        for &child in children {
            if let Ok(mut fill_node) = fills.get_mut(child) {
                fill_node.width = Val::Percent(fraction * 100.0);
            }
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = format!("{:.1}/{ZOMBIE_MAX_HEALTH}", health.0.max(0.0));
            }
        }
    }
    timings.health_bars_ms = perf_start.elapsed().as_secs_f32() * 1000.0;
}

pub(crate) fn freeze_zombie_animations(
    freeze: Res<FreezeZombies>,
    anim_disabled: Res<ZombieAnimDisabled>,
    zombies: Query<&ZombieAnimationPlayer, With<Zombie>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let should_pause = freeze.0 || anim_disabled.0;
    for link in &zombies {
        let Ok(mut player) = players.get_mut(link.0) else {
            continue;
        };
        for (_, active) in player.playing_animations_mut() {
            if should_pause {
                active.pause();
            } else {
                active.resume();
            }
        }
    }
}

const ZOMBIE_SPLIT_SCENE_ROOT_NAME: &str = "target_character";

pub(crate) fn wire_zombie_scene_animation_targets(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    children: Query<&Children>,
    names: Query<&Name>,
    zombies: Query<(), With<Zombie>>,
    new_names: Query<Entity, Added<Name>>,
) {
    for entity in &new_names {
        let Ok(name) = names.get(entity) else { continue };
        if name.as_str() != ZOMBIE_SPLIT_SCENE_ROOT_NAME {
            continue;
        }

        if find_animation_root(entity, &child_of, |e| zombies.contains(e)).is_none() {
            continue;
        }
        commands.entity(entity).try_insert(AnimationPlayer::default());
        wire_animation_target_path(&mut commands, entity, entity, Vec::new(), &children, &names);
    }
}

pub(crate) fn wire_zombie_animations(
    mut commands: Commands,
    anim: Res<ZombieAnim>,
    kind_anims: Res<ZombieKindAnims>,
    child_of: Query<&ChildOf>,
    zombies: Query<(Option<&ZombieShowcaseClip>, Option<&ZombieKind>), With<Zombie>>,
    // Without<ParkedAnimation> or this re-wires a parked body the next frame,
    // resetting its transitions and snapping it back to idle.
    mut new_players: Query<
        (Entity, &mut AnimationPlayer),
        (Without<AnimationGraphHandle>, Without<ParkedAnimation>),
    >,
) {
    for (player_entity, mut player) in &mut new_players {
        let Some(root) = find_animation_root(player_entity, &child_of, |e| zombies.contains(e)) else {
            continue;
        };
        let (showcase, kind) = zombies
            .get(root)
            .map_or((None, None), |(s, k)| (s.copied(), k.copied()));

        if let Some(ZombieShowcaseClip(clip)) = showcase {

            let (graph, clips) = kind_anims.slot(kind.unwrap_or_default());
            commands
                .entity(player_entity)
                .try_insert((AnimationGraphHandle(graph.clone()), AnimationTransitions::new()));
            if let Some(&node) = clips.get(clip % clips.len().max(1)) {
                player.play(node).repeat();
            }
        } else {
            let slot = anim.slot(kind.unwrap_or_default());
            commands
                .entity(player_entity)
                .try_insert((AnimationGraphHandle(slot.graph.clone()), AnimationTransitions::new()));
            player.play(slot.idle()).repeat();
        }
        commands.entity(root).try_insert(ZombieAnimationPlayer(player_entity));
    }
}

const PLAYER_MODEL_SCENE_ROOT_NAME: &str = "target_character";

/// A Meshy export leaves Mixamo's joint names bare; a Mixamo one keeps the
/// `mixamorig:` prefix on every single joint. Matching the bare name takes
/// both, and costs nothing for a name that has no prefix to drop.
///
/// Worth being blunt about the failure this avoids: nothing errors when the
/// names miss. The bones simply never wire, so the head is never collapsed and
/// fills the first-person view, and no clip ever plays.
fn bare_bone_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

pub(crate) fn wire_player_model_scene_nodes(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut roots: Query<&mut PlayerModelSkeletonBones, With<PlayerModelAnimRoot>>,
    new_names: Query<Entity, Added<Name>>,
) {
    for entity in &new_names {
        let Ok(name) = names.get(entity) else { continue };
        // Zombies share these bone names, so the grip is only pinned on a
        // skeleton that hangs under the player model.
        if let Some(rotation) = finger_grip_rotation(bare_bone_name(name.as_str())) {
            if find_animation_root(entity, &child_of, |e| roots.contains(e)).is_some() {
                commands.entity(entity).try_insert(GripFingerPose(rotation));
            }
        }
        match bare_bone_name(name.as_str()) {
            "LeftUpLeg" | "RightUpLeg" | "RightArm" | "RightForeArm" | "RightHand" | "LeftArm"
            | "LeftForeArm" | "LeftHand" | "Head" | "Hips" => {
                let Some(root) = find_animation_root(entity, &child_of, |e| roots.contains(e)) else {
                    continue;
                };
                let Ok(mut bones) = roots.get_mut(root) else { continue };
                if let Ok(transform) = transforms.get(entity) {
                    commands.entity(entity).try_insert(BoneRest(transform.rotation));
                }
                match bare_bone_name(name.as_str()) {
                    "LeftUpLeg" => bones.left_upper_leg = Some(entity),
                    "RightUpLeg" => bones.right_upper_leg = Some(entity),
                    "RightArm" => bones.right_arm = Some(entity),
                    "RightForeArm" => bones.right_fore_arm = Some(entity),
                    "RightHand" => bones.right_hand = Some(entity),
                    "LeftArm" => bones.left_arm = Some(entity),
                    "LeftHand" => bones.left_hand = Some(entity),
                    "Head" => bones.head = Some(entity),
                    "Hips" => bones.hips = Some(entity),
                    _ => bones.left_fore_arm = Some(entity),
                }

            }
            PLAYER_MODEL_SCENE_ROOT_NAME => {

                if find_animation_root(entity, &child_of, |e| roots.contains(e)).is_none() {
                    continue;
                }
                commands.entity(entity).try_insert(AnimationPlayer::default());
                wire_animation_target_path(&mut commands, entity, entity, Vec::new(), &children, &names);
            }
            _ => {}
        }
    }
}

fn wire_animation_target_path(
    commands: &mut Commands,
    player: Entity,
    entity: Entity,
    mut path: Vec<Name>,
    children: &Query<&Children>,
    names: &Query<&Name>,
) {
    let Ok(name) = names.get(entity) else { return };
    path.push(name.clone());
    commands.entity(entity).try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(player)));
    if let Ok(kids) = children.get(entity) {
        for &child in kids {
            wire_animation_target_path(commands, player, child, path.clone(), children, names);
        }
    }
}

pub(crate) fn wire_player_model_animations(
    mut commands: Commands,
    anim: Res<PlayerModelAnim>,
    child_of: Query<&ChildOf>,
    roots: Query<(), With<PlayerModelAnimRoot>>,
    viewmodel: Query<(), With<ViewmodelBody>>,
    // Without<ParkedAnimation> or this re-wires a parked body the next frame,
    // resetting its transitions and snapping it back to idle.
    mut new_players: Query<
        (Entity, &mut AnimationPlayer),
        (Without<AnimationGraphHandle>, Without<ParkedAnimation>),
    >,
) {
    for (player_entity, mut player) in &mut new_players {
        let Some(root) = find_animation_root(player_entity, &child_of, |e| roots.contains(e)) else {
            continue;
        };
        let grip_ik_owns_these_arms = viewmodel.contains(root);
        if grip_ik_owns_these_arms {
            continue;
        }
        commands
            .entity(player_entity)
            .try_insert(AnimationGraphHandle(anim.graph.clone()));
        player.play(anim.idle).repeat();
        commands
            .entity(root)
            .try_insert((PlayerModelAnimationPlayer(player_entity), PlayerModelPose::default()));
    }
}


pub(crate) fn update_local_player_model_animation(
    over: Res<PlayerModelPoseOverride>,
    moving: Res<LocalPlayerMoving>,
    sprinting: Res<LocalPlayerSprinting>,
    move_axis: Res<LocalPlayerMoveAxis>,
    mut roots: Query<(&PlayerModelAnimationPlayer, &mut PlayerModelPose), (With<PlayerModelAnimRoot>, With<ViewmodelBody>)>,
    anim: Res<PlayerModelAnim>,
    mut players: Query<&mut AnimationPlayer>,

    mut last_run_back: Local<bool>,
) {

    if over.0 {
        return;
    }
    let Ok((link, mut pose)) = roots.single_mut() else {
        return;
    };
    let new_pose = if moving.0 && sprinting.0 {
        PlayerModelPose::Run
    } else if moving.0 {
        PlayerModelPose::Walk
    } else {
        PlayerModelPose::Idle
    };

    let run_back = new_pose == PlayerModelPose::Run && move_axis.0.y < 0.0;
    if new_pose == *pose && run_back == *last_run_back {
        return;
    }
    *pose = new_pose;
    *last_run_back = run_back;
    if let Ok(mut player) = players.get_mut(link.0) {
        player.stop_all();
        let node = match new_pose {
            PlayerModelPose::Idle => anim.idle,
            PlayerModelPose::Walk => anim.walk,
            PlayerModelPose::Run => {
                if run_back {
                    anim.run_back
                } else {
                    anim.run
                }
            }
        };
        player.play(node).repeat();
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub(crate) struct PlayerModelPoseOverride(pub(crate) bool);

pub(crate) fn apply_player_model_pose_override(
    over: Res<PlayerModelPoseOverride>,
    anim: Res<PlayerModelAnim>,
    roots: Query<(&PlayerModelAnimationPlayer, &PlayerModelPose), (With<PlayerModelAnimRoot>, Changed<PlayerModelPose>)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if !over.0 {
        return;
    }
    for (link, pose) in &roots {
        let Ok(mut player) = players.get_mut(link.0) else {
            continue;
        };
        player.stop_all();
        let node = match *pose {
            PlayerModelPose::Idle => anim.idle,
            PlayerModelPose::Walk => anim.walk,
            PlayerModelPose::Run => anim.run,
        };
        player.play(node).repeat();
    }
}

pub(crate) fn wire_gun_mesh(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    gun: Query<(), With<Gun>>,
    names: Query<&Name>,
    new_meshes: Query<Entity, Added<Mesh3d>>,
) {
    for mesh_entity in &new_meshes {
        let mut current = mesh_entity;
        // The magazine is the one part that leaves the gun, so it is marked on
        // the way past. Which entity carries the name depends on how many
        // primitives the part has, hence the whole walk rather than the leaf.
        let mut is_magazine = false;
        for _ in 0..20 {
            is_magazine |= names.get(current).is_ok_and(|n| n.as_str().contains("magazine"));
            if gun.contains(current) {
                commands
                    .entity(mesh_entity)
                    .insert((GunMesh, RenderLayers::layer(VIEWMODEL_LAYER)));
                if is_magazine {
                    commands.entity(mesh_entity).insert(ViewmodelMagazine);
                }
                break;
            }
            let Ok(parent) = child_of.get(current) else {
                break;
            };
            current = parent.0;
        }
    }
}

pub(crate) fn update_zombie_animation(
    anim: Res<ZombieAnim>,
    player: Query<&Transform, With<Player>>,

    mut zombies: Query<
        (
            Entity,
            &Transform,
            &ZombieMotion,
            &Health,
            &ZombieAnimationPlayer,
            Option<&ZombieKind>,
            Option<&crate::dismember::SeveredParts>,
            &mut ZombiePose,
            &mut ZombieWalkVariant,
        ),
        (With<Zombie>, Without<ZombieShowcaseClip>),
    >,
    mut players: Query<&mut AnimationPlayer>,
    mut transitions: Query<&mut AnimationTransitions>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    for (entity, zombie_transform, net_transform, health, link, kind, cuts, mut pose, mut walk) in &mut zombies {
        let distance = zombie_transform.translation.distance(player_transform.translation);
        let health_fraction = health.0 / ZOMBIE_MAX_HEALTH;
        let new_pose = zombie_pose_for(distance, health_fraction, net_transform.moving, cuts.is_some_and(crate::dismember::SeveredParts::lost_a_leg));
        let slot = anim.slot(kind.copied().unwrap_or_default());
        let seed = entity.to_bits() as usize;
        let new_walk = match new_pose {
            ZombiePose::Walk => Some(slot.walk(seed, health_fraction)),
            ZombiePose::Crawl => Some(slot.crawl(seed)),
            _ => None,
        };
        if new_pose == *pose && new_walk == walk.0 {
            continue;
        }
        // Only the flavour of the walk changed, not the gait. Those two clips
        // are the same stride at a different limp, so the phase carries: a
        // zombie that takes a hit mid-step keeps the step instead of snapping
        // back to the top of the cycle.
        let gait_kept = new_pose == *pose;
        let was = walk.0.unwrap_or_else(|| slot.node(*pose));
        *pose = new_pose;
        walk.0 = new_walk;
        let node = new_walk.unwrap_or_else(|| slot.node(new_pose));
        let (Ok(mut player), Ok(mut fade)) = (players.get_mut(link.0), transitions.get_mut(link.0))
        else {
            continue;
        };
        let phase = gait_kept.then(|| player.animation(was).map(|a| a.seek_time())).flatten();
        let blend = if new_pose == ZombiePose::Bite {
            ZOMBIE_BLEND_BITE
        } else if gait_kept {
            ZOMBIE_BLEND_GAIT
        } else {
            ZOMBIE_BLEND
        };
        fade.play(&mut player, node, blend).repeat();
        // play() starts the new clip from zero; this puts it back where the
        // old one had got to. Over the new clip's length is fine -- a repeat
        // takes it modulo its duration on the next advance.
        if let (Some(at), Some(active)) = (phase, player.animation_mut(node)) {
            active.seek_to(at);
        }
    }
}

/// Bevy evaluates bones for every animated body in the world: `animate_targets`
/// filters on `Without<IsResource>` and nothing else, so a zombie behind a wall
/// or eighty metres down the street costs the same as the one in front of you.
///
/// Taking the graph handle off a distant body drops it from `animate_targets`
/// and from `advance_animations` both, which is what makes this safe -- the
/// clock stops with the pose rather than running on, so the body resumes mid
/// stride instead of snapping to where the cycle would have reached.
#[derive(Component)]
pub(crate) struct ParkedAnimation(AnimationGraphHandle);

pub(crate) fn park_distant_animations(
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    bodies: Query<
        (Entity, &GlobalTransform, Option<&AnimationGraphHandle>, Option<&ParkedAnimation>),
        With<AnimationPlayer>,
    >,
) {
    let Ok(eye) = camera.single() else {
        return;
    };
    let eye = eye.translation();
    for (entity, at, live, parked) in &bodies {
        let away = eye.distance_squared(at.translation());
        match (live, parked) {
            (Some(handle), None) if away > ANIMATION_PARK_DISTANCE * ANIMATION_PARK_DISTANCE => {
                commands
                    .entity(entity)
                    .insert(ParkedAnimation(handle.clone()))
                    .remove::<AnimationGraphHandle>();
            }
            (None, Some(parked)) if away < ANIMATION_WAKE_DISTANCE * ANIMATION_WAKE_DISTANCE => {
                let handle = parked.0.clone();
                commands.entity(entity).insert(handle).remove::<ParkedAnimation>();
            }
            _ => {}
        }
    }
}
