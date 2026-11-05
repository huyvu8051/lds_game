# Active ragdoll

A plan for closing the distance between what a zombie does when it is shot
and what a NaturalMotion character does. Stages 0-3 are built; everything
under "What is missing" is not.

Zombies only, client only. PvP was cut from this plan deliberately -- the
decision was to get one body falling well before worrying about two.

## What already exists

- **A skeleton of rigid bodies.** `bone_links` builds fifteen bodies and
  fourteen joints off `ZombieHitboxBones`. Each capsule spans its bone
  rather than straddling the head, so the mass is where the limb is and the
  joint's twist axis is the limb axis.
- **Hinges where the body has hinges.** Knees and elbows are
  `RevoluteJoint`, pointed by `flex_axis` off the torso's left and limited
  asymmetrically: the joint may close to `FLEX_LIMIT_*` and open back to
  straight and no further. Hips, shoulders and the spine are
  `SphericalJoint` with a swing cone and a twist limit, centred on the pose
  the body died in.
- **Muscles.** `drive_muscles` writes a `ConstantTorque` per bone per frame,
  a PD servo tracking a ghost rig that is still playing the death clip. The
  corpse therefore keeps moving with the animation as it goes down instead
  of switching from one to the other. Avian's `AngularMotor` does not fit --
  it serves revolute and prismatic joints, not spherical.
- **Two behaviours, one signal.** `track_muscle_targets` reads how far past
  saving the fall is from the torso's tilt (`fighting`, one on its feet and
  nothing on its side) and which way it is going over. While there is
  balance left the arms throw back against the tip (`WINDMILL_RADIANS`);
  once there is none and it is dropping fast they turn over and reach for
  the floor (`BRACE_RADIANS`). Same axis, opposite sign.
- **Stiffness that changes.** Tone rises with `fighting` through
  `BRACE_GAIN`, and a joint a behaviour is actively steering keeps at least
  `REFLEX_FLOOR` whatever the rest of the body is doing.
- **The hit is placed.** The bone that was struck takes `HIT_BONE_SHARE`
  against the torso's one, so a head shot and a hip shot do not produce the
  same fall.
- **A budget.** `MAX_ACTIVE_RAGDOLLS` caps live bodies at four. A wave
  pushes zombies out in bursts, so corpses piling up is a matter of when,
  and the phone has no room for the pile.

## What is missing

Ranked by how much of the difference each one accounts for.

### 1. Stepping to save the fall

The signature of the thing. A NaturalMotion character shot while walking
takes one or two protective steps in the direction it was pushed before it
accepts that it is going down; ours goes over like a sack every time.

The machinery is already there and the legs are simply not wired into it:
`steers: matches!(link.joint, JointKind::Shoulder)` -- only shoulders take
the behaviour rotation. Hips take none.

What it needs: open `steers` to `Hip`, and give the legs a target that is
not the arms' target. The arms throw *across* the topple; a leg steps
*into* it, swinging the hip along `toppling` rather than along
`toppling x Y`. The step has to retract as `fighting` falls, so a body that
is genuinely going down stops trying.

Watch for: the compounding that cost the arms a day. The behaviour rotation
goes on one joint of the chain, never on all three.

### 2. Being knocked down without dying

`arm_ragdoll` only fires behind `DeathClipPlaying`. Ragdoll means death, so
there is no stumble-and-carry-on.

This is the expensive one, because it needs a road back: ragdoll to
animation, with the clip starting from the pose the body actually ended in.
Worth doing after 1 and 3, not before.

### 3. A shot leg that buckles

The hit currently adds impulse weighted onto the bone it struck and nothing
else. Shoot a knee and the knee does not give.

What it needs: a per-joint limpness that the hit sets and that decays --
the muscle's `tone` floored to near nothing on the joint below the wound
for a half second. Cheap, and it suits zombies, where a leg going out from
under one is most of the point.

### 4. Protecting the head

Euphoria holds the neck so the head does not slap the ground, and turns the
face toward whatever just hit it. We do neither; the head is dead weight on
a spine joint.

### 5. Clutching the wound

The hand goes to where the round landed. Named in the original plan, never
built. It is a third behaviour on the same `steers` mechanism, with the
target taken from the hit bone's position rather than from the topple.

### 6. Still moving after it lands

We settle, sleep, and freeze. A Euphoria body keeps breathing and rolls
slowly for a while. A low-amplitude target drift on the spine muscles after
`settled_for` would cover most of it.

### 7. Knowing the room

Reaching for a wall, catching a railing, tripping on a kerb. Needs a
spatial query from the hands and a target that is a found surface rather
than a computed direction. The largest of these in work, and the least
visible per hour.

### 8. Bracing at the actual floor

`BRACING_FALL_SPEED` triggers the brace on vertical speed alone. The hands
reach down, not at the place the body is about to hit.

## Order of work

1. **Stepping** (1), because it reuses what exists and changes the most.
2. **Buckling** (3) alongside it -- same hit path, same muscles, and the
   two read well together: a leg goes, the body steps on the other one.
3. **Head and wound** (4, 5), both third behaviours on the same mechanism.
4. **Ground motion** (6).
5. **Knockdown and recovery** (2), once there is something worth recovering
   from.
6. **Environment** (7) and **aimed bracing** (8) last.

## The bench

`crates/client/src/ragdoll_range.rs`, on the flat map (`MAP_ID=4`). Every
zombie is parked, one subject is stood on a mark and shot, and the joint
angles are logged while it falls. Built because judging this by eye in a
live wave was not working.

| Variable | What it does |
| --- | --- |
| `RAGDOLL_TEST=1` | Arms the bench. Required by the rest. |
| `RAGDOLL_TEST_BONE=head\|chest\|leg` | Where the burst goes. |
| `RAGDOLL_POSE=<joint>` | Bends one joint to its limit and holds it. No shooting, no solver. Stands the subject side-on, clear of the viewmodel. `knee`, `elbow`, `hip`, `hip_back`, `hip_out`, `shoulder`, `shoulder_back`, `shoulder_out`, `spine`, `twist`, `default`. |
| `RAGDOLL_POSE_DEG=<n>` | Overrides the angle, for asking where the mesh stops surviving its own limit. |
| `RAGDOLL_PIN_HINGES=1` | Pins every hinge to three degrees. A pinned joint must read the angle it armed with, so this is what settles an argument about whether a reading is honest. |

Build with `--profile dev-fast`. `bevy/dynamic_linking` does not work here:
`client` exports `staticlib` for iOS, and `bevy_dylib` wants an rlib.

Two things the bench taught, both the hard way:

- **A number that confirms itself proves nothing.** `flex_axis` both builds
  the hinge and signs the log, so a knee folding the wrong way logged as
  healthy flexion for three rounds of measurement. The picture caught it.
  Bend the joint, hold it, look.
- **Pin before reasoning.** A knee reading eighty degrees of hyperextension
  was the thermometer, not the joint, and one pinned run said so.

## Tried and dropped

Do not retry these without a reason that is not in here.

- **Centring the swing cone on anatomical rest.** Give the parent frame the
  rest direction and the child frame the real one, so the solver reads
  deviation from anatomy rather than from the accident of the clip. Worse,
  both ways round: a joint already outside its cone at death is yanked into
  it on the first step, so the body arrives through a snap rather than a
  fall. Knees at -114 against -48 for the pose-centred cone.
- **Deriving the hinge sign from the limb.** `axis x child_along` pointing
  forwards degenerates the moment a shin leaves vertical, which is every
  bent knee. The sign came down to noise.
- **Carrying the hinge axis into the log.** A hinge rides with its parent,
  so in principle the log should carry the axis rather than keep asking the
  torso. In practice it got the same frames wrong. The torso reading stays.

## Open

- The head shot settles into a tangle. It is not a limit violation -- the
  pinned run rules that out -- but whether it is a fall worth watching is a
  judgement nobody has made yet.
- `MAX_ACTIVE_RAGDOLLS = 4` is a guess. The figure to watch on the device
  is the solver's share of the frame, not the body count.
- Shoulder extension is about thirty degrees looser than a person's. It
  still reads as an arm behind a head, so it has been left.
