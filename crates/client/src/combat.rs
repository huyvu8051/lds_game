use avian3d::prelude::*;
use bevy::prelude::*;

use shared::player::GameLayer;

use crate::dismember::LimbDamage;
use crate::dissolve::begin_death;
use crate::CorpseStages;
use crate::ragdoll::{DeathClipPlaying, DEATH_PUSH_SPEED};
use crate::zombie_reaction::CorpseHit;
use crate::map::*;
use crate::sim::*;
use shared::{
    resolve_bone_world_positions, zombie_hit_damage_ray, zombie_hit_nearest_region, BoneWorldPositions, FireShot,
    GameOver, GameStateEvent, Health, KnockbackVelocity,
    Player, PlayerDied, RequestRestart, SetDamageMultiplier,
    ShotConfirmed, ShotFx, Zombie, ZombieAnim, ZombieAnimationPlayer, ZombieHitboxBones, ZombieKind,
    KNOCKBACK_FORCE,
    MAX_SHOT_DISTANCE, Scenario,
    ZOMBIE_REST_HEIGHT, PLAYER_THREAT_PER_SHOT, ZOMBIE_OUTER_HEIGHT, ZOMBIE_OUTER_RADIUS, ZOMBIE_RESPAWN_DELAY,
};

#[derive(Resource)]
pub(crate) struct DamageMultiplierServer(f32);
impl Default for DamageMultiplierServer {
    fn default() -> Self {
        Self(1.0)
    }
}

pub(crate) fn on_set_damage_multiplier(trigger: On<SetDamageMultiplier>, mut multiplier: ResMut<DamageMultiplierServer>) {
    multiplier.0 = trigger.event().0.max(0.0);
}

pub(crate) fn handle_fire_shot(
    trigger: On<FireShot>,
    mut commands: Commands,
    spatial_query: SpatialQuery,
    bone_globals: Query<&GlobalTransform>,
    mut zombies: Query<
        (
            Entity,
            &Transform,
            &ZombieHitboxBones,
            &mut Health,
            &mut KnockbackVelocity,
            Option<&ZombieKind>,
            Option<&ZombieAnimationPlayer>,
        ),
        With<Zombie>,
    >,
    anim: Option<Res<ZombieAnim>>,
    mut anim_players: Query<&mut AnimationPlayer>,
    mut respawns: ResMut<PendingRespawns>,
    mut threats: Query<&mut PlayerThreat>,
    damage_multiplier: Res<DamageMultiplierServer>,
    corpse_stages: Res<CorpseStages>,
    mut limb_damage: ResMut<LimbDamage>,
    dying: Query<
        (Entity, &ZombieHitboxBones),
        (
            Or<(With<DeathClipPlaying>, With<crate::ragdoll::Ragdolling>, With<crate::ragdoll::CorpseSettled>)>,
            Without<Zombie>,
        ),
    >,
    layers: Query<&CollisionLayers>,
) {
    for mut threat in &mut threats {
        threat.0 += PLAYER_THREAT_PER_SHOT;
    }

    let shot = *trigger.event();
    let Ok(direction) = Dir3::new(shot.direction) else {
        return;
    };

    let filter = SpatialQueryFilter::default();
    // A corpse is cover. It is on its own layer because a settled body must
    // not shove whoever dropped it, and that same layer is what marks it as
    // something a round ends in rather than passes through -- severed limbs
    // with it, since they carry the layer too.
    let stops = |entity: Entity| {
        layers.get(entity).is_ok_and(|l| l.memberships.has_all(GameLayer::Corpse))
    };
    let path = trace_shot_path(&spatial_query, shot.origin, direction, &filter, &stops);
    let wall_distance = path.blocking_distance();
    for wall in &path.through {
        commands.trigger(ShotFx::Penetrated { point: wall.entry, normal: wall.normal });
        commands.trigger(ShotFx::Penetrated { point: wall.exit, normal: -wall.normal });
    }

    let cylinder = Collider::cylinder(ZOMBIE_OUTER_RADIUS, ZOMBIE_OUTER_HEIGHT);

    // A dead body is no longer a Zombie -- begin_death takes the component
    // off -- so the query above cannot see it and a round went straight
    // through into the wall behind. All three states of a corpse are taken:
    // falling, ragdolling and settled. It takes no damage, and it stops the
    // shot: a body is cover, so what stands behind one is safe behind it.
    //
    // This used to read the other way round -- a round through a corpse was
    // not allowed to save anything behind it -- and the trace agreed, in the
    // sense that it charged a corpse one of the two penetrations and 45% of
    // the damage and let the rest carry on. Cover is the call now.
    //
    // Tested against the bones rather than the body cylinder: a ragdoll's
    // root transform stops following the limbs the moment physics takes them.
    for (body, bones) in &dying {
        let resolved = resolve_bone_world_positions(bones, |bone| bone_globals.get(bone).ok().map(|g| g.translation()));
        let Some(hit) = zombie_hit_damage_ray(shot.origin, direction, MAX_SHOT_DISTANCE, &resolved) else {
            continue;
        };
        if wall_distance.is_some_and(|w| w < shot.origin.distance(hit.point) - 0.1) {
            continue;
        }
        commands.trigger(CorpseHit {
            body,
            point: hit.point,
            normal: hit.normal,
            damage: hit.damage,
            direction: *direction,
        });
    }

    let mut candidates: Vec<(f32, Entity, BoneWorldPositions)> = zombies
        .iter()
        .filter_map(|(entity, transform, bones, _, _, _, _)| {
            let (distance, _outer_normal) =
                cylinder.cast_ray(transform.translation, transform.rotation, shot.origin, *direction, MAX_SHOT_DISTANCE, true)?;
            let resolved = resolve_bone_world_positions(bones, |bone| bone_globals.get(bone).ok().map(|g| g.translation()));
            Some((distance, entity, resolved))
        })
        .collect();
    candidates.sort_by(|(a, ..), (b, ..)| a.total_cmp(b));

    for (distance, entity, bones) in &candidates {
        let entity = *entity;

        let Some(hit) = zombie_hit_damage_ray(shot.origin, direction, MAX_SHOT_DISTANCE, bones) else {
            let outer_hit_point = shot.origin + *direction * distance;
            if let Some((region, nearest_distance, radius)) = zombie_hit_nearest_region(outer_hit_point, bones) {
                debug!(
                    "[shot] outer capsule hit but missed every inner bone capsule: zombie={entity} — nearest was {region}, \
                     distance={nearest_distance:.3} radius={radius:.3} (missed by {:.3}) — trying the next zombie behind it, if any",
                    nearest_distance - radius
                );
            } else {
            }
            continue;
        };
        let (hit_point, normal, damage) = (hit.point, hit.normal, hit.damage);
        if let Some((limb, segment)) = hit.limb {
            limb_damage.record(entity, limb, segment, damage);
        }
        let hit_distance = shot.origin.distance(hit_point);
        // The head is the one hitbox that does not read the multipliers.
        //
        // HEAD_DAMAGE was written as ZOMBIE_MAX_HEALTH, which made a headshot
        // lethal for exactly as long as every zombie had the same health.
        // Once a wave-thirty zombie has four times as much, the same shot is
        // a quarter of its health and the weak weapons stop killing with it
        // at all -- the gun's own damage_scale, the range band and a wall all
        // chip at it. A headshot that is only lethal on a good weapon at
        // close range is not a headshot, so this is a rule rather than an
        // arithmetic result: hit the head and the thing dies.
        let headshot = hit.limb.is_none_or(|(part, _)| part == shared::ZombiePart::Head)
            && damage >= shared::HEAD_DAMAGE;
        let damage = if headshot {
            f32::INFINITY
        } else {
            damage * damage_multiplier.0 * shot.damage_scale
                * path.damage_scale_at(shot.origin, hit_distance)
                * shot.range.at(hit_distance)
        };

        if wall_distance.is_some_and(|w| w < hit_distance - 0.1) {
            info!(
                "[shot] blocked by level geometry before zombie={entity} (wall at {:.2} < zombie at {:.2})",
                wall_distance.unwrap(),
                hit_distance
            );
            continue;
        }
        if let Ok((_, _, _, mut health, mut knockback, kind, link)) = zombies.get_mut(entity) {
            health.0 -= damage;
            let killed = health.0 <= 0.0;
            if killed {
                begin_death(
                    &mut commands,
                    entity,
                    kind.copied().unwrap_or_default(),
                    anim.as_deref(),
                    link,
                    &mut anim_players,
                    // Flat, so a body is pushed over rather than into the
                    // floor, and scaled by what hit it -- clamped because a
                    // headshot's damage is infinity and would come out as a
                    // NaN the solver cannot do anything with.
                    Vec3::new(shot.direction.x, 0.0, shot.direction.z).normalize_or_zero()
                        * DEATH_PUSH_SPEED
                        * (damage / shared::HEAD_DAMAGE).clamp(0.6, 2.0),
                    hit_point,
                    &corpse_stages,
                );
                respawns.0.push(PendingRespawn { timer: ZOMBIE_RESPAWN_DELAY });
            } else {
                let push = Vec3::new(shot.direction.x, 0.0, shot.direction.z).normalize_or_zero() * KNOCKBACK_FORCE;
                knockback.0 += push;
            }

            commands.trigger(ShotConfirmed { origin: shot.origin, point: hit_point, normal, damage, killed });
        }
        return;
    }

    let Some(hit) = spatial_query.cast_ray(shot.origin, direction, MAX_SHOT_DISTANCE, true, &filter) else {
        return;
    };
    if zombies.contains(hit.entity) {
        return;
    }

    let surface_point = shot.origin + *direction * hit.distance;
    commands.trigger(ShotFx::SurfaceHit { origin: shot.origin, point: surface_point, normal: hit.normal });
}

#[derive(Component)]
pub(crate) struct Dead;

pub(crate) fn on_player_died(
    _trigger: On<PlayerDied>,
    mut commands: Commands,
    mut game_over: ResMut<GameOver>,

    players: Query<Entity, With<Player>>,
    dead: Query<(), With<Dead>>,
) {
    let Ok(entity) = players.single() else {
        return;
    };
    commands.entity(entity).insert(Dead);

    if game_over.0 {
        return;
    }
    let all_dead = players.iter().all(|e| e == entity || dead.contains(e));
    if all_dead {
        game_over.0 = true;
        commands.trigger(GameStateEvent::GameOver);
    }
}

pub(crate) fn on_request_restart(
    _trigger: On<RequestRestart>,
    mut commands: Commands,
    mut game_over: ResMut<GameOver>,
    mut respawns: ResMut<PendingRespawns>,
    scenario: Res<Scenario>,
    points: Res<ZombieSpawnPoints>,
    zombies: Query<Entity, With<Zombie>>,
    mut players: Query<(Entity, &mut Transform, &mut Player), With<Player>>,
) {
    if !game_over.0 {
        return;
    }
    game_over.0 = false;
    respawns.0.clear();

    for entity in &zombies {
        commands.entity(entity).despawn();
    }

    // The wave mode starts empty and lets wave one fill the street, the same
    // way it does on a fresh run. A standing crowd here would be bodies no
    // wave ever budgeted for.
    if !crate::waves::in_wave_mode() {
        let spawns: Vec<(Vec3, f32)> = points
            .0
            .iter()
            .take(scenario.zombie_count())
            .map(|p| (*p + Vec3::Y * ZOMBIE_REST_HEIGHT, 0.0))
            .collect();
        spawn_zombies(&mut commands, &spawns, true);
    }
    let player_spawn = scenario.player_spawn();

    for (entity, mut transform, mut player) in &mut players {
        commands.entity(entity).remove::<Dead>();
        transform.translation = player_spawn;
        player.vertical_velocity = 0.0;
        player.grounded = false;
    }

    commands.trigger(GameStateEvent::Restarted);
}


const MAX_PENETRATIONS: usize = 2;
const MAX_WALL_THICKNESS: f32 = 0.35;
const PENETRATION_DAMAGE_SCALE: f32 = 0.55;
const PENETRATION_STEP_OFF: f32 = 0.02;

pub(crate) struct Penetration {
    pub(crate) entry: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) exit: Vec3,
}

pub(crate) struct ShotPath {
    pub(crate) through: Vec<Penetration>,
    pub(crate) blocked_at: Option<(f32, Vec3, Vec3)>,
}

impl ShotPath {
    pub(crate) fn blocking_distance(&self) -> Option<f32> {
        self.blocked_at.map(|(distance, _, _)| distance)
    }

    pub(crate) fn damage_scale_at(&self, origin: Vec3, distance: f32) -> f32 {
        let passed = self
            .through
            .iter()
            .filter(|wall| origin.distance(wall.entry) < distance)
            .count();
        PENETRATION_DAMAGE_SCALE.powi(passed as i32)
    }
}

/// Measures how thick the surface at `entry` is by probing back toward it from
/// beyond. Casting forward from inside the wall would need the ray to report
/// back faces; coming the other way, the first thing hit is the far side.
fn exit_point(
    spatial_query: &SpatialQuery,
    entry: Vec3,
    direction: Dir3,
    filter: &SpatialQueryFilter,
) -> Option<Vec3> {
    let beyond = entry + *direction * MAX_WALL_THICKNESS;
    let back = Dir3::new(-*direction).ok()?;
    let hit = spatial_query.cast_ray(beyond, back, MAX_WALL_THICKNESS, true, filter)?;
    let thickness = MAX_WALL_THICKNESS - hit.distance;
    (thickness > 0.0).then(|| entry + *direction * thickness)
}

/// Walks the shot forward through anything thin enough to punch through,
/// stopping at the first wall that is not.
pub(crate) fn trace_shot_path(
    spatial_query: &SpatialQuery,
    origin: Vec3,
    direction: Dir3,
    filter: &SpatialQueryFilter,
    stops: &dyn Fn(Entity) -> bool,
) -> ShotPath {
    let mut through = Vec::new();
    let mut from = origin;

    loop {
        let remaining = MAX_SHOT_DISTANCE - origin.distance(from);
        if remaining <= 0.0 {
            return ShotPath { through, blocked_at: None };
        }
        let Some(hit) = spatial_query.cast_ray(from, direction, remaining, true, filter) else {
            return ShotPath { through, blocked_at: None };
        };
        let entry = from + *direction * hit.distance;
        let entry_distance = origin.distance(entry);

        // Cover, not a thin wall: nothing is looked for behind it and it
        // costs none of the penetration budget, because it is not spent --
        // the shot ends here.
        if stops(hit.entity) {
            return ShotPath { through, blocked_at: Some((entry_distance, entry, hit.normal)) };
        }
        if through.len() >= MAX_PENETRATIONS {
            return ShotPath { through, blocked_at: Some((entry_distance, entry, hit.normal)) };
        }
        let Some(exit) = exit_point(spatial_query, entry, direction, filter) else {
            return ShotPath { through, blocked_at: Some((entry_distance, entry, hit.normal)) };
        };
        through.push(Penetration { entry, normal: hit.normal, exit });
        from = exit + *direction * PENETRATION_STEP_OFF;
    }
}
