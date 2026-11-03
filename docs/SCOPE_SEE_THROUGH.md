# Looking through a scope, with one camera and one pass

How the M24's glass was made see-through, written down because every long
scope after it wants the same treatment.

## The problem

There is one camera. The magnification that blows up the world blows up the
viewmodel with it, so at 0.38 the tube fills the screen it is supposed to be
aiming through, and what the eye gets is the inside wall of a cylinder.

Three things were tried before the one that stayed.

**A second camera into a texture.** What PUBG and CoD do. It costs a whole
extra render pass, which on a tile-based GPU means flushing tile memory to
DRAM and loading it back. On the phone that is the most expensive thing a
frame can be asked to do, and this frame is already warm.

**Taking the weapon off the screen** (`hide_weapon_through_scope`, removed in
this branch). One number -- `aim_fov_multiplier` -- deciding something that is
really a judgement about which parts are in the way. It took the rifle, both
arms and whatever was on the rail, and it took them in one frame, which reads
as the gun vanishing rather than as looking down a tube.

**Hiding the tube by a plane or a radius** (`ads_hide`, still in the tree).
Better, because the asset says what goes rather than a formula. Still an
on/off: the scope is there, then it is not.

## What works

Split the sight in two at the eyepiece. Draw the world. Stamp a **depth-only
disc** at the eyepiece opening. Then draw the far half of the tube.

Inside the circle the far half loses the depth test, and the world that is
already in the colour buffer stays. Outside the circle it draws exactly as it
always did, so the scope is still a solid object when it is looked *at*
rather than *through*.

One extra draw of 32 triangles. No second pass, no stencil, no camera that
has to be kept in agreement with the first one.

The step that is *not* needed, and this is the whole saving: re-drawing the
world inside the circle. With one camera and the magnification applied to its
FOV, the world inside the circle is the same image, pixel for pixel, as the
world already drawn. Only the tube has to be kept out of it.

It is also correct from any angle without being told the player is aiming:
the disc is a real piece of geometry, so a line of sight that passes through
the opening sees the world and one that does not sees tube. That is what an
actual tube does.

## The pieces

| Where | What |
|---|---|
| `zombie_assets/tools/split_ads_hide.py` | cuts the sight into two nodes |
| `sight_mount.rs` | `ThroughScope`, `ScopeWindow`, `SightDef::window_radius` |
| `cadisplaylink/native_game_extract.rs` | flags the meshes, sends the disc |
| `metal_renderer/snapshot.rs` | `ExtractedMesh::through_scope`, `ScopeWindow` |
| `metal_renderer/native_metal_renderer.rs` | `through_slots`, `stamp_scope_disc` |

The node name is the contract. A node called **`scope_far`** is the half
drawn late; a node called **`ads_hide`** is the older behaviour and is taken
off the screen instead. A sight carries one or neither, never both -- hidden
geometry cannot also be drawn late.

`window_radius` on the `SightDef` is what the disc is cut to, and its absence
is meaningful: a sight without one gets no disc, and its `scope_far` half
then draws with everything else, which is the same picture as before any of
this.

## Doing it for the next scope

1. **Lift the sight off the gun** if it is still welded to one, and run it
   through the extractor. `tools/lift_node.py` then `tools/extract_optic.py`;
   `tools/harvest_sights.sh` has a row per sight showing how each was done.
   Keep the uncut result as `<name>_whole.glb` -- every re-cut below starts
   from it, and the extractor does not have to run again.

2. **Find the eyepiece in the sight's own frame.** The extractor prints it:
   `THONGSO dai=320 cao=64 rong=59 truc=30 mat=203 vat=117` means the eye end
   is at y = -203mm, the objective at +117mm, and the optical axis sits 30mm
   over the mount. Origin is the bottom of the mount foot, +Y towards the
   objective.

3. **Cut just in front of the eyepiece.** For the M24 the eyepiece body runs
   -203 to -134, so:

   ```
   blender -b --python tools/split_ads_hide.py -- \
     attachments/sights/scope_m24/scope_m24_whole.glb \
     attachments/sights/scope_m24/scope_m24.glb \
     keep=-130 side=eye name=scope_far
   ```

   Shards go by their centre, so rings do not get sliced; on the M24 exactly
   one shard of eight hundred straddled the plane. The tool prints how many
   faces went each way -- 1152 stay, 4574 are drawn late.

4. **Pack it**: `node tools/pack_for_game.js <name>`, which needs a row in
   that file pointing at the sight's glb.

5. **Give the `SightDef` a `window_radius`.** The M24's is 15mm: its tube
   waist reads 11mm and the eyepiece mouth 18mm, and the middle of those is a
   starting point, not a measurement. Smaller is a stricter hole, larger
   clears more tube.

6. **Run it and look.** The desktop binary draws through `metal_renderer` as
   well, so this does not need a phone:

   ```
   MAP_ID=1 WEAPON=3 SIGHT=13 NO_AUTOPILOT=1 cargo run -p client --bin native_desktop --release
   ```

Changing the cut or the radius is two commands and a restart -- no Rust
build. The assets are read once at load, so the game has to be restarted
rather than reloaded.

## What bit us

- **The split node must not be called `ads_hide`** if you want it drawn late.
  That name is wired to `HideWhileSighted`, which takes it off the screen, so
  the late draw would have nothing to draw.

- **The world has to be in the buffer before the tube.** Opaque draws are
  grouped by texture, not sorted by distance, but nothing guarantees the tube
  comes last -- so it is held out of that loop entirely and drawn after the
  blended pass, which is after everything else in the world.

- **`decorate` had to learn the entity.** The hook the host uses to attach
  lightmaps only saw the mesh, and which node is a sight's far half is a fact
  about the entity. It now takes both.

- **The disc faces down the bore, not at the camera.** It is geometry, not a
  billboard; a disc that turned to face the eye would stop being a hole in
  the tube and start being a sprite stuck to the screen.

- **Which end is which.** The first cut kept the objective end, because that
  is the end that was picked in Blender. Looking through the sight, the half
  that must survive is the one the eye is *behind*. `side=` exists because of
  that mistake.

## Still open

- **Desktop through `bevy_render`** does none of this. The windowed Bevy path
  has its own phases, and the disc would have to go into `Transparent3d` with
  the tube behind it on `depth_bias`, since Bevy 0.19 does not expose stencil
  to a standard material. Nothing in the game runs that path today.

- **The iOS build does not link.** `cpal` wants `_AVAudioSession*` and
  nothing pulls in AVFoundation. One line next to the `c++` and `z` lines in
  `crates/client/build.rs` fixes it; it has not been done.

- **Only `scope_m24` carries a `scope_far` node.** The 6x and 8x are the
  obvious next two: both are long enough that the tube fills the frame, and
  both already come out of the extractor as a single clean mesh.
