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
| Mist burst | the same, at a larger, fainter tint | 0.46s |
| Droplets | `BloodParticle`, `update_blood_particles` | 0.6s |
| Stains | `spawn_blood_stain`, `BloodStain` | a corpse's lifetime |

A hit spawns, in order: a two-layer burst at the entry point, eleven
droplets, a second two-layer burst at the exit point with its own droplets,
a stain parented to the bone that was hit, one to three floor stains, and up
to a few wall stains behind the target. Droplets then leave their own marks
where they land, so the count is not fixed.

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

## Not done

**Stains jut out at wall edges.** A stain is a flat quad, so on a corner half
of it hangs in open air. The real answer is a projected decal — draw it as a
box volume and have the fragment shader rebuild world position from the depth
buffer, discarding what falls outside — which is a new pass in the Metal
renderer, not an edit to `blood.rs`. See `SHADER_PLAN.md`. The cheap half is
to raycast the quad's four corners and shrink the stain when one finds no
surface: that ends the jutting without shader work, but does not wrap a
corner. **Neither is chosen yet.**

**Soft particles.** Droplets and bursts cut hard against geometry they
intersect. Same work package, same reason: it needs the depth buffer readable
in a fragment shader.

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
