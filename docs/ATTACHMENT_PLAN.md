# Attachments

A plan for mounting sights, muzzles, grips, magazines and stocks on the
thirty-five weapons, PUBG-style. Nothing below is built yet.

## What already exists

- **`ItemKind` carries the whole attachment set** and no logic at all:
  six muzzles, six grips, three magazines, three stocks, and nine
  sights -- `RedDot`, `Holographic`, `Scope2x`, `Scope3x`, `Scope4x`,
  `Scope6x`, `Scope15x`, `CantedSight`. They drop, they are
  carried, they do nothing.
- **Every weapon mesh carries six named sockets**: `Socket_Muzzle`,
  `Socket_Aim`, `Socket_SightFront`, `Socket_Eject`, `Socket_RearGrip`,
  `Socket_FrontGrip`. Of these the game reads only `Socket_Aim`,
  `Socket_Muzzle` and `Socket_Eject`; the two grips feed hand IK.
- **The magazine is already a separate node** on most weapons, found by
  the substring `magazine`, and `magazine_drop.rs` already hides it and
  leaves a physical one on the floor during a reload. That is the
  working proof that per-part show/hide is cheap.
- **Every weapon is in real-world scale**, checked against real
  hardware rather than against itself: flash hiders at 22mm, STANAG
  magazines at 25.4mm, a DP-28 pan at 267mm against the real 265.

## The question that decides everything else

> Do we rescale the guns so a scope mesh fits their rail?

**No.** The guns are already right, because they were scaled to real
dimensions. Measured, anchored on `Socket_Aim` and sampling the 120mm
of rail in front of it:

| weapon | rail top over bore | width measured | note |
|---|---|---|---|
| `scar_l` | 44.2mm | **20.5mm** | a Picatinny is 21.2mm across |
| `ace32` | — | **20.7mm** | same |
| `mk12` | 40.9mm | **14.9mm** | the rail's top flat, which is 15.6mm |
| `m16a4` | 42.5mm | 34.0mm | sample caught the receiver walls too |
| `g36c` | 41.8mm | 36.0mm | same |
| `mk47_mutant` | 87.1mm | 5.3mm | sample hit the red dot, not a rail |

Where the sample lands on an actual rail the width comes out at spec.
The wide readings are the measurement being sloppy, not the guns being
wrong -- which is itself the first task below.

So the work is **not** a global rescale. It is a **measured rail record
per weapon**, plus a flag for the ones whose top is not a Picatinny at
all: the AUG and the QBU wear a baked-in optic, the Groza has a carry
handle, and the Mosin, the S686 and the revolver have no rail to speak
of.

## Decisions taken

1. **Rails get built onto the guns that lack one.**
2. **The optics already on the guns get harvested and reused.**
3. **Irons are hidden under an optic**, not folded.
4. **`aim_fov_multiplier` carries the magnification.** No scope camera.
5. **Variants and skins come later.** Which mesh is which magnification
   is decided below.
6. **Attachments change the numbers, not just the look.**

## We own no attachment meshes

`zombie_assets` holds weapons, characters, maps and gore. There is no
scope, no muzzle device, no foregrip, no stock. **Every optic we have is
bolted to a gun**, which is why decision 2 is the whole supply.

### What is bolted to what

Read off the renders rather than a classifier -- a width test confuses
an aperture's protective ears with an optic.

| weapon | what it wears |
|---|---|
| `famas` | red dot, plus a suppressor |
| `mk47_mutant` | red dot |
| `vector` | holographic |
| `p90` | holographic, plus a suppressor |
| `m249` | holographic |
| `aug_a3` | short prism, roughly 1.5x |
| `svd` | PSO-pattern tube, **lenses already named** |
| `qbu` | mid-power tube |
| `awm` | long tube |
| `lynx_amr` | long tube |

`m16a4` is the useful control: its rail is empty and the measurement
finds nothing at the sight line at all, which is what an empty rail
should read.

### Harvesting them

Extracting a scope is the same job as aligning a gun, an order of
magnitude smaller: move the origin to the mount foot, add
`Socket_Mount`, `Socket_Eye` and `Socket_Objective`, name the lenses.

It also **solves a problem we already have**. The QBU's, the AWM's and
the Lynx's scopes are opaque in game because the geometric lens filter
cannot find a lens buried in a forty-thousand-triangle gun. Once the
scope is a small object on its own, the lens is obvious. Extraction is
the fix, not a detour around it.

One caveat, and one thing this plan got wrong. The caveat: none of these
were built to a shared mount standard, so each needs its own origin set.
The error: this said the AUG's and the QBU's optics grow out of a carry
handle and have no mount foot. Both turn out to sit on proper Picatinny
mounts -- the AUG on a single ring block, the QBU on a one-piece
cantilever carrying its own top rail -- which makes them the two easiest
harvests rather than the two that needed a foot modelled.

### What that covers, and what it does not

| `ItemKind` | taken from | note |
|---|---|---|
| `RedDot` | `famas` or `mk47_mutant` | two to choose between |
| `Scope2x` | `aug_a3` | short prism, the right length for 2x |
| `Scope3x` | `qbu` | mid tube |
| `Scope4x` | `svd` | **held back**, see below |
| `Scope6x` | `awm` | long tube |
| `Scope15x` | **nothing** | needs sourcing or modelling |
| `CantedSight` | **nothing** | small, 45-degree mount; probably modelled |

Seven of nine covered from what we already own.

### Done so far

Four are off and filed under `zombie_assets/attachments/scopes`:
`scope_2x` from the AUG at 206mm, `scope_3x` from the QBU at 224mm,
`scope_6x` from the AWM at 291mm.
Each carries `Socket_Mount` at the bottom of its foot, `Socket_Eye` and
`Socket_Objective` on the axis, and between fourteen and twenty-seven
blended lens discs.

The lens count is the point. The QBU's, the AWM's and the Lynx's scopes
are opaque in game today because the filter that finds a lens on a whole
gun also catches the tube around it. Off the gun, every one of them
found its lenses on the first pass.

`scope_4x` is held back. The SVD's PSO hangs on a Russian side rail
rather than a foot, the download bundles a separate illuminator
alongside it, and only one disc in the whole tube reads as a lens -- so
it would still be opaque. That one wants a person in Blender.

## Building the missing rails

A rail is **one mesh placed many times**, not one mesh per gun: a
Picatinny strip is 21.2mm wide with a 10.16mm slot pitch, and the only
per-gun work is choosing where it sits and how long it runs.

Needs one: `mosin_nagant`, `s686`, `r1895`, `tommy_gun`, `bizon`,
`skorpion`, `mini14`, `sks`, `micro_uzi`, `mp5k`, `p1911`, `dp28`,
`groza` (which has a carry handle instead), `vss` (which takes a side
mount in reality -- a top rail is the simpler lie).

The gun needs a flat to put it on. Where the receiver is round or
ribbed, the strip either floats or has to be sunk, and that is a
judgement per gun, not a rule.

## Which slots each weapon takes

Stats change, so the table is gameplay as much as geometry.

| class | muzzle | grip | magazine | stock | sight |
|---|---|---|---|---|---|
| assault rifle | yes | yes | yes | yes | yes |
| SMG | yes | yes | yes | yes | yes |
| marksman rifle | yes | yes | yes | yes | yes |
| bolt-action | yes | no | yes | yes | yes |
| machine gun | yes | yes | no | no | yes |
| shotgun | choke, duckbill | no | bullet loops | yes | yes |
| pistol | yes | no | yes | no | yes |

A slot a weapon does not have is not a greyed-out box: the gun simply
has no socket for it.

## Part groups

Five groups per weapon, each independently shown or hidden, plus a body
that is never hidden.

| group | node name | what it is | when hidden |
|---|---|---|---|
| iron sights | `part_sight` | the gun's own irons or integral optic | an optic is mounted and does not co-witness |
| muzzle | `part_muzzle` | flash hider, brake, the bare thread | a muzzle attachment replaces it |
| magazine | `magazine` | already split, already used | during a reload; swapped for an extended one |
| foregrip | `part_foregrip` | only where the gun ships one | a different grip is mounted |
| stock | `part_stock` | fixed, folding or none | a stock attachment replaces it |
| everything else | `body` | receiver, barrel, handguard | never |

`magazine` keeps its name: `wire_gun_mesh` already matches it by
substring and nothing should have to change there.

## Splitting the meshes: two tiers

**Ten weapons ship real part names** and cost almost nothing -- rename
and export:

`svd` (barrel, butt, rail, scope, lens), `mk12` (barrel, flash,
handguard, stock, rail), `g36c` (barrel, butt, flash, stock),
`tommy_gun`, `groza`, `mp5k`, `mosin_nagant`, `m16a4` (handguard,
stock), `awm` (scope), `aug_a3` (scope).

**Twenty-three are anonymous** -- a box per part, placed by eye in
Blender, exactly as the magazine boxes were. Three to four boxes each,
so **seventy to ninety boxes**, and that is the bulk of the work. It
cannot be automated: a box is a judgement about where a part ends.

`tools/export_gun.py` already splits by shard centre inside one box. It
needs to take several named boxes instead of one, and to keep the
groups as separate objects rather than welding everything back.

## Sockets to add

| socket | where | read by |
|---|---|---|
| `Socket_RailBase` | rear end of the top rail, **on the rail's top plane**, x = 0 | every sight |
| `Socket_RailUnder` | front of the under-rail, on its bottom plane | foregrips |
| `Socket_Stock` | where a stock bolts on | stocks |

`Socket_Muzzle` already sits at the bore at the muzzle face, which is
where a suppressor screws on, so muzzles need nothing new.

Alongside them a **rail record** per weapon, which is data rather than
a socket: top height over the bore, width, and the usable run along the
bore. A 15x will not fit a rail that is 90mm long, and the record is
what lets the game say so instead of clipping.

## Scope catalogue

Nine types, already named in `ItemKind`. One mesh each. **No variants
in the first pass** -- PUBG's variants are skins, and a skin is a
material swap on the same mesh, which is a later and much smaller job.

Each scope mesh follows its own convention, the same way weapons follow
theirs:

- origin at the **mount foot**, centred side to side
- `Socket_Mount` at the origin -- this lands on the gun's `Socket_RailBase`
- `Socket_Eye` at the centre of the eyepiece glass
- `Socket_Objective` at the centre of the front glass
- lens meshes named `lens_*`

That last one matters. The QBU's and the AWM's scopes are opaque in
game today precisely because their lenses carry no name and no material
of their own, and the geometric filter catches the whole tube instead.
Naming the lenses makes the glass rule trivial.

## Where the config lives

A scope's eye point relative to its own mount is a property of **the
scope**, not of the gun. So most of it is one global table:

```
ScopeDef {
    kind: ItemKind,          // RedDot .. Scope15x
    mesh: &str,
    magnification: f32,      // 1.0 for a red dot
    eye_offset: Vec3,        // Socket_Mount -> Socket_Eye
    objective_offset: Vec3,
    eye_relief: f32,
    reticle: ReticleKind,
}
```

What is genuinely per **(weapon, scope)** is small:

```
ScopeFit {
    rail_offset: f32,        // how far along the rail it sits
    hide_irons: bool,        // co-witness or not
}
```

with a default that works for most pairs. That keeps it a short
override list rather than a thirty-three by nine matrix.

## The part that touches what already works

`Socket_Aim` is the eye point today, and `gun_view.rs` derives the whole
ADS pose by subtracting it. With attachments the eye point has to come
from the mounted optic instead.

Proposal: `Socket_Aim` stays exactly what it is -- **the iron sight
point** -- and the game asks for an *effective* aim point: the irons
when nothing is mounted, otherwise `Socket_RailBase + rail_offset +
scope.eye_offset`. That is one indirection in one function, not a
rewrite, and it keeps every number already measured.

Magnification is the open one. There is no scope camera: the comment in
the AK's definition records that the optic camera was removed and "the
whole screen is the sight picture now". An 8x or a 15x through
`aim_fov_multiplier` alone may or may not be enough.

What a long scope needs on top of the magnification is to stop being in
the way of its own picture. That is done, for the M24, by cutting the
sight in two at the eyepiece and drawing the far half after the world
against a depth disc -- see `SCOPE_SEE_THROUGH.md`, which is the recipe
for every scope after it.

## Phases

1. **Audit.** Measure the rail on all thirty-three downloads and write
   the record. Flag the ones with no true rail. Half a day, tooling
   included; it is also what proves the no-rescale claim gun by gun.
2. **Convention and tooling.** Multi-box splitter, the three new
   sockets, the record file, the scope mesh convention written down
   next to `WEAPON_ALIGNMENT.md`.
3. **Pilot.** Split the ten named weapons end to end and prove
   show/hide in game on one of them -- the SVD, which already has a
   named scope and named lenses.
4. **Scope meshes.** Nine, to the convention. Sourcing is the risk:
   they must be real Picatinny mounts or none of the alignment holds.
5. **The remaining twenty-three.** Seventy-odd boxes. Grind.
6. **Game code.** Attachment slots on `Gun`, show/hide, the aim-point
   indirection, equip and unequip from the pack.

Phases 1 to 3 answer whether the whole idea holds together, on one
weapon, before phase 5's grind is committed to.

## Still open

- **Which of the duplicates to keep.** Two red dots and three
  holographics; the rest become spares or skins.
- **`Scope15x` and `CantedSight`** have no source at all, and
  `Scope4x` has one that needs hand work.
- **What each attachment actually does to the numbers.** Decision 6 says
  they change them; nobody has said by how much. That is a balance pass,
  and it wants its own table next to the weapon one.
- **Suppressors.** The FAMAS and the P90 each wear a clean detachable
  can, and both come off the same way a scope does. The VSS's is
  integral, with the front sight standing on it, so taking it would
  leave the gun without a barrel.
