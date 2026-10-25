use bevy::prelude::*;

use crate::*;

#[derive(Resource)]
pub(crate) struct ScopeCameraEnabled(pub(crate) bool);
impl Default for ScopeCameraEnabled {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
pub(crate) struct ScopeCamera;

#[derive(Component)]
pub(crate) struct ScopeScreen;

#[derive(Component)]
pub(crate) struct ScopeReticle;

/// The red bead at the duplex centre. Its own entity because a mesh carries
/// one material; a child of the reticle, so throw and scaling come for free.
#[derive(Component)]
pub(crate) struct ScopeReticleCentreDot;

/// One mark per kind of glass, built once and swapped on the single reticle
/// entity rather than respawned, so nothing has to be rewired on a weapon
/// change.
#[derive(Resource)]
pub(crate) struct ReticleAssets {
    pub(crate) chevron: (Handle<Mesh>, Handle<StandardMaterial>),
    pub(crate) duplex: (Handle<Mesh>, Handle<StandardMaterial>),
}

/// Hands the sighted weapon its own mark.
pub(crate) fn swap_scope_reticle(
    active: Res<ActiveWeapon>,
    // Built in setup_gameplay, whose insert_resource only lands at the next
    // sync point, so this runs at least one frame before it exists.
    assets: Option<Res<ReticleAssets>>,
    // Without the exclusion this and centre_dot both take &mut Visibility and
    // Bevy refuses the system outright -- B0001, at startup, before a frame.
    mut reticle: Query<
        (&mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>, &mut Visibility),
        (With<ScopeReticle>, Without<ScopeReticleCentreDot>),
    >,
    mut centre_dot: Query<
        (&mut Visibility, &mut Transform),
        (With<ScopeReticleCentreDot>, Without<ScopeReticle>),
    >,
    mut shown: Local<Option<WeaponId>>,
) {
    let Some(assets) = assets else { return };
    if *shown == Some(active.0) {
        return;
    }
    let Ok((mut mesh, mut material, mut reticle_visibility)) = reticle.single_mut() else {
        return;
    };
    // The AK wears a red dot: the centre dot alone, with the duplex bars
    // hidden. The M24 keeps the duplex, whose gap the dot fills. Anything
    // sighted without glass gets the chevron, though update_scope_lens has
    // already stopped it before this matters.
    // The AK's dot carries the sight by itself, where the M24's only fills the
    // gap the duplex bars already point at, so it is drawn larger.
    let (bars, dot) = match active.0 {
        WeaponId::M24 => (Some(&assets.duplex), Some(1.0)),
        WeaponId::Ak47 => (None, Some(2.5)),
        _ => (Some(&assets.chevron), None),
    };
    // Hiding the bars does not hide the dot: Visibility::Visible on a child
    // wins over a hidden parent, which is what the dot is already set to.
    *reticle_visibility = if bars.is_some() { Visibility::Visible } else { Visibility::Hidden };
    if let Ok((mut visibility, mut transform)) = centre_dot.single_mut() {
        *visibility = if dot.is_some() { Visibility::Visible } else { Visibility::Hidden };
        transform.scale = Vec3::splat(dot.unwrap_or(1.0));
    }
    let Some((next_mesh, next_material)) = bars else {
        *shown = Some(active.0);
        return;
    };
    mesh.0 = next_mesh.clone();
    material.0 = next_material.clone();
    *shown = Some(active.0);
}

/// The two ends of the glass, named for the parts they are rather than for
/// what the engine does with them -- the objective is the front lens where
/// light enters, the eyepiece the rear one the eye looks into. The old names
/// said ScopeCamera and ScopeView, and the first of those stopped being true
/// the day the scope camera moved to the eye.
///
/// The picture is drawn at the eyepiece but set a little down the tube from
/// the glass, so the glass draws over it and the view reads as recessed.
///
/// Carrying that depth in the asset rather than in a constant here is the point
/// — it is a property of one scope's glass and tube, not of every weapon.
///
/// Both sockets' scale is the lens radius, so a unit circle under one is the
/// right width. A camera must take the position and the turn but never that
/// scale, or it would scale the whole world it renders.
///
/// Riding these means recoil, sway, bob and the shoulder swap come for free and
/// cannot drift out of step with the mesh.
#[derive(Component)]
pub(crate) struct ObjectiveSocket;

#[derive(Component)]
pub(crate) struct EyepieceSocket;

/// Pins the sights up so the scope can be looked at without waiting for the
/// autopilot to pick a target far enough away to want them.
pub(crate) fn force_ads() -> bool {
    std::env::var("FORCE_ADS").map(|value| value == "1").unwrap_or(false)
}

/// Where the eye lines up behind the sights: the rear glass on a scope, the
/// rear sight on irons. Distinct from EyepieceSocket, which is where the
/// picture draws, further down the tube than this.
#[derive(Component)]
pub(crate) struct AimSocket;

/// Where the brass leaves the weapon, marked on the mesh so the shell comes
/// out of the port rather than out of a number that has to be guessed against
/// a receiver nobody measured.
#[derive(Component)]
pub(crate) struct EjectSocket;

/// The front post on a weapon that sights down irons. Paired with AimSocket it
/// gives the sight line, the same way the scope pair gives the optical axis.
#[derive(Component)]
pub(crate) struct FrontSightSocket;

/// The bore's end. Everything that leaves the barrel is placed and aimed from
/// here, so flash, smoke and casings follow the weapon through recoil and sway
/// without any of them holding their own copy of where the barrel is.
#[derive(Component)]
pub(crate) struct MuzzleSocket;

/// Caches where the bore ends inside the weapon.
///
/// The socket does not move within the weapon, so reparenting last frame's
/// world transforms yields a value that is exact rather than one frame late.
/// What it is composed with afterwards is the current weapon transform.
pub(crate) fn track_muzzle_socket(
    muzzle: Query<&GlobalTransform, With<MuzzleSocket>>,
    eject: Query<&GlobalTransform, With<EjectSocket>>,
    mut gun: Query<(&GlobalTransform, &mut Gun)>,
) {
    let Ok((gun_global, mut gun)) = gun.single_mut() else { return };
    if let Ok(socket) = muzzle.single() {
        gun.muzzle_local = socket.reparented_to(gun_global);
    }
    if let Ok(socket) = eject.single() {
        gun.eject_local = socket.reparented_to(gun_global);
    }
}

/// Reads the scope's optical axis off the two points the mesh marks along the
/// tube, and caches the turn that brings it onto the camera's.
///
/// Sighting only ever slid the weapon until Socket_Aim sat on the camera axis
/// and then assumed the glass was looking straight down the mesh. Nothing
/// checked it, so a scope mounted with any cant would have aimed off and no
/// weapon could have said so. Two marked points are enough to know instead of
/// assume — 304mm apart on the M24, which is plenty of baseline.
///
/// Neither point moves inside the weapon, so this runs off the propagated
/// transforms and the result is exact rather than a frame late.
pub(crate) fn track_optical_axis(
    glass: Query<&GlobalTransform, With<ObjectiveSocket>>,
    post: Query<&GlobalTransform, With<FrontSightSocket>>,
    aim: Query<&GlobalTransform, (With<AimSocket>, Without<ObjectiveSocket>, Without<FrontSightSocket>)>,
    mut gun: Query<(&GlobalTransform, &mut Gun)>,
) {
    let Ok((gun_global, mut gun)) = gun.single_mut() else { return };
    // Glass on a scope, the front post on irons: either way it is the far end
    // of the sight line, and either way the near end is where the eye goes.
    let front = glass.single().or(post.single());
    let (Ok(front), Ok(aim)) = (front, aim.single()) else {
        gun.optical_fix = Quat::IDENTITY;
        return;
    };
    let down_the_tube = front.reparented_to(gun_global).translation
        - aim.reparented_to(gun_global).translation;
    let Ok(axis) = Dir3::new(down_the_tube) else { return };
    gun.optical_fix = Quat::from_rotation_arc(*axis, Vec3::NEG_Z);
}

/// Hands the mesh's own named points to the systems that want them, so the
/// weapon table carries none of this geometry twice.
///
/// Only a weapon with glass marks the scope pair; one that sights down irons
/// leaves both unmarked and the whole scope view stays off for it.
pub(crate) fn wire_weapon_sockets(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    gun: Query<(), With<Gun>>,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        if !name.as_str().starts_with("Socket_") {
            continue;
        }
        let mut current = entity;
        let mut under_gun = false;
        for _ in 0..20 {
            if gun.contains(current) {
                under_gun = true;
                break;
            }
            let Ok(parent) = child_of.get(current) else { break };
            current = parent.0;
        }
        if !under_gun {
            continue;
        }
        let mut socket = commands.entity(entity);
        match name.as_str() {
            "Socket_Objective" => socket.insert(ObjectiveSocket),
            "Socket_Eyepiece" => socket.insert(EyepieceSocket),
            "Socket_Aim" => socket.insert(AimSocket),
            "Socket_SightFront" => socket.insert(FrontSightSocket),
            "Socket_Eject" => socket.insert(EjectSocket),
            "Socket_Muzzle" => socket.insert(MuzzleSocket),
            "Socket_RearGrip" => socket.insert(TriggerGripSocket),
            "Socket_FrontGrip" => socket.insert(ForeGripSocket),
            _ => continue,
        };
        debug!("[socket] {name} wired");
    }
}

#[derive(Resource)]
pub(crate) struct ScopeZoom(pub(crate) f32);

pub(crate) fn update_scope_zoom_slider_visibility(
    aim_held: Res<AimHeld>,
    touch_aiming: Res<TouchAiming>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    mut track: Query<&mut Node, With<TouchScopeZoomSliderTrack>>,
) {
    let Ok(mut node) = track.single_mut() else {
        return;
    };
    let aiming = cursor.single().is_ok_and(|c| is_aiming(c, &aim_held, touch_aiming.0));
    node.display = if aiming { Display::Flex } else { Display::None };
}

pub(crate) struct GunPoseTimings {
    recoil_t: f32,
    dip: f32,
    bob_x: f32,
    bob_y: f32,
    bob_roll: f32,
    bob_pitch: f32,
}

pub(crate) fn gun_pose_timings(gun: &Gun, def: &WeaponDef, cycle: &WalkCycle) -> GunPoseTimings {
    let recoil_t = gun.recoil;
    let reload_t = gun.reload_remaining / def.reload_duration;

    // A raised cosine rather than the parabola 4t(1-t) it replaces. Both peak
    // at 1 halfway through and both end at 0, but the parabola arrives and
    // leaves at full speed, so the weapon snapped into the dip and snapped
    // back out of it. This one is flat at both ends: it eases in and eases
    // out, which is the curve the sights and the kick already use.
    let dip = ops::sin(std::f32::consts::PI * reload_t).powi(2);
    let amount = cycle.amount();
    let bob_x = ops::sin(cycle.phase) * GUN_BOB_AMOUNT_X * amount;
    // abs() already doubles the rate: |sin| repeats every pi, not every tau.
    // One dip per footfall, against a full sway every two.
    let bob_y = ops::sin(cycle.phase).abs() * GUN_BOB_AMOUNT_Y * amount;

    // The weapon trails the body. Arms are compliant, and a weapon that cants
    // in exact step with the hips reads as welded to them -- the lag is most
    // of what makes it look carried rather than attached.
    let lagged = cycle.phase - GUN_BOB_LAG;
    let bob_roll = ops::sin(lagged) * GUN_BOB_ROLL_DEG * amount;
    // Pitch turns about the mesh origin, which sits on the bore at the muzzle,
    // so this pins the nose and swings the stock -- the muzzle stays the
    // steadiest thing on screen, which is what a carried weapon does.
    let bob_pitch = ops::sin(lagged).abs() * GUN_BOB_PITCH_DEG * amount;
    GunPoseTimings { recoil_t, dip, bob_x, bob_y, bob_roll, bob_pitch }
}

/// Takes the scope picture from the eye rather than from the weapon.
///
/// Riding the weapon's own socket is what kills the parallax effect: the mark
/// hangs off the barrel, and a camera bolted to the same barrel moves with it,
/// so the two never part and the mark sits dead centre whatever the weapon
/// does. From the eye they move independently, which is the whole point —
/// a weapon held off the eye's line shows its mark off-centre, still over
/// what the barrel is pointing at.
pub(crate) fn sync_scope_camera(
    eye: Query<&Transform, (With<FpsCamera>, Without<ScopeCamera>)>,
    mut scope_camera: Query<&mut Transform, With<ScopeCamera>>,
) {
    let (Ok(eye), Ok(mut scope_transform)) = (eye.single(), scope_camera.single_mut()) else {
        return;
    };
    *scope_transform = Transform::from_translation(eye.translation).with_rotation(eye.rotation);
}

/// Turns the mark to point down the weapon's own sight axis.
///
/// It hangs off the scope camera, so it is always in front of the lens and
/// facing it; all the weapon contributes is the direction. That makes it a
/// perfectly parallax-free sight: the mark shows where the barrel points,
/// which is where the bullet goes, however the weapon sits against the eye.
/// The socket says which way that is, so no weapon carries its own copy.
pub(crate) fn place_scope_reticle(
    active: Res<ActiveWeapon>,
    aim: Query<&GlobalTransform, With<AimSocket>>,
    gun: Query<(&Transform, &GlobalTransform), (With<Gun>, Without<ScopeReticle>)>,
    zoom: Res<ScopeZoom>,
    mut reticle: Query<&mut Transform, With<ScopeReticle>>,
) {
    let (Ok(aim), Ok((gun_local, gun_global)), Ok(mut reticle)) =
        (aim.single(), gun.single(), reticle.single_mut())
    else {
        return;
    };
    // Both the camera and the weapon are placed from the eye this frame, so
    // working in the eye's frame cancels the eye out and leaves no stale term.
    // Only the socket's place *inside* the weapon comes from the propagated
    // transforms, and that never animates, so last frame's copy is exact.
    // Mixing a turn written this frame with one propagated last frame is what
    // shakes the mark in step with how fast the view is moving.
    let in_gun = aim.reparented_to(gun_global);
    // A unit throw, because only the direction survives: the mark hangs off
    // the camera, so pushing it further out moves it along the same ray and
    // changes nothing the player can see. There is no throw to tune.
    let down_the_barrel = gun_local.rotation * in_gun.rotation * Vec3::NEG_Z;
    reticle.translation = down_the_barrel;
    reticle.rotation = Quat::IDENTITY;
    // The mesh is built in radians at unit distance, so this is purely the
    // magnification: holding apparent thickness steady as the target grows is
    // what a second focal plane scope does.
    // Against reticle_zoom, not default_zoom: tying the mark to the weapon's
    // power would fatten it every time the optic is wound up.
    reticle.scale = Vec3::splat(active.def().scope.reticle_zoom / zoom.0);
}

pub(crate) fn adjust_scope_zoom(
    active: Res<ActiveWeapon>,
    mut wheel: MessageReader<MouseWheel>,
    aim_held: Res<AimHeld>,
    touch_aiming: Res<TouchAiming>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    mut zoom: ResMut<ScopeZoom>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    autopilot: Res<Autopilot>,
    autopilot_state: Res<AutopilotState>,
    zombies: Query<&Transform, With<Zombie>>,
) {
    let Ok(cursor) = cursor.single() else {
        return;
    };
    if !is_aiming(cursor, &aim_held, touch_aiming.0) {
        wheel.clear();
        return;
    }

    if autopilot.0 {
        if let Some(target_entity) = autopilot_state.target {
            if let (Ok(camera_global), Ok(zombie_transform)) =
                (camera.single(), zombies.get(target_entity))
            {
                let distance = camera_global.translation().distance(zombie_transform.translation);
                let t = ((distance - AUTO_ZOOM_NEAR_DISTANCE)
                    / (AUTO_ZOOM_FAR_DISTANCE - AUTO_ZOOM_NEAR_DISTANCE))
                    .clamp(0.0, 1.0);
                let scope = &active.def().scope;
                zoom.0 = scope.min_zoom + (scope.max_zoom - scope.min_zoom) * t;
                wheel.clear();
                return;
            }
        }
    }
    for event in wheel.read() {
        zoom.0 = (zoom.0 * SCOPE_ZOOM_STEP_FACTOR.powf(event.y.signum()))
            .clamp(active.def().scope.min_zoom, active.def().scope.max_zoom);
    }
}

pub(crate) fn update_scope_lens(
    gun: Query<&Gun>,
    zoom: Res<ScopeZoom>,
    scope_camera_enabled: Res<ScopeCameraEnabled>,
    mut scope_camera: Query<(&mut Camera, &mut Projection), With<ScopeCamera>>,
    mut lens: Query<(&mut Transform, &mut Visibility), (With<ScopeScreen>, Without<ScopeReticle>)>,
    view_socket: Query<&GlobalTransform, With<EyepieceSocket>>,
    gun_placed: Query<(&Transform, &GlobalTransform), (With<Gun>, Without<ScopeScreen>, Without<ScopeReticle>)>,
) {
    let Ok(gun) = gun.single() else {
        return;
    };
    // No rear glass means no scope picture at all. The M4 aims down irons: the
    // gun comes up and that is the whole of it.
    let visible = !view_socket.is_empty()
        && scope_camera_enabled.0
        && gun.ads_blend > SCOPE_ACTIVE_ADS_THRESHOLD
        && gun.wall_raise < SCOPE_HIDDEN_WALL_RAISE_THRESHOLD;

    if let Ok((mut camera, mut projection)) = scope_camera.single_mut() {
        camera.is_active = visible;
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = SCOPE_BASE_FOV / zoom.0;
        }
    }

    if let Ok((mut transform, mut visibility)) = lens.single_mut() {
        *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };

        // What the player looks into is the rear glass, so sit on it. The disc
        // is a unit circle and the socket's scale is the lens radius, which
        // makes the picture exactly as wide as the real eyepiece.
        //
        // Set back down the tube rather than on the socket. Sat on it the disc
        // lands inside the 1.8mm the mesh's own glass occupies and the two
        // fight for the depth buffer; behind it the glass draws over the
        // picture, which is what looking through a scope does.
        //
        // The disc and the gun are both children of the eye, so the disc's
        // offset is the gun's own transform with the socket's place inside it
        // on the end. Only that inner part comes from the propagated
        // transforms, and it never animates; the gun's transform is this
        // frame's, so the disc cannot trail the mesh when the eye turns.
        if let (Ok(socket), Ok((gun_local, gun_global))) =
            (view_socket.single(), gun_placed.single())
        {
            // Position, turn and width all straight off the socket. Its scale
            // is the tube's bore where the picture is set, so the picture
            // fills the tube rather than reaching for the eyepiece's aperture.
            //
            // There was a factor here that opened the disc out to cover that
            // aperture instead. It cannot hold once the picture is set deep:
            // the cone the eye looks down is wider than the tube that far in,
            // so filling the aperture means leaving the tube. Set deep is what
            // this scope wants, so filling the tube is the rule it gets, and
            // the narrowing to the glass is the recess made visible.
            let in_gun = socket.reparented_to(gun_global);
            *transform = gun_local.mul_transform(in_gun);
        }
    }
}

/// Where the look sway turns the weapon about, ahead of the weapon's own
/// origin and down the bore.
///
/// It does not change where the weapon points -- a rigid body turned by an
/// angle about any point turns its bore by that angle, so the reticle sweeps
/// the same wherever this sits. What it picks is which part of the weapon
/// holds still on screen while the rest swings.
const SWAY_PIVOT_DOWN_BORE: f32 = 0.40;

/// The look-over, as a handful of poses with times against them.
///
/// Offset is camera space off the rest pose; the angles are the weapon's own,
/// turned about the trigger grip. Roll does most of the work -- the point is
/// to bring the receiver round where it can be seen, first one flank and then
/// the other.
///
/// The weapon carries no clips of its own. It is a static mesh posed by code,
/// with the hands following it through the grip IK, so this table is the
/// animation.
/// A hold repeats the key before it exactly. Carrying a little drift through
/// one looks like the right idea and is not: each segment eases in and out, so
/// a few degrees spread over most of a second become a slow creep that starts
/// and stops, and the eye reads it as the animation labouring rather than the
/// weapon being looked at. A hold that holds is still.
const INSPECT_KEYS: [(f32, Vec3, Vec3); 6] = [
    //  t     offset (x, y, z)                 yaw, pitch, roll (degrees)
    (0.00, Vec3::new(0.00, 0.00, 0.00), Vec3::new(0.0, 0.0, 0.0)),
    (0.17, Vec3::new(-0.18, 0.03, 0.00), Vec3::new(56.0, 6.0, -30.0)),
    (0.42, Vec3::new(-0.18, 0.03, 0.00), Vec3::new(56.0, 6.0, -30.0)),
    (0.68, Vec3::new(-0.07, 0.01, 0.06), Vec3::new(-7.0, -5.0, 62.0)),
    (0.86, Vec3::new(-0.07, 0.01, 0.06), Vec3::new(-7.0, -5.0, 62.0)),
    (1.00, Vec3::new(0.00, 0.00, 0.00), Vec3::new(0.0, 0.0, 0.0)),
];

/// Reads that table at `t`, smoothed between the two keys it falls between so
/// each turn arrives and leaves rather than starting and stopping.
fn inspect_pose(t: f32) -> (Vec3, Vec3) {
    let t = t.clamp(0.0, 1.0);
    let last = INSPECT_KEYS.len() - 1;
    let mut from = &INSPECT_KEYS[0];
    let mut to = &INSPECT_KEYS[last];
    for pair in INSPECT_KEYS.windows(2) {
        if t <= pair[1].0 {
            from = &pair[0];
            to = &pair[1];
            break;
        }
    }
    let span = (to.0 - from.0).max(1e-6);
    let k = ((t - from.0) / span).clamp(0.0, 1.0);
    let eased = k * k * (3.0 - 2.0 * k);
    (from.1.lerp(to.1, eased), from.2.lerp(to.2, eased))
}

/// The two points a pair of hands holds the weapon at, as one parameter.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GripSockets<'w, 's> {
    trigger: Query<'w, 's, &'static GlobalTransform, With<TriggerGripSocket>>,
    fore: Query<'w, 's, &'static GlobalTransform, With<ForeGripSocket>>,
}

/// Two resources as one parameter: gun_view_animation is at Bevy's sixteen.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct LocalPlayerState<'w> {
    moving: Res<'w, LocalPlayerMoving>,
    sprinting: Res<'w, LocalPlayerSprinting>,
}

pub(crate) fn gun_view_animation(
    grips: GripSockets,
    time: Res<Time>,
    aim_held: Res<AimHeld>,
    touch_aiming: Res<TouchAiming>,
    game_over: Res<GameOver>,
    keys: Res<ButtonInput<KeyCode>>,
    mut look_delta: ResMut<MouseLookDelta>,
    mut pose: ResMut<WeaponPose>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Lean), (With<FpsCamera>, Without<Gun>)>,
    spatial_query: SpatialQuery,
    mut query: Query<(&mut Transform, &GlobalTransform, &mut Gun), Without<FpsCamera>>,
    aim: Query<&GlobalTransform, (With<AimSocket>, Without<Gun>, Without<FpsCamera>)>,
    local: LocalPlayerState,
    active: Res<ActiveWeapon>,
    cycle: Res<WalkCycle>,
) {
    // Taken, not read: the single consumer clearing it is what lets each
    // input path write only when it has a delta, instead of every path having
    // to zero it and racing the others to do so.
    let look_delta = std::mem::take(&mut look_delta.0);
    let camera_single = camera.single().ok();
    let aiming = cursor.single().is_ok_and(|cursor| is_aiming(cursor, &aim_held, touch_aiming.0));

    // Reads the requested lean, not the eased camera value: keying off the
    // eased one makes the gun wait for the camera to settle before it starts
    // moving, instead of travelling alongside it.
    let leaning = camera_single
        .is_some_and(|(_, lean)| lean.target.abs() > GUN_READY_LEAN_CUTOFF);

    // Keyed off the requested lean, not the eased camera value. The swap only
    // uses the negative half of the range, so coming out of a right lean it
    // stays pinned at zero until the camera crosses the middle and then picks
    // up on the slowest part of the camera ease. That is the gun lagging the
    // press. Reading the input instead starts both at the same instant.
    let shoulder_target = camera_single.map_or(0.0, |(_, lean)| (-lean.target).clamp(0.0, 1.0));

    let wall_target = camera_single.map_or(0.0, |(camera_transform, _)| {
        let filter = SpatialQueryFilter::default();
        spatial_query
            .cast_ray(
                camera_transform.translation,
                camera_transform.forward(),
                WALL_GUN_RAISE_DISTANCE,
                true,
                &filter,
            )
            .map_or(0.0, |hit| {
                1.0 - (hit.distance / WALL_GUN_RAISE_DISTANCE).clamp(0.0, 1.0)
            })
    });
    for (mut transform, gun_global, mut gun) in &mut query {
        // The mark eases home while the kick runs after it. Two rates, so the
        // blow lands fast and the weapon comes back slowly, which is the
        // shape of the thing rather than one curve doing both.
        let kick = &active.def().recoil;
        gun.recoil_target *= ops::exp(-kick.weapon.settle_speed * time.delta_secs());
        let strike = 1.0 - ops::exp(-kick.strike_speed * time.delta_secs());
        gun.recoil += (gun.recoil_target - gun.recoil) * strike;
        if gun.reload_remaining > 0.0 {
            gun.reload_remaining = (gun.reload_remaining - time.delta_secs()).max(0.0);
            if gun.reload_remaining == 0.0 {
                gun.ammo = active.def().magazine;
            }
        }

        // The sights come down to reload. The weapon tilts 22 degrees and rolls
        // 20 to clear the magwell, which is not a thing to be looking through.
        let ads_target =
            if (aiming || force_ads()) && gun.reload_remaining <= 0.0 { 1.0 } else { 0.0 };
        // Latched on the rising edge of the request, not on ads_blend nearing
        // zero -- it decays towards zero and never arrives. ready_blend is
        // last frame's, before aiming drives it down. The max covers a raise
        // that starts before the last one finished coming down.
        if ads_target > 0.0 && !gun.ads_wanted {
            gun.ads_from_ready = gun.ready_blend.max(gun.ads_blend);
        }
        gun.ads_wanted = ads_target > 0.0;
        // Dropping the sights is the ready speed either way: what the rest
        // pose costs is the distance up, not the distance back down.
        let speed = &active.def().ads_speed;
        let ads_rate = if ads_target > 0.0 {
            speed.from_rest + (speed.from_ready - speed.from_rest) * gun.ads_from_ready
        } else {
            speed.from_ready
        };
        // A damped spring, not an exponential ease. The ease could only ever
        // arrive from one side: the weapon slid up to the sight line and
        // stopped dead on it, which is the one thing arms carrying three kilos
        // of rifle never do. This overshoots and comes back, twice, smaller
        // each time.
        //
        // Implicit Euler rather than the obvious `v += a*dt; x += v*dt`. That
        // one gains energy when `rate * dt` gets large and a weapon raised
        // during a frame spike would oscillate wider instead of settling.
        // This form is unconditionally stable at any timestep.
        let dt = time.delta_secs();
        let omega = ads_rate;
        let f = 1.0 + 2.0 * dt * ADS_SPRING_DAMPING * omega;
        let oo = omega * omega;
        let hoo = dt * oo;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let det_x = f * gun.ads_blend + dt * gun.ads_velocity + hhoo * ads_target;
        let det_v = gun.ads_velocity + hoo * (ads_target - gun.ads_blend);
        gun.ads_blend = det_x * det_inv;
        gun.ads_velocity = det_v * det_inv;
        // How fast the sights are moving, as a fraction of how fast this
        // weapon can move them. Oscillates through zero and decays, which is
        // what the miss below is after.
        //
        // Not `ads_blend - ads_target`, which was the first attempt: the
        // target is a step, so the instant the player let go of the sights it
        // jumped from nearly nothing to a whole unit and threw the weapon
        // seven degrees in one frame. Velocity is integrated state and cannot
        // jump, however abruptly the player changes their mind.
        gun.ads_settle_error = gun.ads_velocity / ads_rate.max(0.001);

        // Having aimed once means the player is working a sight line and will
        // likely aim again, so the gun waits near the middle instead of
        // swinging all the way back to the hip. Breaking into a sprint says
        // they are done with that sight line, so it drops to the normal rest.
        if aiming {
            gun.ready_armed = true;
        } else if local.sprinting.0 || game_over.0 {
            gun.ready_armed = false;
        }
        // Leaning takes the gun back to its shoulder: the swap is a deliberate
        // move to the other side of cover, and a centred ready pose has nothing
        // to swap. Releasing the lean settles back to ready.
        let ready_target =
            if gun.ready_armed && !aiming && !leaning { 1.0 } else { 0.0 };
        let ready_step = (GUN_READY_SPEED * time.delta_secs()).min(1.0);
        gun.ready_blend += (ready_target - gun.ready_blend) * ready_step;

        // Carried across the body, but only once the sprint has gone a second
        // without a shot. Aiming, reloading or standing still all mean the
        // weapon is wanted, so none of them reach it.
        gun.since_shot = (gun.since_shot + dt).min(f32::MAX / 2.0);
        let carrying = local.sprinting.0
            && local.moving.0
            && !aiming
            && gun.reload_remaining <= 0.0
            && gun.since_shot >= SPRINT_CARRY_DELAY;
        let carry_step = (SPRINT_CARRY_SPEED * dt).min(1.0);
        gun.sprint_blend += (f32::from(carrying) - gun.sprint_blend) * carry_step;

        // Nothing asked of the weapon for long enough and it gets looked over.
        // Walking still counts as idle -- standing perfectly still is rare in
        // a game where the dead come to you -- but running does not.
        let busy = aiming
            || local.sprinting.0
            || gun.reload_remaining > 0.0
            || gun.since_shot < SPRINT_CARRY_DELAY;
        if busy {
            gun.idle_elapsed = 0.0;
        } else {
            gun.idle_elapsed += dt;
        }
        // T asks for it directly, so nobody has to stand around to see it.
        // Still refused while busy: the weapon is in the middle of something.
        let asked = keys.just_pressed(INSPECT_KEY);
        let wait = INSPECT_WAIT + gun.inspect_done as f32 * INSPECT_WAIT_EXTRA;
        if gun.inspect_t <= 0.0 && !busy && (asked || gun.idle_elapsed >= wait) {
            gun.inspect_t = f32::EPSILON;
        }
        if gun.inspect_t > 0.0 {
            gun.inspect_t += dt / INSPECT_SECONDS;
            // Held at the end rather than wound back to the start.
            //
            // A frame lands wherever dt puts it, so the last one before the
            // end sits at something like 0.97 -- a twentieth of the way back
            // from the final look, still turned. Winding t to zero there read
            // as the pose jumping the rest of the way home in one frame, and
            // that was the jolt. Clamped, the sequence always finishes on its
            // own last key.
            if gun.inspect_t >= 1.0 {
                gun.inspect_t = 1.0;
                gun.inspect_done += 1;
                gun.idle_elapsed = 0.0;
            }
            // Cancelled is different: the player wants to shoot, so it stops
            // where it is and the blend takes it away.
            if busy {
                gun.inspect_t = 0.0;
                gun.idle_elapsed = 0.0;
            }
        }
        let inspecting = gun.inspect_t > 0.0 && gun.inspect_t < 1.0;
        if !inspecting && gun.inspect_blend < 0.01 {
            gun.inspect_t = 0.0;
        }
        // Three rates, because the three ways out are not the same thing. Going
        // in is a decision and takes its time; finishing is the weapon coming
        // home; being cancelled is the player wanting to shoot, and that one
        // gets out of the way.
        let inspect_rate = if inspecting {
            INSPECT_IN_SPEED
        } else if busy {
            INSPECT_OUT_SPEED
        } else {
            INSPECT_RETURN_SPEED
        };
        gun.inspect_blend +=
            (f32::from(inspecting) - gun.inspect_blend) * (inspect_rate * dt).min(1.0);
        let (want_offset, want_angles) = inspect_pose(gun.inspect_t);
        // Implicit Euler, as everywhere else here: stable at any timestep,
        // where `v += a*dt; x += v*dt` gains energy on a frame spike and a
        // weapon caught by one would swing wider instead of settling.
        let omega = INSPECT_SPRING_HZ;
        let f = 1.0 + 2.0 * dt * INSPECT_SPRING_DAMPING * omega;
        let hoo = dt * omega * omega;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let (x, v) = (gun.inspect_offset, gun.inspect_offset_vel);
        gun.inspect_offset = (x * f + v * dt + want_offset * hhoo) * det_inv;
        gun.inspect_offset_vel = (v + (want_offset - x) * hoo) * det_inv;
        let (x, v) = (gun.inspect_angles, gun.inspect_angles_vel);
        gun.inspect_angles = (x * f + v * dt + want_angles * hhoo) * det_inv;
        gun.inspect_angles_vel = (v + (want_angles - x) * hoo) * det_inv;
        let (inspect_offset, inspect_angles) = (gun.inspect_offset, gun.inspect_angles);

        // One roll per disturbance, so each return lands somewhere new.
        let disturbed = aiming || leaning || local.sprinting.0;
        if disturbed {
            if !gun.settle_rolled {
                gun.settle_rolled = true;
                let (offset, twist) = settle_jitter(time.elapsed_secs());
                gun.settle_jitter = offset;
                gun.settle_twist_jitter = twist;
            }
        } else {
            gun.settle_rolled = false;
        }

        // Chased rather than used raw. The offset lands straight on the final
        // translation with nothing else smoothing it, so swapping the value in
        // one frame moves the gun in one frame.
        let settle_step = (GUN_SETTLE_SPEED * time.delta_secs()).min(1.0);
        let settle_gap = gun.settle_jitter - gun.settle_current;
        gun.settle_current += settle_gap * settle_step;
        let twist_gap = gun.settle_twist_jitter - gun.settle_twist_current;
        gun.settle_twist_current += twist_gap * settle_step;

        let wall_raise_step = (WALL_GUN_RAISE_SPEED * time.delta_secs()).min(1.0);
        gun.wall_raise += (wall_target - gun.wall_raise) * wall_raise_step;

        let shoulder_step = (GUN_SHOULDER_SPEED * time.delta_secs()).min(1.0);
        gun.shoulder_blend += (shoulder_target - gun.shoulder_blend) * shoulder_step;


        // Late to a turn, not rolled by one. Negated because the weapon is
        // still aimed back where the view was, opposite the way it just moved.
        let lag = -look_delta;
        let sway_target = (lag * LOOK_SWAY_SENSITIVITY)
            .clamp(Vec2::splat(-LOOK_SWAY_MAX_DEG), Vec2::splat(LOOK_SWAY_MAX_DEG));
        // The same damped spring the sights ride: an exponential ease can only
        // arrive from one side, so the weapon stopped dead whenever a turn
        // ended. The target here is how fast the view is turning right now,
        // not a step, so it falls to zero the instant the turn stops and the
        // spring is what carries the weapon on past it.
        let omega = LOOK_SWAY_FREQUENCY;
        let f = 1.0 + 2.0 * dt * LOOK_SWAY_DAMPING * omega;
        let hoo = dt * omega * omega;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let (x, v) = (gun.look_sway, gun.look_sway_velocity);
        gun.look_sway = (x * f + v * dt + sway_target * hhoo) * det_inv;
        gun.look_sway_velocity = (v + (sway_target - x) * hoo) * det_inv;

        let kick = &active.def().recoil;
        let timings = gun_pose_timings(&gun, active.def(), &cycle);

        // Faded out by ads_blend, the way the positional jitter already is
        // through resting: a weapon canted on its own bore would take the
        // sights off the camera axis. The optical correction fades the other
        // way, since it is only wanted while looking through the glass.
        let settle_twist = gun.settle_twist_current * (1.0 - gun.ads_blend);
        // Muzzle up and rolled onto its side, so the magazine well comes round
        // to face the hand that has to empty it.
        //
        // Turned about the firing grip rather than about the mesh origin. That
        // origin is on the bore at the muzzle, so turning about it pins the
        // muzzle and swings the stock down instead -- the weapon drops its
        // tail rather than lifting its nose, which is the wrong half moving.
        // Rotating about a point means turning, then putting back what the
        // turn moved that point by.
        let reload_turn = Quat::from_rotation_x(RELOAD_TILT_DEG.to_radians() * timings.dip)
            * Quat::from_rotation_z(RELOAD_ROLL_DEG.to_radians() * timings.dip);
        let grip_in_gun = grips
            .trigger
            .single()
            .map(|socket| socket.reparented_to(gun_global).translation)
            .unwrap_or(Vec3::ZERO);
        let carry = &active.def().sprint_carry;
        let sprint_turn = Quat::IDENTITY.slerp(
            Quat::from_rotation_y(carry.yaw_deg.to_radians())
                * Quat::from_rotation_x(carry.pitch_deg.to_radians())
                * Quat::from_rotation_z(carry.roll_deg.to_radians()),
            gun.sprint_blend,
        );
        // One turn for both poses, so the correction below is exact rather
        // than two approximations added together.
        let inspect_turn = Quat::IDENTITY.slerp(
            Quat::from_rotation_y(inspect_angles.x.to_radians())
                * Quat::from_rotation_x(inspect_angles.y.to_radians())
                * Quat::from_rotation_z(inspect_angles.z.to_radians()),
            gun.inspect_blend,
        );
        let pose_turn = reload_turn * sprint_turn;
        let pose_swing = grip_in_gun - pose_turn * grip_in_gun;

        let base_rotation = Quat::from_rotation_z(settle_twist.to_radians())
            * pose_turn
            * Quat::IDENTITY.slerp(gun.optical_fix, gun.ads_blend);

        // Back to the rest height first. The ready pose deliberately sits a
        // hand lower -- muzzle down, waiting -- and the look-over was being
        // added on top of wherever the weapon happened to be, so starting it
        // from ready put the whole gesture ten centimetres under where
        // starting it from rest did. It is the same gesture either way.
        let posture = gun
            .base_offset
            .lerp(active.def().ready_offset, gun.ready_blend)
            .lerp(gun.base_offset, gun.inspect_blend);
        let resting = posture
            + gun.settle_current
            + active.def().sprint_carry.offset * gun.sprint_blend
            + inspect_offset * gun.inspect_blend;
        // Sighted means Socket_Aim on the camera axis at this weapon.s eye
        // relief; the mesh says where that point is, so the offset is a
        // subtraction rather than a dialled-in vector. Where it sits inside
        // the gun never animates, so last frame.s copy of that is fine.
        //
        // Turned before it is subtracted. The socket sits 0.91m down the
        // weapon, so a straight subtraction only lands it on the axis while
        // the weapon is square to the camera -- the moment the pose turns it,
        // that lever throws the sight off by the turn times its length. Only
        // base_rotation is taken out: sway and recoil are meant to show.
        let sighted = aim
            .single()
            .map(|aim_global| {
                let in_gun = aim_global.reparented_to(gun_global);
                Vec3::new(0.0, 0.0, -active.def().ads_eye_relief) - base_rotation * in_gun.translation
            })
            .unwrap_or(resting);
        // Slides off the sight line and corrects on, without turning.
        //
        // This was a rotation, which cannot work here: the reticle takes its
        // position from the bore direction, and rotating a rigid weapon about
        // any pivot turns the bore by the full angle. Moving the pivot forward
        // lengthens the stock's lever but does nothing for the bore, so the
        // sight picture swung by the whole miss -- and a scope multiplies it,
        // 1.4 degrees filling a third of a 12x view.
        //
        // A translation leaves the bore pointing where it was. The weapon
        // visibly overshoots and settles; the reticle does not move at all,
        // because nothing it reads has changed.
        //
        // Across the approach, not along it: along was the first attempt and
        // read as one straight line with a wobble on the end, because the
        // weapon was never anywhere it had not been going anyway.
        let travel = sighted - resting;
        let across = Dir3::new(travel.cross(Vec3::NEG_Z)).ok();
        let settle_offset = across
            .map(|across| *across * (active.def().ads_settle_miss_m * gun.ads_settle_error))
            .unwrap_or(Vec3::ZERO);
        // And a tilt to go with the slide. A weapon whose bore stayed exactly
        // true while the rest of it settled read as fake, because the reticle
        // takes its place from the bore and a reticle that never moves is a
        // weapon that was never really moving.
        //
        // Kept small, and small by magnification rather than by mass: the
        // reticle sweeps by the whole tilt and a scope multiplies it. The
        // weapon with the most glass needs the least angle, which is the
        // reverse of what the slide above wants.
        let settle_tilt = across
            .map(|axis| {
                Quat::from_axis_angle(
                    *axis,
                    (active.def().ads_settle_miss_deg * gun.ads_settle_error).to_radians(),
                )
            })
            .unwrap_or(Quat::IDENTITY);

        let base_pos =
            resting.lerp(sighted, gun.ads_blend) - Vec3::Z * view_pullback() + settle_offset;

        // Breathing and two tremors, read off the clock rather than integrated,
        // so it is the same motion whatever the frame rate and two weapons
        // picked up a second apart are not in step.
        //
        // Pitch carries the breath because that is the axis a chest moves the
        // muzzle on. The tremors take yaw as well, and sit at rates with no
        // common multiple so the loop never lands where the eye can catch it.
        let clock = time.elapsed_secs();
        let tau = std::f32::consts::TAU;
        let breath = ops::sin(tau * HOLD_BREATH_HZ * clock);
        let tremor_a = ops::sin(tau * HOLD_TREMOR_A_HZ * clock);
        let tremor_b = ops::sin(tau * HOLD_TREMOR_B_HZ * clock + 1.3);
        // Braced against the shoulder, most of it goes; what is left the glass
        // makes the most of.
        // Each weapon says how much of this its sights show, because a red dot
        // and a 24x scope do not magnify the same wander by the same amount
        // and are not braced the same way either.
        let ads = gun.ads_blend.clamp(0.0, 1.0);
        let hold = 1.0 - ads * (1.0 - active.def().hold_ads_fraction);
        let hold_yaw = (HOLD_BREATH_YAW_DEG * breath + HOLD_TREMOR_YAW_DEG * tremor_a) * hold;
        let hold_pitch = (HOLD_BREATH_PITCH_DEG * breath + HOLD_TREMOR_PITCH_DEG * tremor_b) * hold;
        let hold_turn = Quat::from_rotation_y(hold_yaw.to_radians())
            * Quat::from_rotation_x(hold_pitch.to_radians());
        // Handed to the view, which takes a share of the same numbers rather
        // than running its own sines and slowly drifting out of step.
        pose.hold_yaw_deg = hold_yaw;
        pose.hold_pitch_deg = hold_pitch;
        pose.sighted = ads;

        let outer = Quat::from_rotation_x(WALL_GUN_RAISE_ROT_DEG.to_radians() * gun.wall_raise);
        let sway = Quat::from_rotation_y(gun.look_sway.x.to_radians())
            * Quat::from_rotation_x(gun.look_sway.y.to_radians());
        let inner = settle_tilt
            * Quat::from_rotation_z(timings.bob_roll.to_radians())
            * Quat::from_rotation_x(timings.bob_pitch.to_radians())
            * Quat::from_rotation_y(kick.weapon.yaw_deg.to_radians() * gun.recoil_jitter.x * timings.recoil_t)
            * Quat::from_rotation_z(kick.weapon.roll_deg.to_radians() * gun.recoil_jitter.y * timings.recoil_t)
            * Quat::from_rotation_x(kick.weapon.pitch_deg.to_radians() * timings.recoil_t)
            * base_rotation;

        // Turning about a point means turning, then putting back what the turn
        // moved that point by -- the same arithmetic reload_swing uses. Taken
        // through the rotations either side, since they carry the pivot
        // before and after the sway reaches it.
        let turned = inner * (Vec3::NEG_Z * SWAY_PIVOT_DOWN_BORE);
        let sway_swing = outer * (turned - sway * turned);
        // About the firing grip, like the reload and the carry: a hand holding
        // a rifle is the thing that stays put while the muzzle wanders.
        let held = inner * grip_in_gun;
        let hold_swing = outer * sway * (held - hold_turn * held);

        // Between the hands, not at the trigger. Rolling about the firing grip
        // alone swings the muzzle through an arc of whatever the grip sits
        // below the bore, which reads as the weapon being thrown around rather
        // than turned over. Both hands turning it together pivot it near its
        // own middle, and that is what the gesture looks like.
        let fore_in_gun = grips
            .fore
            .single()
            .map(|socket| socket.reparented_to(gun_global).translation)
            .unwrap_or(grip_in_gun);
        let between_hands = inner * ((grip_in_gun + fore_in_gun) * 0.5);
        let inspect_swing =
            outer * sway * hold_turn * (between_hands - inspect_turn * between_hands);

        let swapped_x = base_pos.x * (1.0 - 2.0 * gun.shoulder_blend);
        transform.translation = Vec3::new(swapped_x, base_pos.y, base_pos.z)
            + pose_swing
            + sway_swing
            + hold_swing
            + inspect_swing
            + Vec3::new(
                timings.bob_x,
                kick.weapon.lift * timings.recoil_t + RELOAD_LIFT * timings.dip - timings.bob_y
                    + WALL_GUN_RAISE_HEIGHT * gun.wall_raise,
                kick.weapon.slide_back * timings.recoil_t,
            );

        transform.rotation = outer * sway * hold_turn * inspect_turn * inner;
    }
}


/// Where the gun settles, nudged a little each time. Coming to rest on the
/// exact same millimetre after every aim is what makes the pose read as a
/// still image rather than a held weapon.
fn settle_jitter(seed: f32) -> (Vec3, f32) {
    let axis = |salt: f32| (ops::sin(seed * salt) * 43758.547).fract().abs() * 2.0 - 1.0;
    (
        Vec3::new(
            axis(12.9898) * GUN_SETTLE_JITTER.x,
            axis(78.233) * GUN_SETTLE_JITTER.y,
            axis(37.719) * GUN_SETTLE_JITTER.z,
        ),
        axis(94.673) * GUN_SETTLE_TWIST_DEG,
    )
}

/// One walk cycle, shared by the weapon and the view.
///
/// Both have to ride the *same* cycle or they drift against each other and the
/// weapon stops following the step it is supposed to be following. `phase`
/// advances only while moving, `blend` fades the whole thing in and out so
/// stopping does not cut it off mid-stride, and `aim_damp` is how much of it
/// survives the sights coming up.
#[derive(Resource, Default)]
pub(crate) struct WalkCycle {
    pub(crate) phase: f32,
    pub(crate) blend: f32,
    pub(crate) aim_damp: f32,
    /// 1.0 at a walk, higher at a sprint.
    pub(crate) gait: f32,
}

impl WalkCycle {
    /// What the weapon and the view both scale their bob by.
    pub(crate) fn amount(&self) -> f32 {
        self.blend * self.aim_damp * self.gait
    }
}

pub(crate) fn advance_walk_cycle(
    time: Res<Time>,
    moving: Res<LocalPlayerMoving>,
    sprinting: Res<LocalPlayerSprinting>,
    gun: Query<&Gun>,
    mut cycle: ResMut<WalkCycle>,
) {
    let dt = time.delta_secs();
    let blend_target = if moving.0 { 1.0 } else { 0.0 };
    cycle.blend += (blend_target - cycle.blend) * (GUN_BOB_BLEND_SPEED * dt).min(1.0);

    // A sprint is a different gait, not a louder walk. Raising the amplitude
    // while leaving the cadence at walking pace is the thing that reads as
    // wrong -- the steps stop matching the ground going past.
    let gait_target = if sprinting.0 { SPRINT_BOB_GAIT } else { 1.0 };
    cycle.gait += (gait_target - cycle.gait) * (BOB_GAIT_BLEND_SPEED * dt).min(1.0);

    if moving.0 {
        // Wrapped rather than left to grow: at 8 rad/s an hour of walking puts
        // the phase near 29000, where an f32 step of 0.13 starts to quantise.
        cycle.phase = (cycle.phase + GUN_BOB_SPEED * cycle.gait * dt)
            .rem_euclid(std::f32::consts::TAU);
    }

    cycle.aim_damp = gun
        .single()
        .map(|gun| 1.0 - gun.ads_blend * (1.0 - AIM_BOB_MULTIPLIER))
        .unwrap_or(1.0);
}
