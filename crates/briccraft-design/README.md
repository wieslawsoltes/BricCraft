# Reusable construction designs

`briccraft-design` stores bounded construction recipes independently of live world
IDs, renderer data, and floating-origin coordinates. Each design retains its part
IDs, colors, individual quarter-turn rotations, and relative X/Y/Z cells. Saved
source-world metadata includes an exact integer origin for reference; placement
never reuses that source offset.

## Model and limits

A world library holds at most **32 designs**. Each design contains **1–256 parts**
inside a **32 × 72 × 32** envelope: X/Z are studs and Y is plates. One plate is 0.4
world units. Names contain 1–64 characters, are trimmed, reject control characters,
and are unique within the library without regard to letter case. IDs increase
monotonically and are not reused after deletion.

Validation checks every catalog ID, color, rotation, relative coordinate, bounding
dimension, overlap, and saved component count. A connected-assembly capture must
form one face-connected component, using the world's wheel/floor contact rule.
An explicitly remembered construction group may retain separate decorative parts;
its capture kind and component count describe that distinction accurately.
Unknown schema versions, duplicate identities, malformed metadata, inconsistent
geometry, and capacity violations reject the whole candidate library.

The rule layer depends only on `briccraft-parts`, `briccraft-world`, and Serde.
Pairwise bounded envelopes validate a design without allocating a render-world
occupancy volume. It contains no renderer, browser, UI, resource wallet, or clock.

## Capturing and placing

The game captures the pointed connected assembly when it fits the limits. A
remembered latest blueprint or design copy is an explicit fallback. Connected
terrain floods stop after 257 parts; oversized or partly loaded boundary assemblies
are rejected. Readiness is cached by scene, world revision, and selection so HUD
polling does not repeat the flood.

Only parked constructions can be captured. Imports must finish or be canceled
before library changes. Capture, rename, and deletion affect recipes only: they
do not remove placed parts, spend studs, or award objectives.

A design's `placements(origin, rotation)` uses the same positive-yaw quarter turns
as built-in blueprints. The origin is always the minimum corner of the rotated
envelope. Colors and each part's relative rotation remain intact.

In the game, Place uses the picked face; without a hit it uses the build focus or
a ground point eight studs ahead of the character. It never silently searches for
a different destination. `Session::place_design_at` exposes exact local cell
placement for editor tools and tests.

Copies go through the existing atomic construction transaction. Expedition charges
the sum of the actual constituent part prices; Creative remains unlimited. A
collision, invalid destination, insufficient balance, or failed global capture
rejects the whole copy. Undo/redo accounts for the whole purchase once. Successfully
placed parts count toward the existing building objective; saving or copying a
recipe introduces no recurring reward or special structure reward.

## Persistence and API

Per-world session saves include a defaulted `designs` library. Existing version 1
and version 2 worlds without that field load with an empty library. Candidate
design data is validated before replacing the live world. Relative recipes survive
world rebasing, save/load, renaming, and deletion of their original constructions.

Individual designs can also be exported as `.bricdesign` files and imported into
other worlds. The versioned `briccraft.design` document is bounded to 256 KiB and
validated before the destination library changes. Import preserves source metadata
and relative geometry, assigns a fresh destination ID, and adds a bounded `(2)`
name suffix when needed. It adds no placed geometry, charges no studs, and awards
no objectives. Copy placement subsequently uses the destination world's budget.

The game exposes `designs`, `design_capture_status`, `capture_design`,
`rename_design`, `delete_design`, `design_quote`, `place_design`, and
`place_design_at`, `export_design`, and `import_design`. Mutation methods return explicit errors; callers can show those
messages in their status UI. The eight built-in blueprints are unchanged.
