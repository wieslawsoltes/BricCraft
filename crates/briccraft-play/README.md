# BricCraft gameplay rules

`briccraft-play` contains construction prices, an atomic stud wallet, and persistent
Expedition objectives. It depends on the parts catalog and Serde; it has no world,
renderer, browser, or UI dependency. `briccraft-game` measures successful player
actions and uses these rules for both classic islands and streamed worlds.

## Modes and persistence

Creative is the default. Construction is unlimited and objective progress pauses.
Expedition starts with **1,200 studs** and keeps its balance, completed objectives,
and travel progress when the player changes modes or saves and loads a world.
Enabling Expedition again does not grant another starting balance. Older saves
without gameplay data load in Creative.

Creative can intentionally assist an Expedition: parts built freely in Creative
can later be recycled. This is a local sandbox, with no competitive or anti-cheat
claim. The guarantees are atomic accounting, one-time grants and rewards, and
correct costs through undo and redo. Mode changes clear construction history so
commands cannot be replayed across different pricing rules.

Map and model imports use Creative. An import into an Expedition is rejected with
an instruction to change mode; imported terrain is never silently charged.

## Prices and transactions

Every catalog part costs at least one stud. Its base price is its bounding volume
in stud × plate × stud cells divided by three, rounded up. Mechanical and specialty
parts add catalog premiums: wheels 10, gears 12, axles 3, beams 4, propellers 30, and
windows or doors 4 studs. Power-category parts instead add 90 for motors, 70 for
batteries, or 18 for other power components. Rotation and color do not affect price.
A blueprint costs the sum of its actual parts. The starter rover contains 23 parts
and costs **377 studs**.

Recycling refunds a part's full catalog value. Undoing a purchase returns its cost;
redoing purchases it again. Undoing recycling must repay the refund and is refused
if the player has spent those studs. Parking or undoing a vehicle transformation
has no construction cost because its parts are unchanged.

The host transaction sequence is explicit:

1. Compute `ChangeValue { added, removed }` from the exact part IDs.
2. Call `Gameplay::approve` without mutating the balance.
3. Atomically apply the world edit, including global stream collision checks.
4. On success, consume the single-use `ResourcePermit` with `Gameplay::commit`.
5. Record the actual successful action with `Gameplay::record`.

Failed placement, insufficient balance, collision, or failed global capture changes
neither the wallet nor objective progress. A permit is not cloneable or serializable;
mode or wallet changes invalidate previously issued permits. Hosts must serialize
world edits and wallet commits rather than mutate gameplay between these steps.

## Objectives

| Objective | Actual action required | One-time reward |
| --- | --- | ---: |
| Gather supplies | Recycle 8 parts | 140 studs |
| Make your mark | Place 24 parts, including blueprint parts | 180 studs |
| Find your footing | Walk 120 studs | 120 studs |
| Establish a base | Complete a house, lighthouse, windmill, or bridge blueprint | 220 studs |
| Bring it to life | Activate a functional connected vehicle | 260 studs |
| Follow the trail | Drive or sail 240 studs | 320 studs |
| Take to the sky | Travel 160 studs in a powered, airborne airplane | 420 studs |
| A world of possibilities | Travel 1,000 studs across all tracked modes | 500 studs |

Rewards are granted automatically once and persisted as completed objectives.
Progress records successful actions and remains after undo; history replay itself
does not record new actions or issue rewards. A finished objective cannot be farmed
by repeated activation, undo/redo, mode toggles, or save/load.

The game measures movement inside each fixed simulation step. Walking uses actual
horizontal character displacement; vehicle travel uses actual physics movement;
flight requires an airplane to be above the local terrain. Free-camera flight,
orbit movement, focus changes, and floating-origin rebases do not count. The rule
layer rejects nonfinite, negative, or teleport-sized distance samples.

`ObjectiveView`, `BuildQuote`, `Gameplay::guidance`, and the game's shared workshop
assessment provide UI-ready information. Workshop readiness comes from the same
connected-assembly and power/running-gear checks used for actual activation.

## Validation

Persistent state validates its schema, balance, counters, and objective mask before
the game replaces a loaded world. Balances and counters are bounded. Tests cover
failed edits and global capture, refund repayment, one-time rewards, repeated
history, mode changes, legacy saves, actual walking/driving/takeoff, and persistence.
