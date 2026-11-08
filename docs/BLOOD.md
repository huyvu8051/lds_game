# Blood

What a round does when it goes into a body, in the four parts it is built
from, and what each part is worth looking at for.

Everything here is generated at load. No blood texture ships: the droplet,
the burst frames and the stain variants are all drawn into `Image`s during
`setup_common`, which is why changing any of them is an edit to a function
rather than a trip through an art tool.

## The pieces

| Piece | Where | Lives |
|---|---|---|
| Spray burst | `blood_burst.rs`, `spawn_blood_burst` | 0.22s |
| Mist cloud | the same frames, two puffs, larger and fainter | 0.37-0.55s |
| Droplets | `BloodParticle`, `update_blood_particles` | 0.6s |
| Stains | `spawn_blood_stain`, `update_blood_stains` | 45s pools, 10s specks |
| Wounds | `wounds.rs`, skinned shader | as long as the body |

A hit spawns, in order: a burst at the entry point with its mist behind it,
eleven droplets, the same pair again at the exit point with its own droplets,
one to three floor stains, and up to a few wall stains behind the target.
Droplets then leave their own marks where they land, so the count is not
fixed. Nothing is left on the body itself — see below.

The mist is a cloud rather than a second sticker, so it is `BLOOD_MIST_PUFFS`
quads and not one, each with its own frame sequence, size, lifetime and
drift. Puffs on the same spot at the same size would be one puff at that many
times the opacity. Four numbers shape it: `BLOOD_MIST_SIZE_SCALE` and
`BLOOD_MIST_PUFFS` for how big and how many, `BLOOD_MIST_DRIFT` for how far
they scatter, and the alpha of the tint in `scene_setup.rs` for how solid
each one reads. Fewer but denser is cloud; more but fainter is haze.

## What blood is allowed to land on

`world_filter()` in `blood.rs`, used by every ray blood casts.

An unparented stain is a world point. Leave one on anything that moves and it
hangs in the air the moment that thing moves on. Death throws fifteen loose
ragdoll capsules out across the path of the spray from the shot that caused
it, so the stains appeared on kills and hardly ever otherwise; severed limbs
and spent casings did the same more quietly.

Ragdolls and limbs are on `GameLayer::Corpse`, casings and dropped magazines
on `GameLayer::Debris`, and the map is on `Default`. One mask covers all
three.

**A living zombie needs no exclusion: it has no collider at all.** A shot is
measured against capsules cast at the bone transforms in `hitbox.rs`, which
are never entered into the physics world. A ray passes straight through one.
An earlier fix excluded the entity that was shot and therefore did nothing.

**Known gap.** A grenade (`grenade.rs`) declares no layer, so it sits on
`Default` and a droplet can still mark one in flight. Fixing it means moving
grenades to `Debris`, not changing anything here.

## Why the burst is a flipbook

It used to be one picture — `splatter_0.png`, the same dried stain the floor
gets — scaled from 0.35 to 1.7 and faded out. A still image growing and
dimming reads as a decal expanding, not as something thrown, and no amount of
tuning the scale or the fade changes that.

`blood_burst.rs` generates a real sequence instead: 26 blobs thrown out from
the centre, each **shrinking** as it travels rather than fading, so the
sequence breaks up into droplets instead of dimming as one sheet. Reach is
squared, so most blobs stay near the middle and a few run ahead — an even
spread reads as a ring.

Sixteen frames of 96 square is 590KB a variant, less than the single 512
splatter each burst used to borrow.

Frames are **white, with their shape in the alpha channel**. The material
carries the colour, which is what lets one sequence serve both the spray and
the mist.

### Motion vectors, deliberately not done

They exist to interpolate between frames so a sprite sheet can ship fewer of
them. Nothing here is shipped, so there is nothing to save: the answer is to
generate enough frames. At 0.22s and 60Hz the eye gets about thirteen, and
there are sixteen.

## How a stain sits on a surface

Two things that have nothing to do with the texture, both taken from
`bullet_holes.rs`, which met the same problems first.

**Offset is millimetres, bias does the rest.** A stain sits
`BLOOD_STAIN_SURFACE_OFFSET` (6mm) off the surface and carries
`depth_bias: 10.0` in its material. It used to be a centimetre of offset and
no bias, which is visible as a sheet floating off a wall at a grazing angle
and still z-fights at range. Physical offset buys nothing that depth bias
does not buy better.

**Stains fade, and there is a ceiling.** `MAX_BLOOD_STAINS` is 256.

A pool lasts `BLOOD_STAIN_LIFETIME`, 45s, which is what a bullet hole lasts:
there is no argument for a chipped wall outliving the blood under it. A
droplet's mark lasts `BLOOD_DROPLET_STAIN_LIFETIME`, 10s, and that difference
is doing work. Up to twenty-two specks land per shot against one or two
pools, so the specks are what fills the budget — and since the budget takes
whatever is nearest death first, and a speck's shorter life always leaves it
nearer, a speck drains before a pool it would otherwise have evicted. Without
that split the ceiling is reached in about ten shots and the pools are the
first thing to go, which is exactly backwards.
 Stains
were the only blood effect with no bound: particles have had
`MAX_BLOOD_PARTICLES` all along, and since droplets began marking where they
land the number a shot leaves is not even fixed — ten rounds a second against
a twelve-second life is nine hundred of them. Over budget, the oldest are
told to leave early. Every stain is born with the same lifetime, so
`remaining` is age and the smallest are the oldest; one `select_nth_unstable`
finds the cutoff without sorting the set.

They fade out over `BLOOD_STAIN_FADE_SECONDS` through a shared ladder of
materials rather than a material per stain, so everything on the same rung
still batches. Before this a stain vanished between one frame and the next.

## Fitting the surface, without a depth buffer

A stain used to be a flat quad laid tangent at the hit. On a corner, half of
it hung in open air; on anything curved it clipped through. The shipped
answer is a projected decal: draw the stain as a box volume and have the
fragment shader rebuild world position from the depth buffer, discarding what
falls outside the box, so the stain is painted by the surface receiving it
and has no geometry of its own to stick out.

**That was priced and rejected.** The Metal renderer has no depth prepass and
throws depth away (`set_store_action(DontCare)`), and scene and UI share one
command encoder, so a decal pass means cutting one pass into three. On a tile
GPU that is the thing not to do, and it breaks the MSAA resolve besides,
which sits at the end of the one pass today. The alternative that keeps one
pass — a second colour attachment carrying world position, read back through
programmable blending — costs no bandwidth but means touching both scene
shaders and about twelve pipeline descriptors.

**What is done instead is the same idea on the CPU, at spawn.** Where a
projected decal asks the depth buffer what surface is under each pixel, this
asks the physics world what surface is under each of 81 sample points, and
builds geometry out of the answers:

- Each grid point casts along the stain's normal from `BLOOD_STAIN_CONFORM_DEPTH`
  above the hit plane to the same distance below. The hit becomes a vertex,
  **carrying its own normal**, so the lighting is the receiving surface's
  rather than one flat plane's.
- A cell with any corner over open air is dropped. The stain stops dead at an
  edge instead of hanging past it.
- A face turned further than `BLOOD_STAIN_FACING_MIN` from the stain's normal
  is rejected, not stretched onto. A projected decal makes the same choice
  for the same reason, so like one, **this does not carry round a sharp
  corner** — it ends at it.
- `BLOOD_STAIN_CONFORM_DEPTH` is the depth of the decal box by another name,
  and decides the same thing: a step inside the reach is draped over, a lip
  taller than it ends the stain.

**The common case still costs four rays and still batches.** Before building
anything, the four corners are sampled; if they all sit within
`BLOOD_STAIN_FLAT_TOLERANCE` of the hit plane, the stain is laid flat on the
shared quad exactly as before. Only a stain that is actually on something
uneven pays for 81 rays and a mesh of its own. Marks under
`BLOOD_STAIN_CONFORM_MIN_SIZE` — the 10cm droplet marks, which are also the
most numerous — skip the test entirely.

**What this does not fix** is the sorting: stains are still alpha-blended
quads that sort per object, so two overlapping ones can swap order as the
camera moves. Only writing into a GBuffer fixes that, and there is no
GBuffer.

## Marks on the body, in bind space

A round leaves a mark where it went in, and the body keeps it.

**It is not a decal, and it is not painted into a texture.** Both were tried
on paper and both were wrong here. A projected decal reads the depth buffer,
which knows where a chest is this frame but not which bone a mark belongs to,
so it cannot follow an animation. The shipped answer for characters is to
paint into the character's own UV space -- render the mesh with UVs as clip
position, project the skinned world position through the hit's box, discard
outside, accumulate in a small per-character buffer. That wants UVs that do
not overlap, and four of the seven zombie characters fail it: `cop`,
`parasite` and `romero` have UV triangle area over 1.0, and `classic` and
`parasite` run outside the unit square entirely.

**Bind space needs neither.** `v.position` in the skinned vertex shader is
the vertex before any bone moved it, and the bind pose never changes. A point
recorded there follows the animation for nothing. The cost is two varyings
and a short loop.

Where it is kept: `wounds.rs` holds a `ZombieWounds` per body, twelve deep,
oldest first out. The shot path records the hit in the nearest bone's own
frame, because it knows which bone was hit and has no business querying
assets; `resolve_zombie_wounds` finishes the conversion with
`inverse_bind⁻¹ * local`, which is the inverse of what `joint_matrices_for`
builds. The renderer takes them as twelve `float4` in `MeshUniforms` -- xyz
the point, w the reach -- and blends them into the albedo before the light
touches it, so a mark takes the same shadow as the skin around it.

What makes it read as a wound rather than a stain is three things, and the
texture is the smallest of them:

- **A torn edge.** Three octaves of tiling value noise, generated in the
  renderer, sampled on all three planes of bind space and weighted by the
  bind normal -- triplanar, because there is no UV to sample along. It moves
  each wound's boundary in and out rather than fading the disc, so the edge
  comes apart into streaks. Three samples a fragment, not three a wound.
- **A cavity.** The inner part goes most of the way to black on a tighter
  curve than the blood's. A hole reads as a hole because it is dark; nothing
  about it is modelled.
- **A wet highlight.** Shininess and specular strength both lerp toward a
  tighter, brighter pair across the wound, so blood catches a hard little
  highlight the cloth beside it does not. At zero they are exactly the
  constants everything else uses.

Plus a crater: the bloodied area lifts, the cavity sinks, and the
screen-space gradient of that height bends the normal. Unparametrised bump
mapping, so it borrows no tangent and no UV of its own.

**What it gives up** is a painted shape. Each wound is a sphere, so it wraps
a thin limb and marks the far side -- at these radii that reads as blood
having run round a wrist, which is not the wrong picture -- and a body stops
gaining marks at twelve rather than growing a buffer nobody drains.

### The flat quad this replaced

Worth keeping, because the geometry is why it never worked. It was a 22cm
quad parented to the bone. A torso is a cylinder of roughly 17cm radius, and
a flat quad laid tangent to it stands clear of the surface at its edges by

    R - sqrt(R**2 - w**2)  =  0.17 - 0.130  =  4cm

so the edges floated and at the silhouette the quad crossed outside the
body's own outline. Shrinking it helps -- the gap falls with the square of
the quad -- but never the silhouette.

## Not done

**Soft particles.** Droplets and bursts cut hard against geometry they
intersect. It needs the depth buffer readable in a fragment shader, which is
a new pass in the Metal renderer.

**Overlapping stains can swap order.** They are alpha-blended quads sorted
per object by distance to their centre, which is the standard approximation
and the standard failure: two stains on one wall sit at nearly the same
distance, so a small camera move can flip which draws first. Only writing
into a GBuffer fixes it, and there is no GBuffer. The symptom needs two
stains overlapping to show at all.

**A grenade is on `GameLayer::Default`.** `grenade.rs` declares no layer, so
blood lands on one in flight and is stranded when it rolls on. One line, at
the grenade's end rather than the blood's.

## Testing each piece

The four parts landed as four commits so each can be looked at on its own.

```
cargo run -p client --bin native_desktop --profile dev-fast
```

Use `MAP_ID=1` to skip the menu. Bear in mind that `MTL_HUD_ENABLED=1` costs
about 2.4ms of its own — fine for looking, wrong for measuring.

| Commit | Look at |
|---|---|
| `7700d5d` | Kill a zombie and watch where it was standing. No stain should be left hanging in the air as the body falls away. This is the one that only shows on kills. |
| `e60ac76` | Shoot a standing zombie and watch the spray itself. It should break into separate droplets as it goes out, not grow as one patch and dim. |
| `3dc2e76` | The same shot, watching behind the spray. A larger, fainter cloud should outlast it by about twice. |
| `68d0edc` | Shoot near a floor or wall and follow the droplets rather than the impact. Each should leave a small mark where it arrives. |

Two settings gate what is visible: `blood.spray` and `blood.stains`. The
droplet marks are under `stains`, not `spray`.
