# Attachments

A plan for mounting sights, muzzles, grips, magazines and stocks on the
thirty-five weapons, PUBG-style. Nothing below is built yet.

## What already exists

- **`ItemKind` carries the whole attachment set** and no logic at all:
  six muzzles, six grips, three magazines, three stocks, and nine
  sights -- `RedDot`, `Holographic`, `Scope2x`, `Scope3x`, `Scope4x`,
  `Scope6x`, `Scope8x`, `Scope15x`, `CantedSight`. They drop, they are
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

## Open questions

1. **Weapons with no rail** -- Mosin, S686, R1895, Tommy Gun, and the
   VSS which takes a side mount. Leave them iron-only, or model a rail
   onto them?
2. **Weapons wearing a baked-in optic** -- the FAMAS's red dot, the AUG,
   the QBU, the AWM, the Lynx. Split the optic out as `part_sight` and
   treat it as a default that can be swapped, or strip it so the slot
   starts empty?
3. **Co-witness.** Hide the irons under an optic, or fold them?
4. **Magnification.** Is `aim_fov_multiplier` enough for an 8x and a
   15x, or does a scope camera have to come back?
5. **Variants and skins.** In scope now, or after the nine types work?
6. **Does an attachment change the numbers**, or only the look? A
   suppressor that does not change the sound, or an extended magazine
   that does not change `magazine`, is a model and not an attachment.
