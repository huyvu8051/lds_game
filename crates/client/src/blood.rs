use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::*;
use shared::player::GameLayer;

#[derive(Component)]
pub(crate) struct BloodParticle {
    pub(crate) velocity: Vec3,
    pub(crate) remaining: f32,
}

/// What blood is allowed to land on: the world, and nothing that is passing
/// through it.
///
/// An unparented stain is a world point, so leaving one on anything that
/// moves hangs it in the air the moment that thing moves on. A ragdoll is
/// fifteen loose capsules thrown out at the instant of death, right across
/// the path of the spray from the shot that caused it -- which is why the
/// stains appeared on kills and almost never otherwise. Severed limbs and
/// spent casings do the same thing more quietly.
///
/// They all carry their own layer, so one mask covers all three. Living
/// zombies need nothing: they have no collider at all -- a shot is measured
/// against capsules cast at the bone transforms in `hitbox.rs` and never
/// entered into the physics world -- so a ray passes straight through one.
fn world_filter() -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask([GameLayer::Default])
}

#[derive(Component)]
pub(crate) struct BloodStain {
    pub(crate) remaining: f32,
    variant: usize,
}

#[derive(Resource)]
pub(crate) struct BloodAssets {
    pub(crate) particle_mesh: Handle<Mesh>,
    pub(crate) particle_material: Handle<StandardMaterial>,
    /// Smaller than the others: this is one droplet landing, not a wound.
    pub(crate) droplet_stain_mesh: Handle<Mesh>,
    pub(crate) floor_stain_mesh: Handle<Mesh>,
    /// A fade ladder per variant, `[variant][step]` -- a shared handle per
    /// step rather than a material per stain, so everything at the same step
    /// still batches together.
    pub(crate) stain_materials: Vec<Vec<Handle<StandardMaterial>>>,
    pub(crate) wall_stain_mesh: Handle<Mesh>,
    pub(crate) burst_mesh: Handle<Mesh>,
    /// Frame sequences, `[variant][frame]`. A flipbook, not a fade ladder.
    pub(crate) burst_materials: Vec<Vec<Handle<StandardMaterial>>>,
    /// The same frames at a larger, fainter, cooler tint -- the mist that
    /// hangs behind the spray. Shipped effects layer three to five elements
    /// into one impact; this is the cheapest second one.
    pub(crate) mist_materials: Vec<Vec<Handle<StandardMaterial>>>,
    pub(crate) mist_mesh: Handle<Mesh>,
}

pub(crate) fn spawn_blood_effects(
    commands: &mut Commands,
    blood_assets: &BloodAssets,
    spatial_query: &SpatialQuery,
    meshes: &mut Assets<Mesh>,
    time: &Time,
    hit_point: Vec3,
    hit_normal: Vec3,
    shot_direction: Vec3,
    blood: BloodEnabled,
) {
    if blood.spray {
        spawn_blood_burst(commands, blood_assets, hit_point, hit_normal, time.elapsed_secs() * 311.0);

        for i in 0..BLOOD_PARTICLE_COUNT {
            let seed = time.elapsed_secs() * 977.0 + i as f32 * 131.7;
            let dir = Vec3::new(ops::sin(seed * 1.3), ops::sin(seed * 0.7).abs() * 0.6, ops::sin(seed * 2.1))
                .normalize_or_zero();
            commands.spawn((
                Mesh3d(blood_assets.particle_mesh.clone()),
                MeshMaterial3d(blood_assets.particle_material.clone()),
                Transform::from_translation(hit_point),
                BloodParticle {
                    velocity: dir * BLOOD_PARTICLE_SPEED,
                    remaining: BLOOD_PARTICLE_LIFETIME,
                },
            ));
        }
    }

    if !blood.stains {
        return;
    }
    // Nothing is left on the body itself. A 22cm quad parented to a bone
    // stood 4cm clear of the torso at its edges and crossed the silhouette,
    // which is what read as pasted-on -- and the games this is modelled on
    // do not keep wounds on a body either, because the body is gone before
    // anyone can look. Here it dissolves 1.1s after it stops moving.
    //
    // The exit wound is a stain on whatever is behind the target, so it goes
    // with the stains rather than with the spray.
    spawn_exit_wound(commands, blood_assets, spatial_query, meshes, hit_point, shot_direction, time.elapsed_secs() * 541.0);


    let range = (BLOOD_FLOOR_STAIN_MAX - BLOOD_FLOOR_STAIN_MIN + 1) as f32;
    let count = (BLOOD_FLOOR_STAIN_MIN as f32 + ops::sin(time.elapsed_secs() * 457.0).abs() * range) as usize;
    let count = count.clamp(BLOOD_FLOOR_STAIN_MIN, BLOOD_FLOOR_STAIN_MAX);

    let Ok(down) = Dir3::new(Vec3::NEG_Y) else {
        return;
    };
    let filter = world_filter();
    for i in 0..count {
        let seed = time.elapsed_secs() * 613.0 + i as f32 * 219.3;
        let jitter = Vec3::new(ops::sin(seed), 0.0, ops::sin(seed * 1.7)) * BLOOD_FLOOR_STAIN_SPREAD;
        let origin = hit_point + jitter;
        if let Some(floor_hit) = spatial_query.cast_ray(origin, down, BLOOD_FLOOR_RAYCAST_DISTANCE, true, &filter) {
            let floor_point = origin + Vec3::NEG_Y * floor_hit.distance;

            spawn_blood_stain(commands, blood_assets, meshes, spatial_query, floor_point, floor_hit.normal, seed, blood_assets.floor_stain_mesh.clone(), BLOOD_FLOOR_STAIN_SIZE);
        }
    }
}

fn stain_variant(blood_assets: &BloodAssets, seed: f32) -> Option<usize> {
    let materials = &blood_assets.stain_materials;
    if materials.is_empty() {
        return None;
    }
    let pick = (ops::sin(seed) * 43758.547).fract().abs();
    Some((pick * materials.len() as f32) as usize % materials.len())
}

fn stain_rotation(normal: Vec3, seed: f32) -> Quat {
    let spin = (ops::sin(seed * 1.7) * 43758.547).fract().abs() * std::f32::consts::TAU;
    Quat::from_rotation_arc(Vec3::Z, normal.normalize_or_zero()) * Quat::from_rotation_z(spin)
}

/// Where the surface is beneath one sample of a stain, and which way it
/// faces. `None` where there is nothing to take a stain: past the lip of a
/// crate, or on a face turned too far to be the same surface.
fn surface_under(
    spatial_query: &SpatialQuery,
    at: Vec3,
    normal: Dir3,
    lift: f32,
    reach: f32,
) -> Option<(Vec3, Vec3)> {
    let from = at + *normal * lift;
    let hit = spatial_query.cast_ray(from, -normal, lift + reach, true, &world_filter())?;
    (hit.normal.dot(*normal) >= BLOOD_STAIN_FACING_MIN)
        .then(|| (from - *normal * hit.distance, hit.normal))
}

/// Whether the four corners of a stain sit close enough to the plane of the
/// hit for a flat quad to pass.
fn lies_flat(
    spatial_query: &SpatialQuery,
    point: Vec3,
    u: Vec3,
    v: Vec3,
    normal: Dir3,
    size: f32,
) -> bool {
    let half = size * 0.5;
    let depth = size * BLOOD_STAIN_CONFORM_DEPTH;
    [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].iter().all(|(su, sv): &(f32, f32)| {
        match surface_under(spatial_query, point + u * su * half + v * sv * half, normal, depth, depth) {
            Some((hit, _)) => (hit - point).dot(*normal).abs() <= BLOOD_STAIN_FLAT_TOLERANCE,
            None => false,
        }
    })
}

/// A stain built to fit the surface under it, or `None` where nothing under
/// it can take one.
///
/// This is what a projected decal gets out of the depth buffer, done on the
/// CPU at spawn instead: sample the surface across the stain's footprint and
/// make geometry out of whatever answers. A cell with a corner over open air
/// is dropped, so the stain stops dead at an edge rather than hanging past
/// it into the sky, and every vertex carries the normal of the face it
/// landed on, so the lighting is the receiving surface's own rather than one
/// flat plane's.
///
/// Like a projected decal it does not carry round a sharp corner. A face
/// turned too far is rejected rather than stretched across it, which is the
/// same choice and for the same reason: a decal dragged onto a perpendicular
/// face reads as streaks.
fn conforming_stain_mesh(
    spatial_query: &SpatialQuery,
    point: Vec3,
    u: Vec3,
    v: Vec3,
    normal: Dir3,
    size: f32,
) -> Option<Mesh> {
    let depth = size * BLOOD_STAIN_CONFORM_DEPTH;
    let row = BLOOD_STAIN_GRID + 1;
    let mut positions = Vec::with_capacity(row * row);
    let mut normals = Vec::with_capacity(row * row);
    let mut uvs = Vec::with_capacity(row * row);
    let mut found = Vec::with_capacity(row * row);

    for j in 0..row {
        for i in 0..row {
            let fx = i as f32 / BLOOD_STAIN_GRID as f32;
            let fy = j as f32 / BLOOD_STAIN_GRID as f32;
            let at = point + u * (fx - 0.5) * size + v * (fy - 0.5) * size;
            match surface_under(spatial_query, at, normal, depth, depth) {
                Some((hit, hit_normal)) => {
                    let lifted = hit + hit_normal.normalize_or_zero() * BLOOD_STAIN_SURFACE_OFFSET;
                    positions.push((lifted - point).to_array());
                    normals.push(hit_normal.to_array());
                    found.push(true);
                }
                None => {
                    positions.push([0.0; 3]);
                    normals.push(normal.to_array());
                    found.push(false);
                }
            }
            uvs.push([fx, 1.0 - fy]);
        }
    }

    // Winding so the face looks back along the normal: u cross v is the
    // normal, and the material does not cull anyway, but the lighting reads
    // the normals these carry.
    let mut indices: Vec<u32> = Vec::new();
    for j in 0..BLOOD_STAIN_GRID {
        for i in 0..BLOOD_STAIN_GRID {
            let a = j * row + i;
            let (b, c, d) = (a + 1, a + row, a + row + 1);
            if found[a] && found[b] && found[c] && found[d] {
                indices.extend_from_slice(&[
                    a as u32, b as u32, c as u32,
                    b as u32, d as u32, c as u32,
                ]);
            }
        }
    }
    if indices.is_empty() {
        return None;
    }
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, crate::gltf_usages())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_indices(Indices::U32(indices)),
    )
}

pub(crate) fn spawn_blood_stain(
    commands: &mut Commands,
    blood_assets: &BloodAssets,
    meshes: &mut Assets<Mesh>,
    spatial_query: &SpatialQuery,
    point: Vec3,
    normal: Vec3,
    seed: f32,
    flat_mesh: Handle<Mesh>,
    size: f32,
) {
    let Some(variant) = stain_variant(blood_assets, seed) else {
        return;
    };
    let Some(material) = blood_assets.stain_materials.get(variant).and_then(|ladder| ladder.first()) else {
        return;
    };
    let Ok(normal) = Dir3::new(normal) else {
        return;
    };
    let rotation = stain_rotation(*normal, seed);
    let (u, v) = (rotation * Vec3::X, rotation * Vec3::Y);

    let (mesh, transform) = if size >= BLOOD_STAIN_CONFORM_MIN_SIZE
        && !lies_flat(spatial_query, point, u, v, normal, size)
    {
        // Built in the stain's own frame, so the entity carries no rotation
        // and each vertex keeps the normal it was found with. Nothing back
        // means nothing under it to stain, and no stain is the right answer
        // -- the flat quad here is what used to hang in the air.
        let Some(built) = conforming_stain_mesh(spatial_query, point, u, v, normal, size) else {
            return;
        };
        (meshes.add(built), Transform::from_translation(point))
    } else {
        (
            flat_mesh,
            Transform::from_translation(point + *normal * BLOOD_STAIN_SURFACE_OFFSET)
                .with_rotation(rotation),
        )
    };

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        transform,
        BloodStain { remaining: BLOOD_STAIN_LIFETIME, variant },
    ));
}

pub(crate) fn update_blood_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    spatial_query: SpatialQuery,
    blood_assets: Res<BloodAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    blood: Res<BloodEnabled>,
    mut query: Query<(Entity, &mut Transform, &mut BloodParticle)>,
) {
    let dt = time.delta_secs();
    let eye = camera.single().ok().map(|global| global.translation());
    // A severance bleeds from both faces of the cut for over a second, so a
    // few at once puts hundreds of these up. Oldest go first, as elsewhere.
    let mut over_budget = query.iter().len().saturating_sub(MAX_BLOOD_PARTICLES);
    for (entity, mut transform, mut particle) in &mut query {
        if over_budget > 0 && particle.remaining > 0.1 {
            particle.remaining = 0.1;
            over_budget -= 1;
        }
        particle.velocity.y += BLOOD_PARTICLE_GRAVITY * dt;
        let step = particle.velocity * dt;

        // Where a droplet lands is where a stain belongs. The spray and the
        // staining used to be unrelated -- droplets flew and vanished, while
        // stains were placed by their own rays from the hit point -- so blood
        // never arrived anywhere it had been seen going. Cast along the step
        // about to be taken, so nothing is missed between frames.
        if blood.stains && particle.remaining > 0.0 {
            if let Ok(dir) = Dir3::new(step) {
                let reach = step.length();
                if reach > 1e-4 {
                    if let Some(hit) =
                        spatial_query.cast_ray(transform.translation, dir, reach, true, &world_filter())
                    {
                        let at = transform.translation + *dir * hit.distance;
                        spawn_blood_stain(
                            &mut commands,
                            &blood_assets,
                            &mut meshes,
                            &spatial_query,
                            at,
                            hit.normal,
                            at.x * 311.0 + at.z * 97.0,
                            blood_assets.droplet_stain_mesh.clone(),
                            BLOOD_DROPLET_STAIN_SIZE,
                        );
                        commands.entity(entity).try_despawn();
                        continue;
                    }
                }
            }
        }
        transform.translation += step;

        if let Some(eye) = eye {
            let to_eye = (eye - transform.translation).normalize_or_zero();
            if to_eye != Vec3::ZERO {
                transform.rotation = Quat::from_rotation_arc(Vec3::Z, to_eye);
            }
        }

        particle.remaining -= dt;
        if particle.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Stains are long-lived but not unbounded, and they go out by fading.
///
/// Written against `update_bullet_holes`, which had the same problem first:
/// an unbounded decal spawn is a leak, and at ten rounds a second it is a
/// fast one. Over budget, the oldest are told to leave early -- every stain
/// is born with the same lifetime, so `remaining` is age, and the smallest
/// are the oldest.
pub(crate) fn update_blood_stains(
    mut commands: Commands,
    time: Res<Time>,
    blood_assets: Res<BloodAssets>,
    mut query: Query<(Entity, &mut BloodStain, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let over_budget = query.iter().len().saturating_sub(MAX_BLOOD_STAINS);
    // The cutoff is the age of the last stain that has to go, so one pass can
    // decide every stain without sorting the whole set.
    let cutoff = (over_budget > 0).then(|| {
        let mut ages: Vec<f32> = query.iter().map(|(_, stain, _)| stain.remaining).collect();
        let nth = (over_budget - 1).min(ages.len() - 1);
        ages.select_nth_unstable_by(nth, f32::total_cmp);
        ages[nth]
    });

    for (entity, mut stain, mut material) in &mut query {
        if cutoff.is_some_and(|cutoff| stain.remaining <= cutoff)
            && stain.remaining > BLOOD_STAIN_FADE_SECONDS
        {
            stain.remaining = BLOOD_STAIN_FADE_SECONDS;
        }
        stain.remaining -= time.delta_secs();
        if stain.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        let Some(ladder) = blood_assets.stain_materials.get(stain.variant) else {
            continue;
        };
        let faded = 1.0 - (stain.remaining / BLOOD_STAIN_FADE_SECONDS).clamp(0.0, 1.0);
        let step = ((faded * BLOOD_STAIN_FADE_STEPS as f32) as usize).min(BLOOD_STAIN_FADE_STEPS - 1);
        if let Some(rung) = ladder.get(step) {
            if material.0 != *rung {
                material.0 = rung.clone();
            }
        }
    }
}


#[derive(Component)]
pub(crate) struct BloodBurst {
    remaining: f32,
    lifetime: f32,
    spin: f32,
    variant: usize,
    /// A per-burst size, on top of the growth every burst shares. Only the
    /// mist uses it, to keep its puffs from being one shape repeated.
    size: f32,
    /// Which layer's frames to step through, and how long it has to do it.
    layer: BurstLayer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BurstLayer {
    Spray,
    Mist,
}

fn spawn_blood_burst(
    commands: &mut Commands,
    blood_assets: &BloodAssets,
    hit_point: Vec3,
    hit_normal: Vec3,
    seed: f32,
) {
    let variants = &blood_assets.burst_materials;
    if variants.is_empty() {
        return;
    }
    let variant = ((ops::sin(seed).abs() * variants.len() as f32) as usize).min(variants.len() - 1);
    let at = hit_point + hit_normal.normalize_or_zero() * BLOOD_BURST_NORMAL_OFFSET;

    // Two layers, offset in time. One effect reads as a sticker; shipped
    // impacts stack three to five, and the cheapest second one is the same
    // sequence played slower, larger and fainter behind the first.
    if let Some(first) = variants.get(variant).and_then(|frames| frames.first()) {
        commands.spawn((
            Mesh3d(blood_assets.burst_mesh.clone()),
            MeshMaterial3d(first.clone()),
            Transform::from_translation(at).with_scale(Vec3::splat(BLOOD_BURST_START_SCALE)),
            BloodBurst {
                remaining: BLOOD_BURST_LIFETIME,
                lifetime: BLOOD_BURST_LIFETIME,
                spin: seed,
                variant,
                size: 1.0,
                layer: BurstLayer::Spray,
            },
        ));
    }

    // The mist is a cloud rather than a second sticker, so it is several
    // puffs and not one: same spot, same sequence and same size three times
    // over is indistinguishable from one puff at three times the opacity.
    let mist = &blood_assets.mist_materials;
    if mist.is_empty() {
        return;
    }
    let spread = BLOOD_BURST_SIZE * BLOOD_MIST_SIZE_SCALE * BLOOD_MIST_DRIFT;
    for i in 0..BLOOD_MIST_PUFFS {
        let s = seed + i as f32 * 157.3;
        let puff = ((ops::sin(s).abs() * mist.len() as f32) as usize).min(mist.len() - 1);
        let Some(first) = mist.get(puff).and_then(|frames| frames.first()) else {
            continue;
        };
        let drift = Vec3::new(ops::sin(s * 1.7), ops::sin(s * 2.3), ops::sin(s * 3.1)) * spread;
        let size = 0.7 + ops::sin(s * 4.7).abs() * 0.6;
        let lifetime =
            BLOOD_BURST_LIFETIME * BLOOD_MIST_LIFETIME_SCALE * (0.8 + ops::sin(s * 5.3).abs() * 0.4);
        commands.spawn((
            Mesh3d(blood_assets.mist_mesh.clone()),
            MeshMaterial3d(first.clone()),
            Transform::from_translation(at + drift)
                .with_scale(Vec3::splat(BLOOD_BURST_START_SCALE * size)),
            BloodBurst { remaining: lifetime, lifetime, spin: s, variant: puff, size, layer: BurstLayer::Mist },
        ));
    }
}

pub(crate) fn update_blood_bursts(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    blood_assets: Res<BloodAssets>,
    mut query: Query<(Entity, &mut Transform, &mut BloodBurst, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let dt = time.delta_secs();
    let eye = camera.single().ok().map(|global| global.translation());
    for (entity, mut transform, mut burst, mut material) in &mut query {
        burst.remaining -= dt;
        if burst.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        let life = 1.0 - (burst.remaining / burst.lifetime).clamp(0.0, 1.0);
        let eased_out = 1.0 - (1.0 - life) * (1.0 - life);
        transform.scale = Vec3::splat(
            (BLOOD_BURST_START_SCALE + (BLOOD_BURST_END_SCALE - BLOOD_BURST_START_SCALE) * eased_out) * burst.size,
        );

        if let Some(eye) = eye {
            let to_eye = (eye - transform.translation).normalize_or_zero();
            if to_eye != Vec3::ZERO {
                transform.rotation = Quat::from_rotation_arc(Vec3::Z, to_eye) * Quat::from_rotation_z(burst.spin);
            }
        }
        let sequences = match burst.layer {
            BurstLayer::Spray => &blood_assets.burst_materials,
            BurstLayer::Mist => &blood_assets.mist_materials,
        };
        let Some(frames) = sequences.get(burst.variant) else {
            continue;
        };
        if let Some(frame) = frames.get(crate::blood_burst::burst_frame_index(life)) {
            if material.0 != *frame {
                material.0 = frame.clone();
            }
        }
    }
}

fn spawn_exit_wound(
    commands: &mut Commands,
    blood_assets: &BloodAssets,
    spatial_query: &SpatialQuery,
    meshes: &mut Assets<Mesh>,
    hit_point: Vec3,
    shot_direction: Vec3,
    seed_base: f32,
) {
    let Ok(forward) = Dir3::new(shot_direction) else {
        return;
    };
    let lateral = forward.any_orthonormal_vector();
    let other = forward.cross(lateral);

    for i in 0..BLOOD_EXIT_PARTICLE_COUNT {
        let seed = seed_base + i as f32 * 97.3;
        let spread = (lateral * ops::sin(seed * 1.9) + other * ops::sin(seed * 2.7)) * BLOOD_EXIT_CONE;
        let dir = (*forward + spread).normalize_or_zero();
        let speed = BLOOD_EXIT_SPEED * (0.65 + ops::sin(seed * 3.3).abs() * 0.35);
        commands.spawn((
            Mesh3d(blood_assets.particle_mesh.clone()),
            MeshMaterial3d(blood_assets.particle_material.clone()),
            Transform::from_translation(hit_point),
            BloodParticle {
                velocity: dir * speed,
                remaining: BLOOD_PARTICLE_LIFETIME,
            },
        ));
    }

    let exit_point = hit_point + *forward * BLOOD_EXIT_WOUND_OFFSET;
    spawn_blood_burst(commands, blood_assets, exit_point, *forward, seed_base * 1.7);

    let filter = world_filter();
    for i in 0..BLOOD_WALL_STAIN_COUNT {
        let seed = seed_base + i as f32 * 263.1;
        let jitter = (lateral * ops::sin(seed * 1.3) + other * ops::sin(seed * 2.1)) * BLOOD_WALL_STAIN_SPREAD;
        let Ok(ray) = Dir3::new(*forward + jitter) else {
            continue;
        };
        let Some(wall) = spatial_query.cast_ray(exit_point, ray, BLOOD_WALL_RAYCAST_DISTANCE, true, &filter) else {
            continue;
        };
        let wall_point = exit_point + *ray * wall.distance;
        spawn_blood_stain(commands, blood_assets, meshes, spatial_query, wall_point, wall.normal, seed, blood_assets.wall_stain_mesh.clone(), BLOOD_WALL_STAIN_SIZE);
    }
}

/// A round droplet with a soft rim, generated rather than shipped: the
/// particle quad was drawn with flat colour and no alpha, so every droplet
/// was a hard red square.
pub(crate) fn droplet_image(size: u32) -> Image {
    let centre = (size as f32 - 1.0) * 0.5;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - centre) / centre;
            let dy = (y as f32 - centre) / centre;
            let edge = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            // cubed keeps a solid core and spends the falloff near the rim
            let alpha = edge * edge * edge;
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        // Not RENDER_WORLD: that drops the CPU copy once the GPU has it, and
        // the native extractor reads image.data -- the droplet then drew as
        // an untextured square.
        crate::gltf_usages(),
    )
}

/// What blood is drawn. Two flags because they cost differently: spray clears
/// itself within a second, a stain sits there until its lifetime runs out.
#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct BloodEnabled {
    pub(crate) spray: bool,
    pub(crate) stains: bool,
}
