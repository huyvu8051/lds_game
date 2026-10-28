use avian3d::prelude::*;
use bevy::app::AnimationSystems;
use bevy::transform::TransformSystems;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window, WindowMode};
use bevy::winit::{UpdateMode, WinitSettings};
use shared::{
    ClearAllZombies, ClientInput, find_animation_root, FireShot, GameOver, GameStateEvent, Health,
    HEAD_DAMAGE, HITBOX_BELLY_MIN_Y, HITBOX_CHEST_MIN_Y, HITBOX_HEAD_MIN_Y, HITBOX_LIMB_LATERAL_MIN,
    KnockbackVelocity, NetPlayer, Player, PlayerDied, ZombieMotion,
    ZombieAnimationPlayer,
    PlayerStance, RequestRestart, Scenario, SetAutoMaintainZombies, SetDamageMultiplier, SetFreezeZombies, SetZombieCount, ShotConfirmed, ShotFx,
    SpawnMoreZombies, Zombie, ZombieAttackTimer, ZombieHitboxBones, ZombieKind, ZombieShowcaseClip, wire_zombie_hitbox_bones,
    zombie_kind_clip_count, zombie_kind_mesh_path, zombie_kind_anim_path, MAX_SHOT_DISTANCE,
    HEAD_HITBOX_RADIUS, LIMB_HITBOX_RADIUS, TORSO_HITBOX_RADIUS,
    resolve_bone_world_positions, zombie_hit_damage_ray,
    ZOMBIE_ATTACK_INTERVAL, ZOMBIE_OUTER_HEIGHT, ZOMBIE_OUTER_RADIUS,
    zombie_pose_for, ZombieAnim, ZombiePose, ZombieWalkVariant, ZOMBIE_KIND_WEIGHTS, ZOMBIE_MAX_HEALTH, ZOMBIE_STOP_DISTANCE,
};

#[cfg(target_vendor = "apple")]
pub mod cadisplaylink;

mod weapon;
mod items;
mod inventory;
mod zombie_locomotion;
mod inventory_screen;
mod loadout_hud;
use weapon::*;
use items::*;
use inventory::*;
use zombie_locomotion::*;
use inventory_screen::*;
use loadout_hud::*;
mod dismember;
use dismember::*;
mod auto_lean;
mod lightmap;
mod autopilot;
use auto_lean::*;
use autopilot::{autopilot_pick_target, autopilot_think, Autopilot, AutopilotIntent, AutopilotLookOnly, AutopilotState};

mod touch_input;
use touch_input::*;

mod credits;
mod settings_menu;
use settings_menu::*;

mod movement_combat;
use movement_combat::*;

mod zombie_client;
use zombie_client::*;

mod vfx;
use vfx::*;

mod audio;
use audio::*;

mod blood;
mod bullet_holes;
use blood::*;
use bullet_holes::*;

mod zombie_reaction;
use zombie_reaction::*;

mod gun_view;
use gun_view::*;

mod map;

mod screen_effects;
use screen_effects::*;

mod grip_ik;
mod muzzle_fx;
mod explosion;
mod ragdoll;
mod viewmodel_body;
use grip_ik::*;
use muzzle_fx::*;
use explosion::*;
use ragdoll::*;
use viewmodel_body::*;

mod dissolve;
use dissolve::*;

mod damage_overlay;
use damage_overlay::*;

mod combat;

mod hud;
use hud::*;

mod character_tuning;
use character_tuning::*;

mod graphics_settings;
use graphics_settings::*;

mod scene_setup;
mod magazine_drop;
mod shell_casings;
use scene_setup::*;
use magazine_drop::*;
use shell_casings::*;

mod game_flow;
use game_flow::*;

mod f9_debug;
use f9_debug::*;

mod navigation;
use navigation::*;

mod sim;
use sim::SimPlugin;

mod zombie_nav;

mod settings_persistence;
use settings_persistence::*;

mod splash_screen;
use splash_screen::*;

/// How far the eye sits behind the player's own eye point. The gun and the
/// body subtract it again, so pulling back moves only the camera.
pub(crate) fn view_pullback() -> f32 {
    std::env::var("VIEW_PULLBACK").ok().and_then(|v| v.parse().ok()).unwrap_or(0.13)
}

/// Low ready: held near the middle and a little forward of the hip rest, so
/// coming back onto the sights is a short move instead of a full swing across
/// the body. Sits just right of centre, on the firing-hand side, and
/// muzzle-down, which is where a rifle hangs when its owner is waiting.
const GUN_READY_SPEED: f32 = 7.0;
const GUN_READY_LEAN_CUTOFF: f32 = 0.12;
const GUN_SETTLE_JITTER: Vec3 = Vec3::new(0.030, 0.024, 0.020);
/// Degrees about the bore the weapon can come to rest turned by.
const GUN_SETTLE_TWIST_DEG: f32 = 3.0;
const GUN_SETTLE_SPEED: f32 = 3.0;
const GUN_SHOULDER_SPEED: f32 = 9.0;

/// A sprint this long with nothing fired takes the weapon across the body,
/// the way it is carried when nobody expects to use it in the next second.
/// Firing drops it straight back to the ready pose and starts the clock over,
/// so a player shooting on the move never leaves the pose they can shoot from.
/// How far out of position the weapon may be and still fire.
///
/// Two poses take the muzzle a long way off the view axis: carried across the
/// body at a sprint, and raised out of a wall. A hipfire shot converges on the
/// crosshair by design -- the muzzle sits right of and below the eye, so a
/// bore-parallel shot would miss the cross at every range -- but that only
/// holds while the weapon is roughly pointing where the player is looking. At
/// these two poses it stops holding, and the round leaves a barrel aimed
/// sideways to fly straight ahead.
///
/// So the shot waits for the weapon. The two waits are not the same, though:
/// the carry is the player's own pose and pressing fire drops it, while the
/// wall is a wall and nothing the player presses will move it.
const WEAPON_ON_AXIS_BLEND: f32 = 0.25;

/// Left alone for this long and the weapon gets looked over, the way someone
/// with nothing to shoot at checks what they are holding.
///
/// Each round waits longer than the last, so a player standing still for a
/// while is not shown the same thing on a loop.
/// Asks for the look-over without waiting for it. F is the usual key for this
/// and the HUD already has it.
const INSPECT_KEY: KeyCode = KeyCode::KeyT;
const INSPECT_WAIT: f32 = 9.0;
const INSPECT_WAIT_EXTRA: f32 = 7.0;
const INSPECT_SECONDS: f32 = 3.6;
/// In, home, and out of the way -- three different endings.
///
/// The sequence never returns to the rest pose itself; it ends on the last
/// look and RETURN carries the weapon back, so the gesture is one movement
/// rather than a braking followed by a fade. OUT is for a cancellation: the
/// player wants to shoot, and making them wait for an animation to finish is
/// how this feature becomes a trap.
/// The weight behind the gesture.
///
/// The keyed table says where the weapon is wanted; a spring says what it
/// costs to get it there. Without one, every segment boundary is a corner --
/// smoothstep brings each move to a stop and starts the next from nothing --
/// and the arm reads as weightless. With one, the weapon arrives a beat late
/// and carries a little past, and the corners are rounded off for free.
///
/// Well over 1.0, so it no longer carries past the mark at all -- it leans
/// into each move and settles onto it, which is the heavier read. The cost is
/// lag: overdamped, the weapon takes longer to arrive, and the keyed segments
/// are two thirds of a second. FREQUENCY is what to raise if it starts
/// trailing the table rather than following it.
const INSPECT_SPRING_HZ: f32 = 19.0;
const INSPECT_SPRING_DAMPING: f32 = 2.16;

const INSPECT_IN_SPEED: f32 = 5.0;
const INSPECT_RETURN_SPEED: f32 = 6.5;
const INSPECT_OUT_SPEED: f32 = 14.0;

const SPRINT_CARRY_DELAY: f32 = 1.0;
const SPRINT_CARRY_SPEED: f32 = 6.0;

const GUN_BOB_SPEED: f32 = 8.0;
const GUN_BOB_AMOUNT_X: f32 = 0.015;
const GUN_BOB_AMOUNT_Y: f32 = 0.012;
const GUN_BOB_BLEND_SPEED: f32 = 6.0;
/// Radians the weapon's cant trails the body's. Arms are compliant; without
/// the lag the weapon rolls in exact step with the hips and reads as welded.
/// How fast the weapon catches up with the gait.
///
/// Call of Duty runs its viewmodel through a damped tracker against the view
/// rather than writing a curve onto it -- `swayOffset` and `swayAngles` are
/// DiffTrack-ed toward the per-frame view delta. This is the same idea
/// applied to the walk: the step writes a target, the weapon chases it.
///

/// How far the weapon trails the body through a step, in radians of the
/// gait. Arms are compliant; a weapon that cants in exact step with the hips
/// reads as welded to them.
const GUN_BOB_LAG: f32 = 0.35;
/// Degrees the weapon cants towards the side the player is stepping, about
/// the bore. Not a lean of the body -- the shoulders stay square and the
/// rifle rolls in the hands, which is what a man does carrying one at the
/// ready while crossing sideways.
///
/// Hip fire only. The sights have to stay level with the eye, and a canted
/// weapon in ADS puts the reticle somewhere the barrel is not.
const GUN_STRAFE_ROLL_DEG: f32 = 3.2;
/// Degrees the muzzle drops walking forward, and lifts walking back. The
/// weapon leads the body into a step and trails it out of one, which is the
/// same compliance `GUN_BOB_LAG` is about -- arms are not a clamp.
///
/// Smaller than the cant: this one is read down the barrel, so it moves the
/// point of aim where a roll does not.
const GUN_STEP_PITCH_DEG: f32 = 1.8;
/// How fast both follow the stick. Slower than the step itself, so changing
/// direction takes the weapon through level rather than snapping it.
const GUN_STEP_LEAN_SPEED: f32 = 7.0;
/// The share of the step lean that survives the sights coming up, the way
/// `AIM_BOB_MULTIPLIER` works for the bob.
///
/// Not zero any more: a weapon that goes perfectly rigid the moment it
/// reaches the eye reads as a prop on a rail. But kept well down, because
/// this is the one place the lean costs something -- the pitch is read along
/// the barrel, so in the sights it moves the point of aim rather than just
/// the picture.
const AIM_STEP_LEAN_MULTIPLIER: f32 = 0.30;
const GUN_BOB_PITCH_DEG: f32 = 0.55;
/// Cadence and amplitude together, so a sprint is a different gait rather
/// than a walk with the volume turned up.
const SPRINT_BOB_GAIT: f32 = 1.35;
const BOB_GAIT_BLEND_SPEED: f32 = 4.0;

/// Deliberately a third of the weapon's. If the view moves as much as the
/// weapon, the weapon looks still and the world looks like it is shaking --
/// what sells a step is the weapon moving *within* the frame.
const CAMERA_BOB_AMOUNT_X: f32 = 0.005;
const CAMERA_BOB_AMOUNT_Y: f32 = 0.008;
/// Roll sells walking harder than translation does, and makes people sick
/// faster too. Under half a degree.
const CAMERA_BOB_ROLL_DEG: f32 = 0.40;
const AIM_BOB_MULTIPLIER: f32 = 0.35;

/// How far the weapon lags behind a turn. SENSITIVITY has to keep an ordinary
/// turn inside MAX_DEG, or every movement alike pins it at full swing.
///
/// SENSITIVITY is degrees of lag per radian the view turned, so sway follows
/// the view rather than the mouse and stays right at any sensitivity or zoom.
const LOOK_SWAY_SENSITIVITY: f32 = 50.0;
const LOOK_SWAY_MAX_DEG: f32 = 3.5;

/// The sway spring. FREQUENCY sets how quickly the weapon answers a turn;
/// DAMPING how much of the answer is overshoot, and lower means more of it:
/// under 1.0 the weapon passes the mark and comes back. 0.65 leaves under a
/// hundredth of it -- enough that the weapon does not stop dead, not enough
/// to read as a wobble through the glass.
/// The weapon is never still, because the body holding it is not.
///
/// Three layers, because one sine reads as a machine. Breathing is slow and
/// almost all pitch -- fifteen a minute, the muzzle rising on the way in and
/// settling on the way out. Over it, two tremors at rates that share no common
/// multiple, so the pattern never repeats where anyone can see it repeat; 1.7
/// and 2.9 are the postural correction a standing body makes, not the 8-12Hz
/// physiological tremor, which is far too small and far too fast to draw.
///
/// Degrees, all of them, turned about the firing grip.
const HOLD_BREATH_HZ: f32 = 0.25;
const HOLD_BREATH_PITCH_DEG: f32 = 0.42;
const HOLD_BREATH_YAW_DEG: f32 = 0.09;
const HOLD_TREMOR_A_HZ: f32 = 1.7;
const HOLD_TREMOR_B_HZ: f32 = 2.9;
const HOLD_TREMOR_YAW_DEG: f32 = 0.030;
const HOLD_TREMOR_PITCH_DEG: f32 = 0.022;

/// The share of the hold the view takes as well as the weapon.
///
/// A body that moves the rifle moves the eye looking down it, so some of this
/// has to be in the camera or the weapon reads as drifting on its own. Kept
/// well under the weapon's, since the two add -- the gun hangs off the camera
/// -- and the part worth seeing is the muzzle wandering against the world,
/// not the world wandering.
const CAMERA_HOLD_FRACTION: f32 = 0.35;

/// How much of the shock a cheek weld takes out of the view. A braced
/// shooter with the stock in the shoulder and the eye on the glass sees far
/// less of it than one firing from the hip, and ADS is also when the player
/// most needs to read the sight picture.
const ADS_SHOCK_DAMP: f32 = 0.6;

/// What the weapon pose is telling the rest of the game, published once.
///
/// Everything that reacts to being aimed used to ask the aim button instead,
/// and the button and the weapon disagree: holding it through a reload leaves
/// the sights down -- the weapon has come off the eye to work the magazine --
/// while the button still says yes. The view stayed zoomed on nothing and the
/// walk stayed slow for a weapon that was not being aimed.
///
/// `sighted` is the blend, not a flag, so a caller that can ease does and one
/// that needs a yes or no picks its own line.
#[derive(Resource, Default)]
pub(crate) struct WeaponPose {
    pub(crate) sighted: f32,
    pub(crate) hold_yaw_deg: f32,
    pub(crate) hold_pitch_deg: f32,
}

const LOOK_SWAY_FREQUENCY: f32 = 12.0;
const LOOK_SWAY_DAMPING: f32 = 0.65;

const WALL_GUN_RAISE_DISTANCE: f32 = 1.5;
const WALL_GUN_RAISE_HEIGHT: f32 = 0.18;
const WALL_GUN_RAISE_ROT_DEG: f32 = 90.0;
/// Where the look has to be before the weapon goes down instead of up to
/// clear a wall ahead, in degrees below level, and where it is fully
/// committed to that.
///
/// Up is the default because that is where the room usually is. Looking down,
/// it is not: there is floor in front of the muzzle and open air behind the
/// stock, so the weapon drops away from the wall instead of being hoisted
/// into it. The band between the two is what keeps the flip from being a
/// switch thrown at exactly level.
const WALL_RAISE_DOWN_START_DEG: f32 = 4.0;
const WALL_RAISE_DOWN_FULL_DEG: f32 = 20.0;
const WALL_GUN_RAISE_SPEED: f32 = 10.0;
/// How often the wall in front is measured, in seconds. One ray into Avian's
/// whole collision tree was running every frame to answer a question about
/// walking pace. Twelve times a second is faster than anyone closes the last
/// metre, and `WALL_GUN_RAISE_SPEED` smooths the steps out: at a 100ms time
/// constant an 83ms sample interval never shows.
const WALL_PROBE_INTERVAL: f32 = 1.0 / 12.0;
/// Where the side probes point, in degrees off straight abeam, mirrored onto
/// both sides -- so six rays covering everything from 45 degrees ahead round
/// to 45 behind on each flank.
///
/// Two rays straight out to the sides missed a corner sitting ahead and to
/// the left, which is exactly where a player tucks in and exactly where the
/// weapon has nowhere to go.
///
/// Measured off abeam rather than off forward because the camera's right
/// stays horizontal when the player looks straight up, and its forward does
/// not -- a fan built on a vertical forward collapses.
const WALL_SIDE_ANGLES_DEG: [f32; 3] = [-45.0, 0.0, 45.0];
/// How far the side probes look.
const WALL_SIDE_DISTANCE: f32 = 1.2;
/// Close enough beside the player to count, as a share of the above -- 0.3 of
/// 1.2m is anything inside 85cm. Near enough that the weapon has nowhere to
/// swing on that side, which is the thing being detected.
const WALL_SIDE_PROXIMITY: f32 = 0.3;
/// How close a wall ahead has to be before it owns the weapon outright and
/// the side rule stands down, as a share of `WALL_GUN_RAISE_DISTANCE`.
///
/// The two rules must never fire on the same wall: one asks the weapon up and
/// the other asks it in, and a flat face in front is seen by both. The
/// forward ray finds it at `d`, and the innermost ray of the fan is 45
/// degrees off, so it finds the same face at `d / cos 45` = 1.414d. The fan
/// trips at 0.85m, so it reaches that wall once `d` is 0.6m -- which reads
/// 0.6 here. Half is that with room to spare, and leaves the side rule
/// working for anything beyond 75cm ahead.
const WALL_AHEAD_OWNS: f32 = 0.5;
/// How much further the wall has to fall away before the weapon goes back
/// out, as the same share -- 0.08 of 1.2m is about 10cm of slack.
///
/// Without it a player standing on the boundary flips the pose every time the
/// probe twitches, which is the same thing AUTOPILOT_ADS_DISTANCE_MARGIN is
/// for: a bare comparison chatters on anything sitting exactly on it.
const WALL_SIDE_HYSTERESIS: f32 = 0.08;

/// The weapon rises to reload rather than dropping out of the way, so the
/// magazine coming out of the well is in frame while it falls.
const RELOAD_LIFT: f32 = 0.10;
/// Muzzle up, about the firing grip. Degrees.
const RELOAD_TILT_DEG: f32 = 22.0;
/// Roll about the bore during a reload. Negative brings the magazine well
/// round towards the weak hand, which is the hand that pulls the magazine.
const RELOAD_ROLL_DEG: f32 = -20.0;

const MUZZLE_FLASH_LIFETIME: f32 = 0.05;
const FLASH_QUAD_SIZE: f32 = 0.32;
const MUZZLE_EMBER_COUNT: usize = 3;
const MUZZLE_EMBER_SIZE: f32 = 0.009;
const MUZZLE_EMBER_SPEED: f32 = 12.0;
const MUZZLE_EMBER_CONE: f32 = 0.22;
const MUZZLE_EMBER_LIFETIME: f32 = 0.13;

const MUZZLE_BLADE_CHANCE: f32 = 0.5;
const MUZZLE_BLADE_WIDTH: f32 = 0.55;
const MUZZLE_BLADE_LENGTH: f32 = 1.9;
const MUZZLE_LIGHT_INTENSITY: f32 = 210_000.0;
const MUZZLE_LIGHT_RANGE: f32 = 3.2;

const TRACER_CHANCE: f32 = 0.8;
const TRACER_SPEED: f32 = 380.0;
const TRACER_STREAK_LENGTH: f32 = 22.0;
const TRACER_WIDTH: f32 = 0.035;
const TRACER_MIN_PIXELS: f32 = 3.0;
const TRACER_NEAR_SKIP: f32 = 3.0;
const TRACER_FADE_STEPS: usize = 5;
const TRACER_SEGMENTS: usize = 14;

const TRACER_START_Y_OFFSET: f32 = -0.05;

const SMOKE_LIFETIME: f32 = 1.9;
const SMOKE_START_SCALE: f32 = 0.52;
/// How opaque the native path's stand-in puff starts. The real material is
/// thinner than this and builds up by overlapping; one flat quad cannot.
const SMOKE_STAND_IN_ALPHA: f32 = 0.34;
const SMOKE_GROWTH_PER_SEC: f32 = 1.5;
const SMOKE_DRAG: f32 = 1.2;
const SMOKE_MAX_PUFFS: usize = 9;
const SMOKE_BUOYANCY: f32 = 0.22;

/// Nothing goes up while the trigger is down. FIRE_RATE is 0.1, so a shot
/// resets the clock long before this, and the trail only starts once the
/// shooting has actually stopped.
const BARREL_SMOKE_DELAY: f32 = 0.25;
const BARREL_SMOKE_WINDOW: f32 = 1.6;
const BARREL_TRAIL_MAX_POINTS: usize = 26;
const BARREL_TRAIL_INTERVAL: f32 = 0.045;
const BARREL_TRAIL_LIFETIME: f32 = 2.4;
const BARREL_TRAIL_RISE: f32 = 0.34;
const BARREL_TRAIL_SWAY: f32 = 0.12;
const BARREL_TRAIL_WIDTH: f32 = 0.018;
const BARREL_TRAIL_SPREAD: f32 = 0.13;
const BARREL_TRAIL_ALPHA: f32 = 0.55;

/// The trail is laid down in world space, so a fast turn sweeps the muzzle
/// through a wide arc and paints a ribbon across the view. Faded out by how
/// hard the eye is moving: rad/s for the turn, m/s for the walk, against a
/// sprint of 6.4.
/// Nothing below the start: ordinary aiming and walking leave it alone, and
/// only a hard swing or a sprint takes it out. Measured in play, a turn
/// averages 0.5 rad/s and peaks near 4.7, a walk averages 1.1 m/s against a
/// sprint of 6.4.
const BARREL_TRAIL_TURN_START: f32 = 0.9;
const BARREL_TRAIL_TURN_FADE: f32 = 2.6;
const BARREL_TRAIL_MOVE_START: f32 = 2.5;
const BARREL_TRAIL_MOVE_FADE: f32 = 5.5;
const BARREL_TRAIL_FADE_SPEED: f32 = 6.0;

const BLOOD_PARTICLE_COUNT: usize = 14;
const BLOOD_PARTICLE_SPEED: f32 = 3.2;
const BLOOD_PARTICLE_GRAVITY: f32 = -9.0;
const BLOOD_PARTICLE_LIFETIME: f32 = 0.6;
const BLOOD_PARTICLE_SIZE: f32 = 0.075;
const BLOOD_DROPLET_TEXTURE_SIZE: u32 = 32;
const MAX_BLOOD_PARTICLES: usize = 900;
const BLOOD_STAIN_SIZE: f32 = 0.22;
const BLOOD_FLOOR_STAIN_SIZE: f32 = 0.55;
const BLOOD_STAIN_VARIANTS: usize = 4;
const BLOOD_STAIN_LIFETIME: f32 = 12.0;

const BLOOD_BURST_SIZE: f32 = 0.34;
const BLOOD_BURST_LIFETIME: f32 = 0.22;
const BLOOD_BURST_START_SCALE: f32 = 0.35;
const BLOOD_BURST_END_SCALE: f32 = 1.7;
const BLOOD_BURST_NORMAL_OFFSET: f32 = 0.05;

const BLOOD_EXIT_PARTICLE_COUNT: usize = 11;
const BLOOD_EXIT_SPEED: f32 = 6.0;
const BLOOD_EXIT_CONE: f32 = 0.38;
const BLOOD_EXIT_WOUND_OFFSET: f32 = 0.3;
const BLOOD_WALL_RAYCAST_DISTANCE: f32 = 4.5;
const BLOOD_WALL_STAIN_SIZE: f32 = 0.42;
const BLOOD_WALL_STAIN_COUNT: usize = 3;
const BLOOD_WALL_STAIN_SPREAD: f32 = 0.22;

const BLOOD_FLOOR_RAYCAST_DISTANCE: f32 = 5.0;
const BLOOD_FLOOR_STAIN_MIN: usize = 1;
const BLOOD_FLOOR_STAIN_MAX: usize = 3;
const BLOOD_FLOOR_STAIN_SPREAD: f32 = 0.3;

const SPARK_PARTICLE_MIN: usize = 4;
const SPARK_PARTICLE_MAX: usize = 9;
const SPARK_PARTICLE_SPEED: f32 = 3.0;
const SPARK_PARTICLE_GRAVITY: f32 = -9.0;
const SPARK_PARTICLE_LIFETIME: f32 = 0.25;
const SPARK_PARTICLE_SIZE: f32 = 0.02;

// Recoil is per weapon now: see WeaponDef::recoil in weapon.rs.

const BULLET_SPREAD: f32 = 0.02;

const MOUSE_SENSITIVITY: f32 = 0.002;

/// The gyro's own scale, separate from the pointer's: the two are different
/// instruments and a player who likes a fast mouse need not want a fast phone.
const GYRO_SENSITIVITY_MIN: f32 = 0.25;
const GYRO_SENSITIVITY_MAX: f32 = 4.0;
const GYRO_SENSITIVITY_STEP: f32 = 0.25;

const SENSITIVITY_SCALE_MIN: f32 = 0.25;
const SENSITIVITY_SCALE_MAX: f32 = 3.0;
const SENSITIVITY_SCALE_STEP: f32 = 0.5;

const DAMAGE_MULTIPLIER_MIN: f32 = 0.25;
const DAMAGE_MULTIPLIER_MAX: f32 = 20.0;
const DAMAGE_MULTIPLIER_STEP: f32 = 0.5;

const AIM_MOUSE_SENSITIVITY_MULTIPLIER: f32 = 0.5;

const PITCH_CLAMP: f32 = 1.54;

const TOUCH_JOYSTICK_BASE_PX: f32 = 140.0;
const TOUCH_JOYSTICK_KNOB_PX: f32 = 64.0;

const TOUCH_JOYSTICK_RADIUS_PX: f32 = (TOUCH_JOYSTICK_BASE_PX - TOUCH_JOYSTICK_KNOB_PX) / 2.0;

/// Push the stick past this and the run starts, the way a phone does it.
///
/// Measured against the raw drag rather than the clamped one, so the extra
/// travel beyond full tilt is what asks for it. A thumb that merely reaches
/// full speed is not asking; one that keeps going is.
const TOUCH_SPRINT_RADIUS_PX: f32 = TOUCH_JOYSTICK_RADIUS_PX * 2.30;
/// And it has to be forward. Sprinting sideways or backwards is not a thing
/// the player can do anyway, so a stick shoved left should not light it up.
const TOUCH_SPRINT_FORWARD_DOT: f32 = 0.72;
/// Once running, it takes coming most of the way back to stop -- otherwise
/// the run flickers off every time the thumb drifts across the line.
const TOUCH_SPRINT_RELEASE_PX: f32 = TOUCH_JOYSTICK_RADIUS_PX * 1.70;
const TOUCH_FIRE_BUTTON_PX: f32 = 90.0;

/// How long a kill stays in the feed, and how much of that is spent fading.
/// How far in from the right edge the column sits. Wider than the 16px the
/// settings button uses: text is right-aligned inside the column and grows
/// leftward from this line, so the margin is the only thing standing between
/// the longest line and the edge of the screen.
const KILL_FEED_RIGHT: f32 = 64.0;
/// Wide enough for the longest line the feed can build, since the column has
/// to state a width to be anchored on the right at all.
const KILL_FEED_WIDTH: f32 = 300.0;
const KILL_FEED_LIFETIME: f32 = 4.5;
const KILL_FEED_FADE: f32 = 1.0;
/// Older entries are dropped rather than scrolled: a feed that fills the
/// right-hand side of the screen is worse than one that forgets.
const KILL_FEED_MAX: usize = 5;

const STATIC_UI_Z_INDEX: i32 = 100;

const TOUCH_LOOK_SENSITIVITY_MULTIPLIER: f32 = 1.0;

impl Default for TouchControlsEnabled {
    fn default() -> Self {

        Self(cfg!(target_os = "ios") || std::env::var("TOUCH_UI_PREVIEW").is_ok())
    }
}

const CROSSHAIR_CONTAINER_PX: f32 = 50.0;
const CROSSHAIR_CENTER_PX: f32 = CROSSHAIR_CONTAINER_PX / 2.0;
const CROSSHAIR_LINE_LENGTH_PX: f32 = 6.0;
const CROSSHAIR_LINE_THICKNESS_PX: f32 = 2.0;
const CROSSHAIR_BASE_GAP_PX: f32 = 4.0;
const CROSSHAIR_GAP_SPREAD_SCALE: f32 = 500.0;

const BASE_FOV: f32 = std::f32::consts::PI / 4.0;

pub(crate) const PHYSICS_SUBSTEPS: u32 = 4;
pub(crate) const UI_LAYOUT_HZ: u64 = 30;
pub(crate) const CLIP_SELECT_HZ: u64 = 60;

/// How long one zombie clip takes to give way to the next. A bite is a
/// decision and lands almost cut; everything else eases across.
pub(crate) const ZOMBIE_BLEND: std::time::Duration = std::time::Duration::from_millis(150);
pub(crate) const ZOMBIE_BLEND_BITE: std::time::Duration = std::time::Duration::from_millis(40);
/// Between two walk variants the gait is unchanged, so the blend is short and
/// the phase is carried rather than restarted.
/// Past this a body stops being animated, and inside the shorter one it
/// starts again. The gap is hysteresis: a single distance parks and wakes on
/// alternate frames for anything standing on the line. Generous on purpose --
/// a frozen body in view is a bug, and MAX_SHOT_DISTANCE is 200m.
pub(crate) const ANIMATION_PARK_DISTANCE: f32 = 60.0;
pub(crate) const ANIMATION_WAKE_DISTANCE: f32 = 55.0;

pub(crate) const ZOMBIE_BLEND_GAIT: std::time::Duration = std::time::Duration::from_millis(80);

/// Panels and readouts run at the rate their layout already runs at. Each
/// call makes its own timer, so the gated groups land on different frames
/// instead of all waking together.
///
/// Only for systems that read state and write state. Anything integrating
/// `delta_secs` would run at a third speed, since Time is still the frame's
/// delta and not the time since that system last ran -- and anything reading
/// `just_pressed` or a message would miss most of them, both being gone
/// within a frame or two of arriving.
/// Which clip a body should be playing, decided half as often as it is drawn.
/// Picking one 16ms late is a transition starting 16ms late, which nothing
/// can see; it is only the sampling of the clip that has to keep up, and that
/// is a different set of systems.
fn clip_tick() -> impl FnMut(Res<Time>) -> bool + Clone {
    bevy::time::common_conditions::on_timer(std::time::Duration::from_millis(1000 / CLIP_SELECT_HZ))
}

fn ui_tick() -> impl FnMut(Res<Time>) -> bool + Clone {
    bevy::time::common_conditions::on_timer(std::time::Duration::from_millis(1000 / UI_LAYOUT_HZ))
}

/// Damping ratio of the spring that raises the sights. Below 1 is
/// underdamped, which is what overshoots.
///
/// Overshoot per beat is exp(-2*pi*z / sqrt(1 - z*z)), so the dial is far from
/// linear: 0.2 gives 28% then 8%, two clear beats; 0.5 gives 2%, one soft
/// nudge; 0.7 gives 0.2%, which is not an overshoot at all. Most of the
/// character lives below 0.4.
///
/// Settled at 0.72 by hand, with the slide and tilt in weapon.rs carrying the
/// visible part rather than the spring.
const ADS_SPRING_DAMPING: f32 = 0.72;

/// Degrees the weapon points off the sight line per unit of settling error.
///
/// One number, not a pitch and a yaw: the direction comes from the weapon's
/// own travel, so a raise from the hip misses vertically and a swing from the
/// side misses horizontally without either being spelled out. Zero puts the
/// overshoot back along the path, which reads as a straight line with a
/// wobble.

const AIM_ZOOM_SPEED: f32 = 12.0;
const AIM_SPREAD_MULTIPLIER: f32 = 0.35;

const AIM_DOF_FOCAL_DISTANCE_A: f32 = 0.2;
const AIM_DOF_FOCAL_DISTANCE_B: f32 = 0.25;
const AIM_DOF_APERTURE_F_STOPS: f32 = 11.0;

const RESOLUTION_SCALE_PRESETS: [f32; 7] = [0.2, 0.3, 0.4, 0.5, 0.65, 0.8, 1.0];

const FPS_CAP_PRESETS: [u32; 4] = [120, 80, 60, 30];





const WORLD_LAYER: usize = 0;
const VIEWMODEL_LAYER: usize = 1;

/// The ring that blacks out everything outside the sight picture while the
/// main camera is doing the magnifying itself. Its own layer because the
/// viewmodel layer is exactly what gets switched off in that mode.
/// How much of the screen half height the sight picture fills in that mode.

/// The holographic dot's radius, in radians subtended -- the quad is hung one
/// unit down the barrel, so a radius in metres there is an angle. A real 1 MOA
/// dot is 0.00029rad, which is a pixel and a half and reads as a dead pixel;
/// this is nearer the 4 MOA most holographic sights actually carry.
const HOLO_DOT_RADIANS: f32 = 0.0013;

/// How far the sights have to be up before a scoped weapon hands the screen
/// over to its reticle and steps out of the way. Not 1.0: the swap wants to
/// happen while the view is still closing, or the rifle is still filling the
/// frame when the magnification arrives.
const SCOPED_RETICLE_SIGHTED: f32 = 0.6;

/// The duplex reticle, in radians subtended -- hung a unit down the barrel
/// like the holo dot, so the numbers are angles. Sized against a twelve times
/// view, which is what the M24's aim multiplier comes out at, so the thick
/// posts enter from the edge of the picture and the fine cross holds the
/// middle.
const SCOPE_RETICLE_REACH: f32 = 0.070;
/// Where each arm stops being a thick post and becomes a hairline, as a
/// fraction of the reach.
const SCOPE_RETICLE_TAPER: f32 = 0.42;
const SCOPE_RETICLE_POST_WIDTH: f32 = 0.0013;
const SCOPE_RETICLE_LINE_WIDTH: f32 = 0.00035;
/// The hole left in the middle, so the mark never covers what it is on.
const SCOPE_RETICLE_CENTRE_GAP: f32 = 0.035;
const SCOPE_RETICLE_DOT_SIZE: f32 = 0.0011;
const SCOPE_RETICLE_CENTRE_DOT_SIZE: f32 = 0.0013;
const SCOPE_RETICLE_DOTS_PER_ARM: usize = 4;

const STATIONARY_SPREAD_MULTIPLIER: f32 = 0.4;

const SPRINT_SPREAD_MULTIPLIER: f32 = 2.2;
const CROUCH_SPREAD_MULTIPLIER: f32 = 0.6;
const PRONE_SPREAD_MULTIPLIER: f32 = 0.2;

const LEAN_SPEED: f32 = 6.0;
const LEAN_ROLL_DEG: f32 = 15.0;
const LEAN_SIDE_OFFSET: f32 = 0.4;

const EYE_HEIGHT_OFFSET: f32 = 0.65;

const CROUCH_EYE_HEIGHT_OFFSET: f32 = EYE_HEIGHT_OFFSET * 0.55;
const PRONE_EYE_HEIGHT_OFFSET: f32 = EYE_HEIGHT_OFFSET * 0.15;

const HIT_MARKER_LIFETIME: f32 = 3.0;

const PLAYER_MAX_HEALTH: f32 = 100.0;

const ZOMBIE_ATTACK_DAMAGE: f32 = 12.0;

const FLOATING_DAMAGE_LIFETIME: f32 = 0.9;
const FLOATING_DAMAGE_RISE_SPEED: f32 = 0.7;
const FLOATING_DAMAGE_FONT_SIZE: f32 = 14.0;
const FLOATING_DAMAGE_CRIT_FONT_SIZE: f32 = 24.0;

const ZOMBIE_HEALTH_BAR_HEIGHT: f32 = 1.3;
const ZOMBIE_HEALTH_BAR_WIDTH_PX: f32 = 50.0;
const ZOMBIE_HEALTH_BAR_HEIGHT_PX: f32 = 6.0;

impl Default for SettingsMenuVisible {
    fn default() -> Self {
        Self(std::env::var("OPEN_SETTINGS_MENU").is_ok())
    }
}

impl Default for MapSelectionPending {
    fn default() -> Self {
        Self(game_flow::map_id_override().is_none())
    }
}

const SETTINGS_MENU_PAGE_COUNT: usize = 3;

impl Default for GunVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
struct MainSun;

const MAIN_SUN_ILLUMINANCE: f32 = 8_000.0;

#[derive(Resource, Default)]
struct LocalPlayerMoving(bool);

#[derive(Resource, Default)]
struct LocalPlayerSprinting(bool);

#[derive(Resource, Default)]
struct LocalPlayerMoveAxis(Vec2);

#[derive(Resource, Default)]
struct PerfTimings {
    health_bars_ms: f32,

}

/// Radians the view turned this frame, after sensitivity and magnification.
/// The weapon lags this, not the raw mouse; see LOOK_SWAY_SENSITIVITY.
/// Whether the map casts into the runtime shadow map.
///
/// Off by default now that the map's shadows are baked: the lightmap's green
/// channel already says whether the sun reaches each texel and the probe grid
/// says it for anything standing in the air. Re-casting the whole map every
/// frame repeats work that is already in a texture.
#[derive(Resource)]
pub(crate) struct StaticShadows(pub(crate) bool);
impl Default for StaticShadows {
    fn default() -> Self {
        Self(false)
    }
}

/// Whether zombies, the player and the weapon cast. This one cannot be baked
/// -- a shadow that moves is not a property of the map -- so it stays on.
#[derive(Resource)]
pub(crate) struct DynamicShadows(pub(crate) bool);
impl Default for DynamicShadows {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Default)]
struct MouseLookDelta(Vec2);

#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.add_plugins((

            // Bevy's own audio is disabled: seedling owns the graph, and two
            // mixers fighting over the device is not a thing that works.
            DefaultPlugins
                .build()
                .disable::<bevy::audio::AudioPlugin>()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Last Day in Saigon".to_string(),

                        mode: WindowMode::Windowed,
                        // VSYNC=0 unclamps the frame rate. Measuring a cost
                        // through vsync cannot see anything cheaper than a
                        // refresh: every setting reads as 8.33ms on a 120Hz
                        // panel whether it costs 2ms or 8.
                        present_mode: if std::env::var("VSYNC").map(|v| v == "0").unwrap_or(false) {
                            bevy::window::PresentMode::AutoNoVsync
                        } else {
                            bevy::window::PresentMode::default()
                        },
                        ..default()
                    }),
                    ..default()
                })

                .set(TaskPoolPlugin {
                    task_pool_options: if cfg!(target_os = "ios") {
                        TaskPoolOptions::with_num_threads(4)
                    } else {
                        TaskPoolOptions::default()
                    },
                }),
            MeshPickingPlugin,
            bevy::ui_render::prelude::UiMaterialPlugin::<DamageOverlayMaterial>::default(),
            MaterialPlugin::<MuzzleSmokeMaterial>::default(),
            MaterialPlugin::<MuzzleFlashMaterial>::default(),
            bevy::core_pipeline::fullscreen_material::FullscreenMaterialPlugin::<ScreenEffects>::default(),
            PhysicsPlugins::default(),
            SimPlugin,
            zombie_nav::ZombieNavmeshPlugin,
            // Seedling owns the audio graph; Steam Audio sits inside it and
            // takes the room from Avian's colliders, so the acoustics follow
            // the same geometry the bullets do rather than a second model of
            // the map kept in step by hand.
            bevy_seedling::SeedlingPlugins,
        ))
        // Not on iOS: see the note on the dependency in Cargo.toml.
        .add_plugins((
            #[cfg(not(target_os = "ios"))]
            bevy_steam_audio::SteamAudioPlugin::default(),
            #[cfg(not(target_os = "ios"))]
            avian_steam_audio::AvianSteamAudioScenePlugin,
        ))
        // Four, not Avian's default six: the solver was running at 384Hz for
        // capsule characters and ragdolls, and the two builds have to agree or
        // they are not the same game. Raise it again if joints go rubbery or
        // a crowd starts interpenetrating.
        .insert_resource(avian3d::prelude::SubstepCount(PHYSICS_SUBSTEPS))
        // The HUD is near static -- ammo, health, a compass strip -- and
        // recomputing its layout at the refresh rate cost 0.09ms a frame to
        // produce the same numbers. A newly spawned node now waits up to 33ms
        // to be placed, which is the price.
        .configure_sets(
            PostUpdate,
            bevy::ui::UiSystems::Layout.run_if(bevy::time::common_conditions::on_timer(
                std::time::Duration::from_millis(1000 / UI_LAYOUT_HZ),
            )),
        )

        .insert_resource(if cfg!(target_os = "ios") {
            WinitSettings {
                focused_mode: UpdateMode::reactive(std::time::Duration::from_secs_f32(1.0 / 120.0)),
                ..WinitSettings::game()
            }
        } else {

            let cap = std::env::var("FPS_CAP")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|c| *c > 0.0)
                .unwrap_or(60.0);
            WinitSettings {
                focused_mode: UpdateMode::reactive(std::time::Duration::from_secs_f32(1.0 / cap)),
                unfocused_mode: UpdateMode::reactive_low_power(std::time::Duration::from_secs_f32(1.0 / 10.0)),
                ..WinitSettings::game()
            }
        });

    app.add_plugins(GamePlugin);
    app.run();
}

pub(crate) fn debug_keys_allowed() -> bool {
    true
}

/// Public so the perf rigs can build it beside a renderer it never reaches:
/// desert_rust_metal links this crate to price the game's own systems without
/// drawing a thing from Rust.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
    app.init_resource::<PlayerModelPoseOverride>()
        .register_type::<PlayerModelPose>()
        .register_type::<ZombiePose>()
        .register_type::<PlayerModelPoseOverride>()
        .init_resource::<ZombieCountUi>()
        .init_resource::<AutoMaintainUi>()

        .init_state::<AppState>()
        .init_resource::<AfterLoading>()
        .init_resource::<AimToggleMode>()
        .init_resource::<AimHeld>()
        .add_observer(on_zombie_replicated)
        .add_observer(on_zombie_kind_replicated)
        .add_observer(on_shot_fx)
        .add_observer(note_shot_for_barrel_smoke)
        .add_observer(on_explosion)
        .add_observer(on_shot_confirmed)
        .add_observer(drop_loot_on_kill)
        .add_observer(on_corpse_hit)
        .add_observer(on_game_state_event)
        .add_plugins(F9DebugPlugin)
        .add_plugins(navigation::ClientNavigationPlugin)
        // RENDER_DIAG=1 times every render graph node. On Metal only the CPU
        // side is recorded, which is the side a second camera is expensive on:
        // the graph is walked per view whether that view draws anything or not.
        .add_plugins(render_diagnostics_if_asked())
        .add_plugins(NoShadowPassPlugin)
        .add_plugins(DrawCountPlugin)

        .insert_resource(Noclip(false))
        .init_resource::<HitFlash>()
        .init_resource::<Autopilot>()
        .init_resource::<AutopilotLookOnly>()
        .init_resource::<AutopilotState>()
        .init_resource::<AutopilotIntent>()
        .init_resource::<FreezeZombies>()
        .init_resource::<ShowHitboxes>()
        .init_resource::<RenderResolutionScale>()
        .init_resource::<FpsCap>()
        .init_resource::<GodMode>()
        .init_resource::<ShowAutopilotTarget>()
        .init_resource::<ShowHitMarkers>()
        .init_resource::<ShowFloatingDamage>()
        .init_resource::<ShowBulletTracers>()
        .init_resource::<BarrelSmoke>()
        .init_resource::<LimbDamage>()
        .init_resource::<gun_view::WalkCycle>()
        .init_resource::<movement_combat::FireShock>()
        .init_resource::<ActiveWeapon>()
        .init_resource::<weapon::WeaponSwitchRequest>()
        .add_systems(
            PostUpdate,
            sever_limbs
                .after(bevy::app::AnimationSystems)
                .before(TransformSystems::Propagate),
        )
        .init_resource::<SettingsMenuVisible>()
        .init_resource::<SettingsMenuScrollState>()
        .init_resource::<SettingsMenuPageIndex>()
        .init_resource::<MouseUiLocked>()
        .init_resource::<TouchControlsIdleHidden>()
        .init_resource::<AimBlurEnabled>()
        .init_resource::<AimDofFocalDistanceAlt>()
        .init_resource::<MouseLookDelta>()
        .init_resource::<WeaponPose>()
        .init_resource::<PerfTimings>()
        .init_resource::<ZombieAnimDisabled>()
        .init_resource::<ZombieHealthBarsHidden>()
        .init_resource::<AutoLean>()
        .init_resource::<PlayerModelTuning>()
        .init_resource::<GunVisible>()
        .init_resource::<MapMsaaOn>()
        .init_resource::<ShadowQuality>()
        .init_resource::<StaticShadows>()
        .init_resource::<DynamicShadows>()
        .init_resource::<LightQuality>()
        .init_resource::<PointLightQuality>()
        .init_resource::<ShadowCascades>()
        .init_resource::<BloomEnabled>()
        .init_resource::<ScreenEffectsEnabled>()
        .init_resource::<FogEnabled>()
        .init_resource::<BulletHolesEnabled>()
        .init_resource::<BloodEnabled>()
        .init_resource::<PlayerModelVisible>()
        .init_resource::<MuzzleFlashEnabled>()
        .init_resource::<ImpactSparksEnabled>()
        .init_resource::<ShellCasingsEnabled>()
        .init_resource::<MuzzleSmokeEnabled>()
        .init_resource::<PortSmokeEnabled>()
        .init_resource::<BarrelSmokeEnabled>()
        .init_resource::<LocalPlayerMoving>()
.init_resource::<LocalPlayerSprinting>()
.init_resource::<LocalPlayerMoveAxis>()
        .init_resource::<TouchControlsEnabled>()
        .init_resource::<TouchMoveState>()
        .init_resource::<TouchMoveAxis>()
        .init_resource::<TouchLookState>()
        .init_resource::<TouchFiring>()
        .init_resource::<TouchAiming>()
        .init_resource::<TouchJump>()
        .init_resource::<NoclipFlyInput>()
        .init_resource::<TouchReload>()
        .init_resource::<TouchSprintHeld>()
        .init_resource::<TouchLeanHeld>()
        .init_resource::<SensitivityScale>()
        .init_resource::<GyroEnabled>()
        .init_resource::<GyroInvert>()
        .init_resource::<GyroSensitivity>()
        .init_resource::<DamageMultiplierUi>()
        .init_resource::<KillCount>()
        .init_resource::<MapSelectionPending>()
        .init_resource::<RecentlyHitZombies>()
        .init_resource::<KillStreak>()
        .init_resource::<Loadout>()
        .init_resource::<Inventory>()
        .init_resource::<Boost>()
        .init_resource::<CorpseStages>()
        .init_resource::<WeaponSpawnEnabled>()
        .init_resource::<ZombieWalk>()
        .init_resource::<InventoryScreenOpen>()
        .init_resource::<Equipment>()
        .init_resource::<Attachments>()
        .add_systems(Startup, (apply_persisted_settings, setup_common, force_game_over_on_start).chain())
        .add_systems(Startup, (spawn_pole_targets, init_pickup_assets))
        .add_systems(OnEnter(AppState::Menu), enter_menu)
        .add_systems(OnExit(AppState::Menu), exit_menu)
        // Gameplay is built while the loading screen is still up, so the map
        // and the weapon are queued and waited on rather than popping in once
        // play has already started.
        .add_systems(
            OnEnter(AppState::Loading),
            (
                spawn_loading_screen,
                setup_gameplay.run_if(|after: Res<AfterLoading>| after.0 == AppState::InGame),
            ),
        )
        .add_systems(OnExit(AppState::Loading), despawn_loading_screen)
        .add_systems(Update, finish_loading.run_if(in_state(AppState::Loading)));
    app.add_systems(Update, persist_settings_on_change);
    app.add_systems(Update, build_zombie_anims_when_loaded);

    app.add_systems(
        Update,
        (
            (
                multi_touch_button_interaction,
                suppress_touch_buttons_while_menu_open,
                toggle_mouse_ui_lock,
                suppress_ui_interaction_when_locked,
                touch_move_axis,
                update_joystick_knob_visual,
                touch_look,
                touch_fire_button,
                touch_weapon_button,
                touch_aim_button,
                touch_jump_button,
                update_noclip_fly_input,
                touch_crouch_button,
                touch_reload_button,
                touch_prone_button,
                touch_lean_buttons,
                update_touch_button_press_visual,
            )
                .chain(),
            // Reaching the menu is input, and input is not gateable: the key
            // is cleared the frame after it is pressed and the scroll message
            // is gone in two.
            (
                toggle_settings_menu,
                scroll_settings_menu,
                touch_scroll_settings_menu,
                update_touch_controls_idle_hidden,
            )
                .chain(),
            (
                (
                    update_settings_menu,
                    open_settings_menu_from_env,
                    settings_menu_page_buttons,
                    update_settings_menu_pages,
                    update_mouse_ui_lock_indicator,
                    resolution_scale_buttons,
                update_resolution_scale_button_colors,
                (fps_cap_buttons, update_fps_cap_button_colors, apply_fps_cap_desktop).chain(),
                    resize_scene_render_target,
                    cheat_toggle_buttons,
                    apply_map_render_toggles,
                )
                    .chain(),
                (
                    cheat_toggle_buttons_2,
                    update_cheat_toggle_colors,
                    cheat_kill_all_button,
                    cheat_zombie_health_buttons,
                    zombie_count_buttons,
                    update_zombie_count_text,
                    sensitivity_buttons,
                    update_sensitivity_text,
                    gyro_buttons,
                    update_gyro_ui,
                    loadout_hud_buttons,
                    // Grouped only to stay inside the twenty this tuple
                    // takes; nothing here is ordered against anything.
                    (
                        give_weapon_buttons,
                        corpse_stage_buttons,
                        update_corpse_stage_buttons,
                        weapon_spawn_buttons,
                        update_weapon_spawn_buttons,
                        zombie_walk_buttons,
                        update_zombie_walk_buttons,
                        toggle_inventory_screen,
                        update_inventory_screen,
                        rebuild_inventory_screen,
                        inventory_screen_rows,
                    ),
                    update_bars,
                    update_weapon_slots,
                    update_item_ui,
                    damage_multiplier_buttons,
                    update_damage_multiplier_text,
                    auto_maintain_button,
                    update_auto_maintain_button_color,
                    hide_ui_when_locked,
                )
                    .chain(),
                (
                    cheat_toggle_buttons_3,
                    (render_toggle_buttons, update_render_toggle_colors).chain(),
                    (fire_fx_toggle_buttons, update_fire_fx_toggle_colors).chain(),
                    autopilot_toggle_button,
                    update_extra_toggle_colors,
                    cheat_spawn_zombie_button,
                    shadow_quality_buttons,
                    update_shadow_quality_button_colors,
                    light_quality_buttons,
                    update_light_quality_button_colors,
                    apply_point_light_quality,
                    point_light_quality_buttons,
                    update_point_light_quality_button_colors,
                )
                    .chain(),
            )
                .chain()
                .run_if(ui_tick()),
            (
                (
                    grab_cursor,
                    update_aim_held,
                    toggle_noclip.run_if(debug_keys_allowed),
                    toggle_player_stance,
                    toggle_debug_flags.run_if(debug_keys_allowed),
                    autopilot_pick_target,
                    // No longer ordered against LandmassSystems::Output --
                    // landmass runs in FixedUpdate now, and this does not.
                    // The desired velocity read here can be one frame old,
                    // which is nothing against a 20Hz decision rate.
                    autopilot_think,
                    mouse_look,
                    // Before shoot, not after. Everything that leaves the
                    // barrel is placed from the weapon's transform, and a
                    // weapon posed afterwards leaves that transform a frame
                    // old -- 2cm of it while the weapon is swinging to ready,
                    // which shows as the flash hanging off the muzzle.
                    // Before everything that rides it: the weapon and the
                    // view both read this cycle in the same frame it is
                    // advanced, so they cannot drift apart.
                    // Grouped only to stay inside the twenty this tuple
                    // takes; the chain still orders them.
                    (advance_walk_cycle, advance_fire_shock).chain(),
                    gun_view_animation,
                    shoot,
                    reload_gun,
                    suggest_auto_lean,
                    apply_lean_input,
                    apply_view_recoil,
                    apply_aim_zoom,
                    apply_aim_dof,
                    send_player_input.run_if(map::world_ready),
                    sync_camera_to_player,
                    zombies_damage_player,
                )
                    .chain(),

                (count_local_kills, update_kill_count_ui, update_kill_feed).chain(),
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
            (
                check_game_over,
                restart_game.run_if(debug_keys_allowed),
                restart_button,
                map_select_button,
                reset_config_button,
                wire_zombie_scene_animation_targets,
                wire_zombie_animations.run_if(resource_exists::<ZombieKindAnims>),
                wire_zombie_hitbox_bones,
                wire_gun_mesh,
                wire_weapon_sockets,
                log_frame_pacing,
                track_muzzle_socket,
                update_zombie_animation.run_if(resource_exists::<ZombieAnim>).run_if(clip_tick()),
                freeze_zombie_animations.run_if(clip_tick()),
                park_distant_animations.run_if(clip_tick()),

                (
                    wire_player_model_scene_nodes,
                    (spawn_viewmodel_body, wire_viewmodel_body_meshes, apply_player_model_visible),
                    spawn_grip_sockets,
                    wire_player_model_animations.run_if(resource_exists::<PlayerModelAnim>),
                    update_local_player_model_animation
                        .run_if(resource_exists::<PlayerModelAnim>)
                        .run_if(clip_tick()),
                    apply_player_model_pose_override
                        .run_if(resource_exists::<PlayerModelAnim>)
                        .run_if(clip_tick()),
                    update_compass_ui.run_if(ui_tick()),
                    update_radar_ui,
                    update_zombie_health_bars.run_if(ui_tick()),
                    update_floating_damage_texts,
                    play_footsteps,
                    play_zombie_footsteps,
                    update_damage_overlay,
                    update_screen_effects,
                    demote_fully_metallic_imports,
                    exclude_oversized_shadow_casters.run_if(ui_tick()),
                    (
                    test_blast_key.run_if(debug_keys_allowed),
                    select_weapon_slot,
                    switch_weapon,
                    use_consumable_key,
                    advance_use_cooldown,
                    advance_boost,
                    advance_pickups,
                    collect_pickups,
                    forget_gone_zombies.run_if(ui_tick()),
                    update_gore,
                    update_death,
                    spawn_what_was_armed_last_frame,
                    apply_pending_blast,
                    arm_ragdoll,
                    settle_corpse,
                )
                    .chain(),
                update_dissolve,
                )
                    .chain(),
            )
                .chain(),
            (
                place_holo_reticle,
                (apply_bloom, apply_screen_effects_toggle, apply_fog).chain(),
                update_ammo_ui.run_if(ui_tick()),
                update_crosshair,
                set_hitbox_gizmo_depth_bias.run_if(ui_tick()),
                draw_zombie_hitboxes.run_if(ui_tick()),
                update_splash_screen,
                despawn_hit_markers,
                despawn_muzzle_flashes,
                update_bullet_tracers,
                update_smoke,
                update_blood_particles,
                (
                    (update_blood_bursts, update_bullet_holes, merge_coincident_holes, update_impact_dust, update_casings, update_port_smoke),
                    (drop_magazine_on_reload, restore_magazine_after_reload, update_dropped_magazines),
                ),
                (
                    update_muzzle_smoke,
                    update_muzzle_flash,
                    update_barrel_smoke,
                    // Nested rather than added beside the others: this tuple's
                    // parent was already at Bevy's twenty-element limit.
                    //
                    // Only on the native path -- the windowed build renders
                    // the real materials and a second would draw over them.
                    (attach_muzzle_fx_stand_ins, fade_muzzle_fx_stand_ins)
                        .chain()
                        .run_if(native_renderer_in_use),
                ),
                update_spark_particles,
                despawn_blood_stains,
            )
                .chain(),
        )
            .chain(),
    );

    app.add_systems(
        PostUpdate,
        (
            pose_viewmodel_arms,
            shape_viewmodel_skeleton,
            drive_skeleton_from_ragdoll,
            aim_zombie_heads,
            apply_bone_flinch,
            lean_zombie_into_turn,
            attach_foot_pins,
            pin_zombie_feet,
            hold_finger_grip,
        )
            .after(AnimationSystems)
            .before(TransformSystems::Propagate)
            .run_if(in_state(AppState::InGame)),
    );

    app.add_systems(
        PostUpdate,
        (
            drive_reload_hand,
            update_pole_targets,
            solve_grip_ik,
        )
            .chain()
            .after(TransformSystems::Propagate)
            .run_if(grip_ik_enabled)
            .run_if(in_state(AppState::InGame)),
    );
    }
}

const RESIZE_DEBOUNCE_FRAMES: u8 = 5;

impl Default for ZombieCountUi {
    fn default() -> Self {
        Self(10)
    }
}

impl Default for AutoMaintainUi {
    fn default() -> Self {
        Self(true)
    }
}

const TOUCH_CONTROLS_IDLE_HIDE_SECS: f32 = 10.0;

const COMPASS_WIDTH: f32 = 280.0;
const COMPASS_HEIGHT: f32 = 26.0;

const COMPASS_PX_PER_DEG: f32 = 2.2;

const COMPASS_MARKS: [(f32, &str); 8] =
    [(0.0, "N"), (45.0, "NE"), (90.0, "E"), (135.0, "SE"), (180.0, "S"), (225.0, "SW"), (270.0, "W"), (315.0, "NW")];

const RADAR_LEFT: f32 = 16.0;
const RADAR_TOP: f32 = 16.0;
const RADAR_SIZE: f32 = 150.0;
const RADAR_RADIUS: f32 = RADAR_SIZE / 2.0;
const RADAR_RANGE: f32 = 40.0;
const RADAR_BLIP_SIZE_PX: f32 = 8.0;
const RADAR_SWEEP_DEG_PER_SEC: f32 = 60.0;
const RADAR_TRAIL_COUNT: u8 = 3;
const RADAR_TRAIL_STEP_DEG: f32 = 10.0;

#[unsafe(no_mangle)]
#[inline(never)]
#[cfg(target_os = "ios")]
pub extern "C" fn start_app() {
    main();
}

/// Everything `GamePlugin` leans on besides a renderer, in the order
/// `native_game_app` builds it.
///
/// Public so the perf rigs can stand `GamePlugin` up without copying this list
/// and then drifting from it. The iOS path has no `MaterialPlugin` -- it draws
/// through `metal_renderer` -- so these collections are registered by hand or
/// the systems that ask for them abort, which across FFI takes the process
/// with it.
pub fn register_game_assets(app: &mut App) {
    use bevy::asset::AssetApp;
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    metal_renderer::gltf_materials::register_gltf_standard_materials(app);
    app.init_asset::<bevy::pbr::StandardMaterial>();
    app.init_asset::<bevy::audio::AudioSource>();
    app.init_asset::<DamageOverlayMaterial>();
    app.init_asset::<MuzzleSmokeMaterial>();
    app.init_asset::<MuzzleFlashMaterial>();
}
