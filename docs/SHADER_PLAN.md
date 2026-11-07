# Shader plan

The project had no custom shaders until the damage overlay. This note records
what is worth writing next and, for each one, how much it couples to Bevy's
shader library — because that coupling is what decides the cost of the 0.20
upgrade.

## The upgrade constraint

Bevy 0.20 drops `naga_oil` and moves to [WESL](https://wesl-lang.dev).

| | bevy 0.19.1 | bevy 0.20.0 |
|---|---|---|
| crate `wesl` | 0.3.1 (opt-in via `shader_format_wesl`) | 0.6.0 (always on) |
| preprocessor | naga_oil | WESL |
| GLSL | supported | removed |

The migration guide states plainly: **"Plain WGSL files with no preprocessor
directives keep working."** Everything else changes — imports take a semicolon
and new module paths, `#ifdef` becomes `@if`, `#{BINDING}` becomes
`constants::BINDING`, `#define_import_path` is gone.

WESL itself is pre-1.0 and moved three minor versions between the two Bevy
releases, so WESL-specific syntax written today is a guess at what 0.20 accepts.
Prefer directive-free WGSL.

## Staying on 0.19 for now

Checked 2026-10-16. The upgrade is blocked upstream, not by our code:

| crate | upstream | 0.20 status |
|---|---|---|
| `bevy_landmass`, `landmass_rerecast` | `andriyDev/landmass` | `bevy-0.20` branch exists |
| `bevy_rerecast` | `janhohenheim/rerecast` | open PR bumping to 0.20.0-rc.2 |
| `avian3d` | `avianphysics/avian` | nothing — highest branch is `bevy-0.18`, `main` is 0.19 |

Avian is the blocker and it is 56k lines across 153 files, six times the size
of this client, touching ECS, schedules, reflection and debug rendering.
Forking it to bump Bevy ourselves would cost more than rewriting the game, and
upstream would land their own bump in the meantime.

Our own code is cheap to port by comparison: one shader (`scope_lens.wgsl`),
`ExtractComponent` needing an `AppLabel`, and a deprecated `Interaction` that
still compiles. Checked against all 75 of the 0.20 migration guides — `Val`,
`BorderRadius`, `bevy_shape` and tonemapping all turned out not to apply here.

Watch avian. When it ships 0.20 the rest follows.

## Coupling tiers

**Tier 0 — no imports.** Fragment-only effects over a UI node. Declare the
varyings struct locally instead of importing it. Survives the upgrade untouched.

**Tier 1 — directives, so the file must stay `.wgsl`.** A 3D `Material` that
overrides only the fragment shader needs two things it cannot get any other
way: `VertexOutput`, whose fields are `#ifdef`-gated and so cannot be
redeclared locally, and the material bind group index, which Bevy substitutes
as `#{MATERIAL_BIND_GROUP}` — hardcoding `2` fails validation because that
group holds a storage buffer. Both are naga_oil syntax, so the file must be
named `.wgsl`; naming it `.wesl` routes it to the WESL parser, which rejects
both and panics the render thread. Migration to 0.20 is the import line, the
`#{...}` substitutions becoming `constants::MATERIAL_BIND_GROUP`, and the
rename back to `.wesl`.

**Tier 2 — engine bindings.** Anything reading lights, view uniforms or the
skinning pipeline. Several imports, paths all moved, and the surrounding
`#ifdef` logic has to become `@if`. Worth deferring to 0.20.

## Done

- **Damage overlay** (tier 0) — `assets/shaders/damage_overlay.wesl`. Red
  vignette that tightens and pulses as health drops, plus a hit flash.

- **Scope lens** (tier 1) — `assets/shaders/scope_lens.wgsl`. Barrel
  distortion, chromatic fringing and a vignette on the render-target image.
  Pipeline validates; the look itself has not been checked by eye.
- **Film grain and low-health desaturation** (tier 0) —
  `assets/shaders/screen_effects.wesl`, one post-process pass via
  `FullscreenMaterial`. Grain reseeds per frame and biases toward shadows;
  colour drains to grey as health drops. Measured: high-frequency energy in a
  flat region went from 0.006-0.010 to 0.022.

- **Volumetric muzzle smoke** (tier 1) — `assets/shaders/muzzle_smoke.wgsl`.
  Replaces the single `smoke.png` card with a raymarched puff: 40 steps through
  a soft sphere, density from domain-warped fbm, eroded over the puff life so
  it dissolves instead of fading flat. Each shot passes its own seed, so no two
  puffs repeat. Puff life is 1.9s with drag and buoyancy, so the muzzle push
  bleeds off and the cloud lingers. At 10 shots/sec that would stack 19
  raymarched quads over the same pixels, so live puffs are capped at 9 (oldest
  retired first) and the march is 30 steps. WGSL parses and validates under
  naga 29.0.4; the look itself has not been checked by eye.

  Soft particles (depth-buffer fade where the quad meets geometry) were
  deliberately skipped: that is tier 2, it binds to 0.19 internals, and muzzle
  smoke almost never intersects world geometry. It is worth it for ground smoke,
  not for this.

- **Procedural muzzle flash** (tier 1) — `assets/shaders/muzzle_flash.wgsl`.
  Replaces `muzzle.png`. First attempt used angular harmonics and read as
  cartoon: `sin(angle * n)` is a flower function, and the eye reads those even
  lobes as drawn. Now the rays come from fbm sampled on the unit circle (so
  there is no seam at +/-pi), with a second higher-frequency term that also
  walks along the radius to tear the lobes into ragged streaks. Per-shot seed,
  flicker rather than a smooth fade, deep red fringe through orange to a core
  that blows out white. Additive blend, pushed past 1.0 so bloom catches the
  core. Fourteen embers eject down a narrow cone, reusing SparkParticle.

  Structure follows what practitioners describe on Real Time VFX: a camera-
  facing core plus a separate directional tongue of flame along the bore. That
  second quad is axis-locked and only spun about the bore, so looking down the
  barrel shows it edge-on — the CPU measures how edge-on it is and passes that
  as `shape.y`, giving a fresnel fade without needing engine view bindings. It
  spawns on only half the shots, so consecutive shots are not stamped out.

  Worth recording: AAA does NOT do this procedurally. The norm is hand-painted
  or photographed flipbooks, 4-8 variants cycled at random, and smoke baked out
  of Houdini/EmberGen rather than raymarched live. Our shaders cost more per
  pixel; what that buys is unlimited variation and no texture authoring.
  WGSL parses and validates under naga 29.0.4; not yet checked by eye.

## Next

### Light adaptation — not tier 0 after all
Listed here as easy, which was wrong. Eye adaptation needs the average
luminance of the frame, so it needs a downsample pass feeding back into
exposure — multi-pass, not a single fragment shader. Bevy 0.19 ships no
auto-exposure to lean on either. Worth doing, but it is not a cheap one.

### Wet blood, the rest of it — tier 2
Decals now take real lighting, so there is already a specular that moves with
the view — that part is done without a shader. What is left is control: a
glossy pool interior against a rough dried rim, which needs a roughness map or
a shader reading light bindings. Low priority now that the cheap half landed.

### Burn-away dissolve — tier 2
Death currently sinks and fades the corpse, which needs no shader. A burn edge
creeping across the body needs a material on a skinned mesh — the deepest
coupling of anything here.

### Projected decals — tier 2
Blood is a flat quad, so it clips through curved walls and stair edges, and at
a corner it juts out into the air on the far side. Proper decal projection
needs depth reconstruction.

**What the shipped games do.** Not quads. A decal is drawn as a *box volume*:
for each pixel inside the box the fragment shader rebuilds the world position
from the depth buffer, takes it into the box's local space, discards anything
outside the unit cube, and reads the blood texture from what is left. The
stain is then painted by the surface receiving it rather than by geometry of
its own, so it wraps a corner, stops dead at a table edge, and cannot hang in
the air — with no surface there are no pixels to shade.

Two limits, and both apply here:

- **It does not suit skinned characters.** The depth buffer knows where a
  body is this frame, not which bone a stain should follow. Shipped games
  blend wounds into the character's own material instead, or skip them.
  Parenting the stain to the bone, which `spawn_blood_stain_on_zombie` already
  does, is the right answer for a body.
- **It needs the depth buffer readable mid-pass.** The Metal renderer here can
  do that, but it is a new pass, not an edit to `blood.rs`.

**A cheaper half**, if the full thing is not wanted yet: raycast the quad's
four corners, and shrink or drop the stain when a corner finds no surface.
That ends the jutting without any shader work. It does not wrap a corner —
the stain just gets smaller near one.

**Already fixed, and not this.** Stains used to hang in mid-air, which looked
like the same fault and was not. An unparented stain is a world point, so
leaving one on anything that moves strands it the moment that thing moves on.
The ray that places it was hitting the ragdoll: death throws fifteen loose
capsules out across the path of the spray from the shot that caused it, which
is why the stains appeared on kills and hardly ever otherwise. Severed limbs
and spent casings did it more quietly.

The first attempt excluded the entity that was shot, on the theory that
`cast_ray`'s `solid` flag was answering with the victim's own chest. It was
not: a living zombie carries no collider at all — a shot is measured against
capsules cast at the bone transforms in `hitbox.rs`, never entered into the
physics world — so the exclusion named something the ray could not have hit.
The fix that works is a layer mask. Ragdolls, limbs and casings each carry
their own `GameLayer`, so restricting blood to `Default` covers all three.

## Rules of thumb

- Write plain WGSL. Reach for a directive only when there is no alternative.
- Prefer a UI material over a 3D material when the effect is screen-space.
- Test on both maps. The garage is dark enough to hide effects that look fine
  on the desert map, and vice versa — a lit-decal change was wrongly rejected
  once because it was only checked in the garage.
