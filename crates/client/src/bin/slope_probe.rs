//! Walks a player up a ramp of a known angle and prints what happens.
//!
//! The map's slopes come in whatever angles the artist built, reached from
//! whatever direction the player wandered in from, which is why "sometimes
//! slow, sometimes launched" was all anyone could say about them. This builds
//! one ramp at one angle, walks straight up it at a fixed timestep, and prints
//! a number per frame.
//!
//!   SLOPE_DEG=25 cargo run -p client --release --bin slope_probe

use avian3d::prelude::*;
use bevy::prelude::*;
use shared::movement::simulate_player_step;
use shared::{ClientInput, Player, PlayerStance};

#[derive(Resource)]
struct Probe {
    frame: u32,
    angle_deg: f32,
}

fn main() {
    let angle_deg: f32 = std::env::var("SLOPE_DEG")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25.0);

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        // Avian's collider cache reads AssetEvent<Mesh>, which does not exist
        // without the asset and mesh plugins, and a missing message is a hard
        // panic rather than an empty read.
        .add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::mesh::MeshPlugin)
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Probe { frame: 0, angle_deg })
        .add_systems(Startup, setup)
        .add_systems(Update, step)
        .run();
}

fn setup(mut commands: Commands, probe: Res<Probe>) {
    // Flat ground to start on, so the walk is already at full speed by the
    // time it reaches the slope -- hitting a ramp from a standing start would
    // confuse acceleration with whatever the ramp does.
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(40.0, 1.0, 20.0),
        Transform::from_xyz(0.0, -0.5, 10.0),
    ));
    // The ramp, hinged at the far edge of the flat ground and rising away.
    // STAIRS=1 builds a flight instead, because the two have to be checked
    // together: every attempt at the ramp so far has cost the stairs.
    if std::env::var("STAIRS").is_ok() {
        for i in 0..20 {
            let rise = 0.18 * (i + 1) as f32;
            commands.spawn((
                RigidBody::Static,
                Collider::cuboid(40.0, rise.max(0.02), 0.30),
                Transform::from_xyz(0.0, rise * 0.5, -0.15 - 0.30 * i as f32),
            ));
        }
        return;
    }

    // Hinged on the origin so the ramp's top face leaves y=0 exactly where
    // the flat ground ends, and climbs from there. Worked out rather than
    // nudged: the centre sits half a length along the slope and half a
    // thickness below its own surface.
    let theta = probe.angle_deg.to_radians();
    let (half_len, half_thick) = (20.0_f32, 0.5_f32);
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(40.0, 1.0, 40.0),
        Transform::from_xyz(
            0.0,
            half_len * theta.sin() - half_thick * theta.cos(),
            -half_len * theta.cos() - half_thick * theta.sin(),
        )
        .with_rotation(Quat::from_rotation_x(theta)),
    ));
    commands.spawn((
        Player { vertical_velocity: 0.0, grounded: false, horizontal_velocity: Vec3::ZERO, ground_normal: Vec3::Y },
        PlayerStance::Standing,
        Transform::from_xyz(0.0, 1.0, 2.0),
    ));
}

fn step(
    mover: MoveAndSlide,
    mut probe: ResMut<Probe>,
    mut query: Query<(&mut Transform, &mut Player, &PlayerStance)>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok((mut transform, mut player, stance)) = query.single_mut() else { return };
    // Fixed, so two runs are comparable. The real game's dt wanders and that
    // is one more thing that would make a slope behave differently twice.
    let dt = 1.0 / 120.0;
    let input = ClientInput {
        move_axis: Vec2::new(0.0, 1.0),
        yaw: 0.0,
        ..Default::default()
    };

    let before = transform.translation;
    simulate_player_step(&mover, &mut transform, &mut player, *stance, &input, dt);
    let moved = transform.translation - before;
    let speed = Vec2::new(moved.x, moved.z).length() / dt;

    // Only while it is on the slope, and only every few frames: the interest
    // is in how the number behaves, not in six hundred of them.
    if probe.frame % 4 == 0 {
        println!(
            "f{:>4}  z {:>7.3}  y {:>6.3}  speed {:>6.3}  climb {:>6.3}  grounded {}  normal.y {:.3}",
            probe.frame,
            transform.translation.z,
            transform.translation.y,
            speed,
            moved.y / dt,
            u8::from(player.grounded),
            player.ground_normal.y,
        );
    }

    probe.frame += 1;
    if probe.frame > 1400 {
        exit.write(AppExit::Success);
    }
}
