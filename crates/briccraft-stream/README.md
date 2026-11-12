# briccraft-stream

A reusable, renderer-independent world stream. Terrain exists in signed 64-bit X/Z stud coordinates;
Y remains a 72-plate slab, matching `briccraft-world`. The active GPU/editing window is 96 × 72 × 96 and
moves in 16-stud increments. This is horizontal open-world streaming with a bounded working set.

## Window lifecycle

```rust
use briccraft_stream::{BiomePreset, StreamWorld, WindowOrigin};

let mut stream = StreamWorld::new(42, BiomePreset::Meadow, 64)?;
stream.request_window(WindowOrigin::default())?;
// Call once per frame. Continue drawing the old complete World while pending.
let progress = stream.advance_generation(2)?;
if progress.complete {
    let snapshot = stream.snapshot(stream.requested_origin().unwrap())?;
    // Publish snapshot.world and invalidate renderer atlases together.
}
# Ok::<(), briccraft_stream::StreamError>(())
```

`GenerationBudget` additionally caps cell work. One generation unit is a complete 16 × 72 × 16 chunk;
budgets below that unit make no progress. Publication never exposes a partially generated window.
The LRU holds 36–256 chunks; active requested chunks are protected and obsolete chunks are evicted.
Chunk generation depends only on seed, preset, signed chunk coordinate, and generator version.
Integer lattice noise retains detail beyond the precision range of floating-point global positions.

`WorldSnapshot::rebase_around(camera_local)` returns a new aligned origin and a local translation
when the camera leaves the central 32..64-stud region. Capture local edits first, request the new
window, budget its generation, and publish the completed snapshot. Apply the same translation to
the camera, vehicle physics state, preview positions, and active assembly geometry. Global part
identities and exact global placement coordinates do not move.

The five presets provide flower meadows, rocky/snowy alpine terrain, cactus dunes, palm coastlines,
and a flat 18-plate import/building canvas. Sparse deterministic pavilions and oasis wells are made
from ordinary editable parts. The origin meadow reserves a 64 × 64 flat construction area.

## Construction and identity

`capture_edits(&mut snapshot, &current_world)` compares the local scene with its captured baseline,
then atomically commits a global change and updates local-ID mappings. Call it after a construction
transaction so the global undo stack retains individual edits. A repeated capture is a no-op.
`snapshot.part_key(local_id)` and `snapshot.local_id(key)` bridge render and persistent identities.

`place_parts`, `remove_parts`, and `move_parts` validate whole transactions against generated terrain
and sparse edits, including footprints crossing chunks. Moves preserve `PartKey`; parking a vehicle
can therefore move its original keys even after their source window has been evicted. After global
changes, refresh the current snapshot before capturing another local edit. A stale snapshot is an
explicit error instead of silently overwriting newer global changes.

Global `undo`/`redo` persists through rebases and cache eviction. History is bounded to 128 actions
and 250,000 changed parts. New edits discard redo. History is intentionally a session resource;
saving/loading begins a new undo session.

Procedural parts fit inside their owner chunks. Geographic source parts are indexed into every
intersected chunk, while retaining a single stable owner identity. A part that intersects the outer window boundary
is returned whole in `boundary_parts`, retained in the global edit store, and omitted from the strict
local occupancy grid until it fits. No part is split into different shapes or lost. The 32-stud camera
guard keeps these boundary fragments outside interaction range; renderers should fog/fade the outer
edge of the active volume. Supported catalog footprints are at most 32 studs wide.

## Large imports

`begin_import(Vec<GlobalPlacement>)` validates all IDs, dimensions, colors, rotations, coordinates,
and vertical ranges, then prepares an unpublished `ImportJob`. `advance_import` validates collision
chunks under the same chunk/cell budgets. Dropping the job cancels it without consuming IDs or
changing the world. `commit_import` publishes the entire validated plan as one global undo action.
Concurrent edits invalidate an in-progress job, so a stale import cannot overwrite them.

Per-transaction limits are 150,000 parts and 4,096 affected chunks. The sparse edit store is capped at
250,000 entries and serialized input at 64 MiB. Geometry outside Y=0..72 is rejected before mutation.
Import models can span many active windows; only visible chunks and instances enter GPU snapshots.

## Persistence

`to_json` writes seed, preset, generator version, exact window origin, next identity, and sparse edits.
Procedural terrain and cached chunks are regenerated. `from_json` validates duplicate identities,
catalog metadata, bounds, generated keys, and collision consistency before returning a world.
Deletion tombstones retain generated identities. Restoring a generated part to its original state
removes its override, so ordinary edits do not expand into full terrain saves.

`GlobalPos` stores exact global stud/plate integers. `WindowOrigin::global_position` is an approximate
display helper returning f64; use `global_cell` for persistent coordinates. `local_position` subtracts
the integer origin before converting to floats, preserving local detail at very distant coordinates.

## Real-map sources

`StreamWorld::new_mapped(seed, anchor, capacity)` creates an external-source world. `MapAnchor`
fixes latitude, longitude, stud origin, meters per stud, the elevation reference, meters per plate,
the source region extent and whether measured terrain is enabled. No procedural geometry fills
unknown source regions. `advance_generation` leaves the missing chunks pending until the host
publishes a source region; the previous complete snapshot remains usable.

`publish_map_region(MapRegion { bounds, placements })` accepts one complete aligned source region.
The browser uses 128 × 128 studs per region; the renderer still uses its existing 96 × 72 × 96 window.
Bounds contain minimum X/Z and exclusive maximum X/Z. Every part must fit wholly inside the region.
All catalog metadata, vertical bounds and intra-region overlaps are validated before publication.
An identical repeated region is a no-op; a conflicting replacement is rejected to preserve the
geography under player edits. Source generation allocates no user IDs and creates no undo action.

Source identities derive from canonical spatial/catalog ordering inside each owner chunk, not
provider response order. Source parts crossing a 16-stud chunk edge retain the same generated key
from both sides. Normal removal, movement, undo and redo continue to use sparse overrides.

`map_demands(global_focus)` requests missing regions intersecting the current/requested 96-stud
render window, ordered nearest first. Moving out of the central camera guard requests the next
window while the player still has room inside loaded terrain. There is no speculative source
prefetch for adjacent regions, planned routes, or other zoom levels.
The game host requests one region at a time, uses generation/world-ID fences, and publishes only a
complete conversion. The `briccraft-game` mapped session guard holds walkers/vehicles inside the
last resident window while data is pending, then resumes motion through the usual floating-origin
publication. Existing camera pose, constructions, design library and gameplay state are retained.

The source store is bounded separately from the render cache: at most 32 retained regions and
250,000 source parts. Farthest pristine regions may be evicted. Visible/pending regions and regions
referenced by edits or construction history are protected; if all retained regions are protected,
new data fails explicitly rather than discarding constructions. The browser may cache evicted
source data for later requests. Saves include retained source regions, anchor metadata and sparse
edits so the current area can reopen without a network response. Old procedural saves remain valid.

Fixed vertical scale is required: adjacent region conversion must not refit its elevation minimum
or height compression. The 72-plate vertical slab is unchanged. The host must report an area that
cannot fit this fixed range; it must not clip heights or silently change scale at a source seam.
Mapped sessions support walking, flight, cars and aircraft. Their water bodies remain editable
source geometry; boat activation requires the island's sea until vehicle buoyancy can follow
individual mapped water elevations. Streamed guided train networks remain a separate feature.
