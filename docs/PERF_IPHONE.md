# Phone performance

What was done to the renderer for the phone, what it measured out at, and what
is left. The phone is the constraint that matters: it is fragment-bound at
120 Hz and it gets hot, so every item here is weighed in milliseconds of
fragment time or in bytes read per vertex, not in draw calls.

Test conditions, because they change the answer: **100% resolution** and
**120 Hz**. At 100% there is one command buffer for the scene, which is what
makes the encoder breakdown below readable. Anything measured at a lower
resolution scale is measuring a different renderer.

`RESOLUTION_SCALE_DEFAULT` is 1.0 (`crates/client/src/graphics_settings.rs`)
so the default matches how the game is actually judged. `MENU_FPS_CAP` is 30
(`crates/client/src/cadisplaylink/native_menu.rs`) — the menu has no reason to
run at 120.

## What the phone actually spends

Measured again after the two fixes below, same viewpoint, HUD off:

| | before | after |
|---|---|---|
| scene pass | 3.281 | 3.323 |
| sun shadow | 0.427 | 0.463 |
| point shadow | 0.551 | gone |
| system compositor | 4.101 | 2.132 |
| **fragment total** | **8.361** | **5.920** |

The 120 Hz budget is 8.333 ms, so the frame went from 100.3% of it to 71.0%.
The app now submits exactly two render command buffers a frame, both at
1.00/frame: the sun shadow and the scene. Two findings got it there.

### The point shadow pass ran for a light that does not exist

`FALLBACK_LIGHT.range` is 4000. `LightUniform::from_snapshots` substitutes
that fallback whenever the snapshot carries no point light -- which is the
normal state, because the only `PointLight` in the game is the muzzle flash
(0.05 s lifetime) and `desert_city` has none of its own. So the pass's guard,
`light.range > 0.0`, was true on every frame, and the pass built a 90 degree
frustum 4000 units deep from [400, 800, 400], kept essentially the whole map
through frustum culling, and drew it into a 2048 square depth map.

The muzzle flash itself declares `shadow_maps_enabled: false`. The renderer
ignored it -- the flag was not even in `LightSnapshot`.

Cost: 0.892 ms/frame of GPU time, 0.551 of it fragment. The pass is now gated
on a real, shadow-casting, visible point light, so it runs while the trigger
is held and not otherwise.

Two things that had to come with it. The shader samples the point shadow map
whenever `atten > 0`, and with the fallback's range of 4000 that is nearly
every fragment in the map -- so skipping the pass without more would leave it
reading the last frame that did run. The map is now cleared to depth 1.0 once
per transition out of casting, which `LessEqual` compare reads back as lit.
And the far plane was `light.range.max(SHADOW_DISTANCE * 2.0)`, never below 80
for a muzzle flash that lights 3.2 units; it is now the light's own range.

### The Metal layer was not opaque

`MetalView.swift` set `backgroundColor` to clear and never said the layer was
opaque, so the compositor had to blend the full drawable against what was
behind it rather than copy it. In the trace that was a per-frame
`Read Surface: 404 -> Write Surface: 401/402/403` costing 1.319 ms/frame,
plus 1.037 ms/frame of compositor compute.

With `isOpaque` set on the view and `set_opaque(true)` on the layer in the
renderer, those became a plain `Write Surface` at 0.398 ms/frame and the
compute dropped to 0.476. Compositor fragment time fell 4.101 -> 2.132.

Nothing renders wrong: the scene pass clears to alpha 1 across the whole
drawable, so there was never anything behind the layer to show through.

iPhone 14 Pro Max, Metal System Trace, 21 s at a viewpoint chosen because it
visibly drops frames. Fragment time per frame, HUD off:

| encoder | ms/frame |
|---|---|
| scene (`Command Buffer 2:Render Command 0`) | 3.281 |
| system compositor (five surface blits) | 4.099 |
| `Command Buffer 1` / `Command Buffer 0` | 0.978 |
| **total** | **8.358** |

The 120 Hz budget is 8.333 ms. Fragment is at 100.3% of it — the frame is
fragment-bound, and it is bound with **half the cost outside the scene pass**.
The compositor blits are iOS composing our layer, not our shaders.

The lesson that keeps mattering: optimising the scene pass alone cannot get
this frame under budget, because the scene pass is only 39% of the fragment
time. Layer count and layer size are the other lever.

GPU occupancy over the same trace, same viewpoint:

| | Fragment | Vertex |
|---|---|---|
| app | 47.9% | 30.6% |
| system | 38.3% | 16.2% |

With the Metal HUD **on**, app Fragment goes to 74.5% and app Vertex to 41.0%,
and the app's fragment interval count doubles (7516 → 14741). The HUD is for
counting encoders, never for timing. `docs/METAL_HUD.md` says the same thing.

Vertex load tracks open sightlines: it is highest exactly where the map is
long and wide, which is the shape of a renderer with no occlusion beyond the
tiler's own Hidden Surface Removal.

**The view-angle lag predates this work.** It was bisected back past the first
of these commits and reproduces on the parent of the whole series, so none of
the changes below caused it. It is still unexplained.

## What was changed

In `metal_renderer` (branch `perf/macos-rigs`):

| commit | what |
|---|---|
| `fe62cbc` | keep the draw list between frames instead of rebuilding it |
| `947c7f3` | count references per asset instead of sweeping on a despawn |
| `fb8cba7` | keep an index of the transparent slots |
| `e94da36` | delete the optic pass |
| `8b7e1fe` | cut per-draw encoding, per-pixel work and geometry bandwidth |
| `63c5c3f` | switches for the parts of the opaque pass that scale with the view |
| `f41235a` | keep the tangent out of the interpolants that have no use for it |

The one with the most in it is `8b7e1fe`. Three separate things:

- **Vertex 56 → 40 bytes.** Normal and tangent became `snorm16` pairs, uv2
  became `unorm16`. Verified both sides agree on the layout
  (`size=40 normal@12 uv@20 tangent@28 uv2@36` in Rust and in MSL) and that
  the worst-case normal error is 0.00149°. Note `struct Vertex` is declared
  **twice** — the shadow shader reads the same buffer — so a change to one
  without the other is a silent stride mismatch.
- **Pass/mesh uniform split**, so the per-draw `setVertexBytes` carries only
  what varies per draw and stays under the 4 KB limit.
- **Incremental opaque ordering.** The first version rebuilt the order on
  every mesh update and cost 126 µs/frame during combat, which was worse than
  what it replaced. It now appends and `swap_remove`s, regrouping only once 64
  are pending.

In this repo, `f94f361` made the `Alpha` material in `desert_city` opaque —
it never had anything but opaque texels. Blended primitives went 217 → 18.
Blending is fragment work that the tiler cannot discard, so this is the
cheapest fragment win in the series.

### Switches for A/B on device

`63c5c3f` added three env toggles so each view-scaling part of the opaque pass
can be turned off on the phone without a rebuild:

```
NO_FLAT_NORMAL_PIPELINE   NO_TEXTURE_BIND_GUARD   NO_OPAQUE_SORT
```

These exist because the whole series was once sitting uncommitted as 151 hunks
in one file, which made a bisect impossible. Commit the perf work in pieces.

## What is left, in the order the measurements put it

Ranked by the heat section below, not by guesswork. The first two attack the
CPU, which is where the power goes; the third is the GPU's own limiter.

**1. Cut the draw count** -- merge meshes by material and spatial cluster.
Over 15% of all cycles sit in Metal's draw encoding (`AGX::*`,
`*ResourceListAddHashValue`), on two P-cores that never go idle. This is the
single biggest measured lever on heat. Not started.

**2. Work out why the main thread never blocks.** Fragment time is 5.920 ms
against an 8.333 ms budget and the GPU stays at its Minimum performance state,
yet Main Thread and Compute Task Pool are both 100% Running. The CPU, not the
display, is pacing the frame. Anything the loop does every frame that it does
not have to do is heat for nothing. Start at `ca-client-buffer-wait-interval`:
a frame loop that paces properly should spend time there.

**3. Move fragment math to F16.** The GPU is ALU-bound (limiter 60%) and does
that arithmetic in F32 -- F16 utilization is 0.62%. This buys fragment budget;
it does less for heat than 1 and 2, because the GPU never leaves its minimum
clock.

**Re-encode the gore textures** -- *demoted*. They do carry exactly one mip
level, so distant samples fetch at full resolution, and that was written up
here as the largest remaining fragment cost. The counters disagree: Texture
Cache Limiter is 8.89% and read bandwidth 2.38 against a 28.0 peak. Worth
fixing as asset hygiene, not as a performance measure.

**Split the skinned uniforms** the way the static ones were split in `8b7e1fe`.
Measured as marginal at 10 zombies; revisit if the zombie count goes up.

## Where the heat comes from

Measured with a thermoelectric cooler on the phone, so nothing here is a
throttled number: `device-thermal-state-intervals` reads **Nominal** for the
whole run. This is un-throttled steady demand.

**The GPU is not what ramps power.** `gpu-performance-state-intervals` says the
A16 GPU sat at its **Minimum** performance state for 100% of the run -- and
100% before the fixes too (17.41 s of 17.41 s in the pre-fix trace). The GPU
never clocks up for this workload, so its contribution to heat is not a clock
ramp. Read the GPU counters in that light: ALU Limiter at 60% is 60% of the
*minimum-clock* GPU.

**Both performance cores run continuously.** From a `CPU Profiler` trace:

| core | share of the game's cycles |
|---|---|
| CPU 4 (P) | 53.5% |
| CPU 5 (P) | 44.7% |
| the four E cores, together | 1.7% |

And neither busy thread ever waits: Main Thread is **100% Running** (1 Blocked
sample in 2525), Compute Task Pool **100% Running** (2 in 4057). The sampler
does record blocked threads -- IO Task Pool shows 0% Running -- so this is real
and not an artifact of only sampling on-CPU threads.

Two A16 P-cores held at full tilt with no idle is the expensive placement, and
it is the best explanation for the heat that the data supports.

The largest identifiable slice of that CPU work is **Metal's own draw
encoding**, not game logic. By leaf symbol:

| share of cycles | symbol |
|---|---|
| 6.8% | `AGX::RenderContext::encodeISPPrimitiveType` |
| 2.9% | `AGX::RenderUSCStateLoader::extractProgramVariant` |
| 2.1% | `NativeMetalRenderer::render_frame` |
| 1.8% | `AGX::ESLStateLoadEncoderGen1::emitUscState` |
| 1.6% | `bevy_ecs QueryParIter::for_each_init` |
| 1.6% | `NativeMetalRenderer::draw_scene` |
| 1.4% / 1.1% | `ioGPUResourceListAddHashValue` / `MTLResourceListAddHashValue` |
| 3.6% / 1.0% | `avian3d propagate_collider_transforms_recursive` / `parry3d TriMesh::aabb` |

The `AGX::*` and `*ResourceListAddHashValue` frames together are over 15% of
all cycles: that is the CPU price of issuing draws. Physics is the Compute Task
Pool's share.

This reorders the remaining work. **Cutting draw count attacks the measured
heat source; re-encoding the gore textures does not.**

### The GPU counters, and what they rule out

From `Metal GPU Counters` (means over 15 s; every percentage counter declares a
range of 100, and the observed maxima stay under it):

| counter | mean | max |
|---|---|---|
| ALU Limiter | 60.12% | 99.25% |
| Fragment Occupancy | 56.55% | 94.04% |
| ALU Utilization | 50.28% | 82.12% |
| F32 Utilization | 38.94% | 62.32% |
| Texture Filtering Limiter | 34.32% | 66.54% |
| Texture Sample Limiter | 33.44% | 65.84% |
| GPU Last Level Cache Limiter | 12.46% | 54.55% |
| Texture Cache Limiter | 8.89% | 21.79% |
| GPU Read / Write Bandwidth | 2.38 / 1.85 | 28.0 / 15.4 |
| F16 Utilization | 0.62% | 21.03% |
| Partial Renders Count | 0 | 0 |

The GPU is **ALU-bound**, and it does that arithmetic almost entirely in F32 --
F16 utilization is 0.62%. Half precision where the shader can take it is the
lever on the GPU side.

What this **rules out**: memory traffic. Texture Cache Limiter is 8.89% and
last-level cache 12.46%, read bandwidth 2.38 against a 28.0 peak. The earlier
guess in this document -- that the single-mip gore textures were the largest
remaining fragment cost -- is not what the counters say. Texture *filtering*
(34%) outranks texture *cache* (8.9%), so the sampler's work costs more than
its misses. `Partial Renders Count` of 0 also means nothing is spilling tile
memory.

### What could not be measured from the terminal

- **Per-line shader costs.** `gpu-shader-profiler-sample` exports, but each
  sample is a bare shader-core PC with no process id, and the shader address
  ranges collide: 121 range pairs overlap between this app and other
  processes, and the game's `ui_fragment` scored the exact share the system's
  `tile_simd_blur_5` did in an unfiltered pass. Attribution by address range is
  therefore not trustworthy. The real path is the `.gputrace` in Xcode.
- **Getting that `.gputrace` off the device.** `MetalCaptureEnabled` is now in
  `Info.plist`, so `GFX_CAPTURE=<n>` works and writes the bundle -- but
  `devicectl device copy from` fails on it with POSIX 62 (ELOOP): the bundle
  holds symlinks and devicectl opens with `O_NOFOLLOW`. Use Xcode's Devices and
  Simulators window, Download Container, instead.
- **L1D cache misses.** The `CPU Counters` instrument does advertise an
  `l1d_miss_sampling` counting mode on this device. But recording the stock
  template through `xctrace` leaves the counter columns as `<sentinel/>` --
  no PMU events get configured. Configuring events appears to need the
  Instruments GUI once, saved as a custom template that `xctrace --template`
  can then take by path.

## Measuring again

**A reinstall wipes the app's persisted graphics settings.** Two traces were
lost to this: the Metal HUD came back on by itself, and shadow quality came
back as 0 -- which skips both shadow passes, so a trace recorded in that state
cannot say anything about either of them. Check `[config] shadow map Npx
(quality N)` in the app's own log before trusting a shadow measurement.

**The HUD costs more than whatever is being measured and moves between runs.**
It was 3.701 ms/frame of fragment in one trace and 5.089 in the next, because
a different set of elements was enabled. It also inserts its own command
buffers, which renumbers everyone else's -- so per-buffer attribution stops
being trustworthy with the HUD on. Record with it off, or compare only
HUD-on against HUD-on.

**Launching from the home screen does not pick up `MTL_HUD_ENABLED=0`** --
only a `devicectl ... --environment-variables` launch does, and that restarts
the app, which loses wherever the camera was.

Use `profile.sh` from `desert_swift_metal` — it records an Instruments trace
and reads nothing off the screen. Two things that cost a trace each:

- It needs `PROC_NAME=bevy_in_iOS`.
- `devicectl --console` plus a `kill` sends SIGTERM to the app and destroys
  the recording. Stop it the way the script does.

Run with `MAP_ID=1`; without it the build sits in the menu, which is capped at
30 and measures nothing.
