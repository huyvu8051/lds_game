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

**Taking the weapon off the screen** (`hide_weapon_through_scope`, removed).
One number -- `aim_fov_multiplier` -- deciding something that is really a
judgement about which parts are in the way. It took the rifle, both arms and
whatever was on the rail, and it took them in one frame, which reads as the
gun vanishing rather than as looking down a tube. It also never worked on
this renderer: the path draws the arms from a query that reads no
`Visibility` at all.

**Cutting the sight in two and hiding the far half** (`ads_hide`, still in
the tree). Better, because the asset says what goes rather than a formula.
Still an on/off: the scope is there, then it is not.

## What works

Draw the world. Stamp a **depth-only disc** at the eyepiece glass. Then draw
everything on the weapon's layer.

Inside the circle the weapon loses the depth test and the world that is
already in the colour buffer stays. Outside the circle it draws exactly as it
always did, so the gun is still a solid object when it is looked *at* rather
than *through*.

One extra draw of 32 triangles. No second pass, no stencil, no camera that
has to be kept in agreement with the first one, and no asset work at all --
the sight ships whole.

The step that is *not* needed, and this is the whole saving: re-drawing the
world inside the circle. With one camera and the magnification applied to its
FOV, the world inside the circle is the same image, pixel for pixel, as the
world already drawn. Only the weapon has to be kept out of it.

It is also correct from any angle without being told the player is aiming:
the disc is a real piece of geometry, so a line of sight that passes through
the opening sees the world and one that does not sees tube. That is what an
actual tube does, and it is why there is no threshold anywhere in this -- a
threshold is what made the arms pop out of existence half way through the
ADS raise, back when they were simply not sent.

## The black a scope shows

A real scope goes dark when the head comes off the eyepiece: the picture
slides across the opening and the rim swells into a crescent, and at a hard
enough swing there is no picture left at all. Two circles in one plane give
that for free.

| Circle | Centred on | Radius |
|---|---|---|
| the opening | the glass, which rides the weapon | `window_radius` |
| the picture | straight ahead of the eye, down the bore | `picture_radius` |

They are concentric only while the eye is on the tube's axis. Sway slides
one across the other, and the gap between them is the black.

Four draws, each leaning on the one before through the depth buffer, in this
order:

1. **the picture**, into depth alone, 2mm nearer the eye
2. **the mark**, depth compare `Greater` -- kept to what the picture covers
3. **the opening**, painted black, depth compare `Less` -- which fails over
   the picture and covers everything else in the circle
4. **the opening again**, into depth alone, which shuts the weapon out of
   all of it

A fifth draw softens the rim: a band of black laid just inside the lens,
opaque at the edge and clear a few millimetres in. The depth test that cuts
the black is all or nothing, so without it the picture ends on a hard line.
Blended, and it writes no depth -- it sits inside the opening, where the
weapon loses anyway. `picture_feather` is how deep it reaches.

Step 3 is the one that does the work, and it is why nothing has to be
clamped: the black covers whatever the picture does not, wherever the
picture happens to be, including nowhere.

**Step 1 has to be clipped to the opening.** This is the trap. It writes
depth, and depth is what the weapon is tested against, so an unclipped
picture stamps its circle across the tube wall as well -- the wall then
loses the test and the world shows through the side of the scope. The clip
is the lens where the two circles overlap: sample each circle, keep the
points that fall inside the other, add the two places the rims cross, sort
by angle and fan. The intersection of two discs is convex, so sorting is a
polygon. It comes out empty once they come apart, which is the whole
opening going black.

The first attempt built a ring between the two circles instead, which is
simpler and wrong: the inner circle has to stay inside the outer one or the
ring folds through itself, so it had to be clamped -- and the clamp is a
limit on how dark it can ever get, which is the opposite of the point.

## The pieces

| Where | What |
|---|---|
| `sight_mount.rs` | `window_radius`, `window_forward`, `picture_radius`, `picture_feather` |
| `cadisplaylink/native_game_extract.rs` | works out the disc, flags the arms |
| `metal_renderer/snapshot.rs` | `ScopeWindow`, `SkinnedRenderState::viewmodel` |
| `metal_renderer/native_metal_renderer.rs` | `stamp_scope_disc`, `scope_picture_lens`, `draw_scope_shadow`, the late lists |

Two numbers per sight, and their absence is meaningful: a sight with no
`window_radius` gets no disc, and the weapon then draws with everything else,
which is the picture before any of this. Only `scope_m24` has them today.

`window_forward` is the one that is easy to get wrong -- see below.

## Doing it for the next scope

1. **Lift the sight off the gun** if it is still welded to one, and run it
   through the extractor: `tools/lift_node.py` then `tools/extract_optic.py`.
   `tools/harvest_sights.sh` has a row per sight showing how each was done.

2. **Read the eyepiece off the extractor's own line.** `THONGSO dai=320
   cao=64 rong=59 truc=30 mat=203 vat=117` means the eye end is at
   y = -203mm, the objective at +117mm, and the optical axis sits 30mm over
   the mount. Origin is the bottom of the mount foot, +Y towards the
   objective.

3. **Find where the eyepiece body ends.** Walk the tube's radius along the
   bore in Blender; the eyepiece is the stretch of narrow tube behind the
   first flare. On the M24 it runs -203 to -134, so `window_forward` is
   0.073: 73mm down the bore from `Socket_Eye`.

4. **Measure the opening.** `window_radius` is the mouth the eye looks
   through. The M24's tube waist reads 11mm and its eyepiece mouth 18mm, and
   15mm is between them -- a starting point, not a measurement. Smaller is a
   stricter hole, larger clears more tube.

   `picture_radius` goes under it by the width of the rim the scope should
   show at rest: 12.5mm against the M24's 15mm. Closer to `window_radius`
   and the black only appears on a hard swing; further under and there is a
   permanent dark ring. Zero turns the shadow off.

   `picture_feather` is how far the black fades in from that rim -- 3mm on
   the M24, a fifth of the opening. Zero leaves a hard edge.

5. **Run it and look.** The desktop binary draws through `metal_renderer`
   too, so this does not need a phone:

   ```
   MAP_ID=1 WEAPON=3 SIGHT=13 NO_AUTOPILOT=1 cargo run -p client --bin native_desktop --release
   ```

Both numbers are a rebuild, not an asset repack, since they live in the
`SightDef`.

## What bit us

- **The disc belongs at the glass, not at `Socket_Eye`.** The socket is the
  rear mouth of the tube, and everything *in front of* the disc is what the
  circle takes away -- so a disc left at the socket ate the eyepiece housing
  itself, and looking through the scope the tube behind the glass was simply
  missing. That is what `window_forward` exists for.

- **Anything that must line up with the picture has to be read where the
  meshes are read.** The disc was first worked out in a system, which runs in
  `Update`, so it saw transforms propagated at the end of the frame before
  while the weapon's own mesh was read in the extract with this frame's. The
  disc trailed the gun by one frame, which leaning or jumping made obvious.
  It is computed in `extract_render_snapshot` now, and cannot disagree.

- **The arms are a skinned actor**, drawn in a pass of their own, so they are
  not covered by holding static meshes back. They carry `viewmodel: true` on
  `SkinnedRenderState` and are drawn after the disc by the same closure the
  main pass uses. Hiding them with a threshold instead -- the first attempt
  -- popped them out of existence half way through the raise.

- **The reticle is clipped to the circle, not masked by it** -- the one
  thing on screen that works the opposite way round, because it belongs to
  the picture rather than standing in front of it. It is drawn straight
  after the disc with depth compare `Greater` and no write: inside the
  circle the nearest stored depth is the disc, so the mark passes; outside,
  it is the world, which is further than the mark, so the mark is cut. It
  has `RETICLE_LAYER` of its own so the extract can tell it apart from the
  weapon.

- **A fitted sight has to override the weapon's own aperture.** `holo` on a
  WeaponDef is the opening of the optic that gun was drawn wearing, and the
  AK still carries one although that optic is a fitted sight now. With an
  M24 scope on an AK the mark was tested against the AK's own 18mm window,
  which the eye is nowhere near -- so the dot was gone until recoil swung
  the gun through it, and then it flashed. A fitted sight means the sights
  being up is the whole test.

- **The decision belongs in the draw loop, not on the mesh.** Extraction is
  incremental: fitting a sight changes none of the weapon's meshes, so a flag
  written at extract time would still say "draw me early" on every one of
  them.

## Still open

- **Desktop through `bevy_render`** does none of this. That path has its own
  phases, and the disc would have to go into `Transparent3d` with the weapon
  behind it on `depth_bias`, since Bevy 0.19 does not expose stencil to a
  standard material. Nothing in the game runs that path today -- the desktop
  binary is `native_desktop`, which is this renderer.

- **The iOS build does not link.** `cpal` wants `_AVAudioSession*` and
  nothing pulls in AVFoundation. One line next to the `c++` and `z` lines in
  `crates/client/build.rs` fixes it; it has not been done.

- **`HideWhileSighted` and `ads_hide` are inert.** No sight carries that node
  any more. Kept because it costs a name comparison, and it is the only thing
  the windowed Bevy path could use.

- **Only `scope_m24` has a window.** The 6x and 8x are the obvious next two:
  both are long enough that the tube fills the frame, and both already come
  out of the extractor as a single clean mesh. Two numbers each.
