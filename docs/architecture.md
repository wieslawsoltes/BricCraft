# Architecture

BricCraft is a Rust workspace with a pinned FerroUI submodule. All visible game controls are native FerroUI declarative elements. The HTML host only mounts the application and supplies browser timing, file/IndexedDB operations and procedural audio.

## Crate boundaries

| Crate | Responsibility | Framework dependency |
| --- | --- | --- |
| `briccraft-parts` | Stable catalogue identities, dimensions, colors and connectors | None |
| `briccraft-world` | Seeded world, occupancy, atomic construction/history, picking, blueprints, validated persistence | None |
| `briccraft-stream` | Global coordinates/identities, deterministic chunks, bounded residency, sparse edits, global history and staged imports | None |
| `briccraft-import` | OSM geometry, georeferencing, OBJ/glTF/GLB parsing, texture sampling, surface voxelization and exact-cover brick packing | None |
| `briccraft-import-worker` | Small wasm-bindgen conversion boundary for a dedicated Web Worker | wasm-bindgen only |
| `briccraft-mechanics` | Gear constraints, shafts, transmission, differential, steering, springs, rails and vehicle dynamics | None |
| `briccraft-game` | Input state, cameras, construction/vehicle session and session persistence | None |
| `briccraft-render-data` | Bounded occupancy/descriptor atlases, independent body namespaces and packing metrics | None |
| `briccraft-render` | GPU ray traversal, procedural part surfaces, lighting, water and animated assemblies | FerroUI Base effects and bitmaps |
| `briccraft-ui` | Declarative HUD, hotbar, parts browser, blueprint chooser, settings, touch controls | FerroUI View, Controls and Base |
| `briccraft-browser` | Application lifecycle, commands, shader snapshots, native pointer input and browser host | FerroUI Browser, Skia and themes |

## Global worlds and bounded residency

Global X/Z positions are signed 64-bit studs. The local `World` stays 96 × 72 × 96 cells,
covering six by six 16-stud chunks. A 64-chunk LRU cache retains recently visited terrain;
the saved world contains the seed, generator version, biome preset and sparse edits,
not an ever-growing copy of generated terrain. Meadow, Alpine, Desert, Coast and Flat
share deterministic global sampling, including negative coordinates.

The renderer sees only complete snapshots. Generation and import collision validation
share a two-chunk budget per display frame. A completed replacement shifts the camera
and current vehicle pose into its local coordinate system; persistent construction
identities and exact global positions do not change. Orbit rebasing follows its subject,
while walking, free flight and driving follow the moving position. The outer 16 studs
blend into the atmosphere. A part crossing the outer snapshot boundary stays intact
in global storage and becomes visible once its complete envelope fits the window.

Generated IDs use `(chunk, index)` and user parts use monotonically assigned identities.
Undo transactions contain global before/after placements, so leaving and evicting a
region does not invalidate construction history. Loading a save intentionally starts a
fresh undo session. The global world supports 250,000 sparse edit entries; one staged
import supports 150,000 parts and 4,096 touched chunks. These are explicit resource
budgets, not claims that the entire visible world must be loaded.

## Map and mesh conversion

Network access is user initiated and owned by the browser host. One bounded Overpass
request retrieves a selected area; a small source cache avoids repeated downloads.
Local extracts and the included Brandenburg Gate extract need no live map backend.
Traveling through a world does not scrape successive public-service bounding boxes.
The endpoint is configurable for self-hosted or dedicated use.

`briccraft-import` projects geographic coordinates to a local metric frame, converts
mapped footprints, roads and water to plate-grid cells, and records the evidence used
for each building height. Explicit height tags remain distinct from inferred floor
counts or defaults. Source reports retain projection, dimensions, warning lists and
attribution. Missing elevation and facade geometry are reported rather than invented.

Static mesh import applies source scene transforms and triangulates supported geometry.
Vertex colors, base-color factors and embedded PNG/JPEG textures contribute to palette
colors, including texture-coordinate transforms. Conservative triangle/cell tests
produce a surface shell. Greedy exact-cover packing turns those cells into non-overlapping
catalog bricks and plates without replacing the source with bounding-box approximations.
Source bytes, feature/vertex/triangle counts, decoded texture memory, work, voxels and
output parts are bounded. See the import crate's README for format-specific limitations.

Parsing, texture decoding, voxelization and packing run in a separate WASM Web Worker.
Terminating it cancels conversion. The game then validates the full placement incrementally
before one atomic commit; a collision or cancellation leaves no partial imported model.
New imports can open a new Flat world or place a model at an exact global construction
anchor in an existing world. A mapped world can supply meters per stud for physical
glTF scale. Uniform fitting preserves proportions and respects the available vertical
budget. Source reports from additional models are retained alongside the original map.

## Geometry and rendering

World coordinates use studs on X/Z and plates on Y; a plate is 0.4 stud in height. An instance stores a stable catalogue ID, color, plate-grid position and a quarter-turn orientation. Occupancy contains instance identity, so a 2×4 brick remains one part for edits, picking and rendering.

The renderer packs occupancy and part descriptors into immutable data textures. A world revision or active-assembly membership change rebuilds the textures; camera movement, lighting, selection, water and vehicle motion update shader parameters. A conservative coarse occupancy structure skips empty regions. Fine traversal intersects the actual procedural part shape and its studs, rather than drawing a cube for each occupied cell.

An active vehicle has an independent local volume. Each camera ray is transformed into the vehicle's space, intersected there, and transformed back for lighting. Moving the vehicle therefore does not repeatedly rewrite terrain occupancy. Parking converts its current pose back to grid placements in one atomic transaction.

Its retained `Rc<[Instance]>` and pivot remain in the construction's original local frame,
even after the original terrain region is evicted. Only current pose and static-world
exclusions change on a rebase. Moving-body and static identities have separate atlas
namespaces. Cuboid packing writes occupancy and coarse ranges directly; camera-only
frames reuse both immutable images. Performance counters expose actual publication
counts/bytes separately from frame rate. Reproducible measurements are in the renderer
and render-data crate READMEs; CPU shader timings are not represented as GPU FPS.

The current FerroUI browser `Viewport3D` path falls back to a CPU rasterizer. BricCraft therefore uses `CustomEffect`, `EffectImage`, `ShaderEffectDescription::overlay` and `ImmutableCustomEffect` to run its world shader in the explicitly selected Skia backend. It does not install a second JavaScript/WebGL renderer. The framework also draws all UI and its shadows. The animated world effect is deliberately not wrapped in `BitmapCache`: the pinned framework does not invalidate cached visual contents for uniform-only effect changes, which would freeze camera and construction updates. Quality controls adjust shader work without caching the moving scene.

## Simulation and persistence

The session samples held input at 60 Hz. Vehicle integration subdivides to 120 Hz; display frame spikes are bounded. Input is cleared on focus loss, panel interaction and pause. Picking and placement share the same world coordinate conventions as rendering.

Construction transactions validate all removals/additions before committing, so a blueprint, undo, redo or parked assembly cannot be half applied. Save files carry explicit schema versions and reject malformed identities, bounds, colors, overlaps and camera settings before replacing the live session. Browser saves use IndexedDB transactions. Export/import uses the same validated JSON format.

Session version 2 stores either a classic local world or sparse streamed state, plus the
origin, camera, stable world identity, display name, attribution and source report.
Version 1 island saves remain readable. IndexedDB version 2 adds independent named
world records and a lightweight metadata manifest; listing worlds does not deserialize
all their geometry. The legacy `current` record is migrated and retained as a backup.
The host stores Rust's JSON string unchanged: roundtripping through JavaScript numbers
would lose global coordinates and seeds beyond 2^53. World identity changes happen in
Rust, while separate copies and reversible archiving operate on independent records.

## Current boundaries

This release implements a creative construction sandbox with horizontally streamed worlds.
Complete Minecraft gameplay parity—survival progression, hostile creatures, crafting
recipes, redstone-like logic and networking—is not implemented. Vertical occupancy is
72 plates (28.8 studs); imports must fit uniformly within that budget. There is no
planet-scale terrain-elevation or photogrammetric tile service. Imported real areas are
finite source datasets placed in the open procedural world; their footprint/height
fidelity is limited by source evidence and selected stud resolution.

The catalogue contains procedural shape families, not a complete manufacturer parts database. Placement occupancy is conservative by part envelope; it does not yet permit physically threading an axle through another part's hole. The mechanical library models gear ratios, shaft constraints and vehicle behavior, but the game does not yet infer and simulate every possible user-built Technic mechanism. Vehicle dynamics are game-oriented approximations rather than a general rigid-body/contact solver. Train movement uses the generated coastal rail route; streamed worlds explicitly reject activation without a guided railway network. Cars, planes and boats retain their bodies while crossing regions.

These limits are explicit so future work can extend the appropriate crate without replacing UI, persistence or the renderer.
