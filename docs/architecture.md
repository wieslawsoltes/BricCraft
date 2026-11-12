# Architecture

BricCraft is a Rust workspace with a pinned FerroUI submodule. All visible game controls, including the place picker, are native FerroUI declarative elements. The HTML host mounts the application and supplies browser timing, file/IndexedDB operations, bounded source transport, worker lifetime and procedural audio.

## Crate boundaries

| Crate | Responsibility | Framework dependency |
| --- | --- | --- |
| `briccraft-parts` | Stable catalogue identities, dimensions, colors and connectors | None |
| `briccraft-world` | Seeded world, occupancy, atomic construction/history, picking, blueprints, validated persistence | None |
| `briccraft-stream` | Global coordinates/identities, procedural and mapped source chunks, bounded residency, sparse edits, global history and staged imports | None |
| `briccraft-terrain` | Geographic/WebMercator elevation grids, Terrarium/HGT decoding, bilinear sampling, height fitting and solid terrain packing | None |
| `briccraft-import` | OSM/Shortbread MVT geometry, fixed map projection, OBJ/glTF/GLB parsing, texture sampling, surface voxelization and exact-cover brick packing | None |
| `briccraft-import-worker` | Small wasm-bindgen conversion boundary for a dedicated Web Worker | wasm-bindgen only |
| `briccraft-mechanics` | Gear constraints, shafts, transmission, differential, steering, springs, rails and vehicle dynamics | None |
| `briccraft-character` | Generic voxel collision interface, swept character movement, wall sliding, stepping, gravity and grounded jumps | None |
| `briccraft-play` | Catalog costs, resource permits, Creative/Expedition rules, movement objectives and one-time rewards | None |
| `briccraft-design` | Bounded relative construction recipes, metadata, transforms, library and portable-file validation | None |
| `briccraft-game` | Input state, cameras, construction/vehicle session and session persistence | None |
| `briccraft-render-data` | Bounded occupancy/descriptor atlases, independent body namespaces and packing metrics | None |
| `briccraft-render` | GPU ray traversal, procedural part surfaces, lighting, water and animated assemblies | FerroUI Base effects and bitmaps |
| `briccraft-ui` | Declarative HUD, hotbar, parts browser, blueprint chooser, settings, touch controls | FerroUI View, Controls and Base |
| `briccraft-browser` | Application lifecycle, commands, shader snapshots, native pointer input and browser host | FerroUI Browser, Skia and themes |

## Global worlds and bounded residency

Global X/Z positions are signed 64-bit studs. The local `World` stays 96 × 72 × 96 cells,
covering six by six 16-stud chunks. A 64-chunk LRU cache retains recently visited terrain.
Procedural saves contain the seed, generator version, biome preset and sparse edits;
Meadow, Alpine, Desert, Coast and Flat share deterministic global sampling, including
negative coordinates. Mapped saves additionally retain bounded source regions and
their fixed geographic reference.

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

Network access is owned by the browser host. **Build this area** makes one explicit bounded Overpass
request for the selected rectangle; a small source cache avoids repeated downloads.
Local map extracts and the included Brandenburg Gate extract need no live map backend.
The Telegraph Hill sample also includes its original elevation tiles and provenance.
Its endpoint remains configurable for self-hosted or dedicated use. **Free roam** uses
the separate OSMF Shortbread vector service and requests data for successive resident
views; it does not issue recurring Overpass bounding-box queries.

`briccraft-import` projects geographic coordinates to a local metric frame, converts
mapped footprints, roads and water to plate-grid cells, and records the evidence used
for each building height. Explicit height tags remain distinct from inferred floor
counts or defaults. Source reports retain projection, dimensions, warning lists and
attribution. Facade geometry is not inferred from footprint tags.

### Continuous mapped sessions

The native Places map selects an anchor for a new session. A separate Web Worker
decodes bounded zoom-14 Shortbread MVT ranges, converts supported surface buildings,
roads, water and land-cover layers into the shared geographic importer, and optionally
adds measured DEM terrain. Unsupported layers and underground paths are not invented.
Vector feature IDs are explicitly bundle-local; missing source height/width tags remain
reported inferences. The [map guide](maps.md) records providers and service constraints.

`MapAnchor` fixes latitude/longitude, the stud origin, metres per stud, elevation reference,
metres per plate, a terrain flag and region size. Each `MapRegion` is a complete aligned
128 × 128-stud rectangle. Conversion clips and packs within the exact integer rectangle,
uses the same projection for all regions and disables per-region automatic coarsening.
DEM samples share a global sampling lattice. The first terrain conversion reserves
vertical headroom; later regions retain its vertical reference and scale. An elevation
or structure outside the fixed 72-plate range fails explicitly instead of producing a seam.

Mapped `StreamWorld` backing never generates procedural replacement ground. Missing
source regions keep the requested window pending while the last complete snapshot is
drawn. `map_demands` considers only current/requested resident windows. Source requests
are serialized; cancellation reaches fetch and worker work, and epoch/world-ID fences
reject late completions, including reopening the same saved world. A starting world
is prepared as a candidate and replaces the current session only after its latest state
is saved. Adjacent source publication preserves the live session and camera.

Source placement order is canonicalized into stable generated identities. Parts crossing
an internal chunk boundary have one identity indexed into every intersected chunk. Source
arrival adds no construction undo action or resource charge. Normal building/removal and
vehicle parking continue through sparse global edits and existing transaction validation.
The source cache retains at most 32 regions and 250,000 parts; far pristine regions can
be evicted, while active/pending views and regions referenced by edits/history stay
protected. Exhaustion fails without discarding player work. Saves retain those sources,
anchor metadata and sparse edits, so already generated geography survives reopening.

Walkers and vehicles stop inside the resident window while another region is unavailable.
They can turn or reverse, and movement resumes after the complete next view publishes.
Returning to the current view withdraws an obsolete demand. Cars and aircraft work on
mapped terrain; boats require the classic sea until buoyancy supports mapped water heights.
The renderer disables its classic global sea plane for mapped worlds; mapped water remains
source geometry. This path preserves existing rendering policies and does not establish
a hardware frame-rate claim.

### Elevation and complete-area generation

`briccraft-terrain` owns elevation independently of OSM and rendering. An `ElevationGrid`
records node-center bounds, Geographic or WebMercator spacing, nullable metre samples,
vertical datum and source evidence. Terrarium decoding preserves the original 8-bit
channels; browser canvas color management never touches elevation bytes. Bilinear
sampling crosses tile seams and rejects missing required neighbors. HGT decoding is
also available to Rust consumers; the browser's live path uses Terrarium.

The browser plans at most 16 DEM tiles, downloads three concurrently, limits each
compressed tile to 1 MiB and their combined payload to 12 MiB, and caches at most 48
tiles. Packed bytes transfer to the worker. The Rust decoder validates exact ranges,
dimensions, alpha and bounded allocation before sampling the area. No survey timestamp
or datum is invented when the provider has not supplied one.

For an area import, elevation is sampled over the whole requested projected rectangle.
Roads and land cover follow it. Buildings use level foundations at their highest ground
sample. Enclosed water uses a recorded inferred elevation. Solid ground is packed from
column heights, without allocating every interior voxel in a hash table. Parts stay
within global 16×16 chunk boundaries, so terrain remains complete when a resident
window moves. Feature geometry retains its separate bounded voxel budget.

The default fit option computes a shared vertical scale for terrain relief and buildings
within the 54 available plates above the import baseline. Horizontal scale remains
unchanged unless a resource budget forces deterministic whole-area coarsening. Up to
nine complete conversion attempts are allowed; failure never publishes a partial area.
Reports retain requested/effective horizontal and vertical scales, compression,
elevation range, datum, column counts and inference counts. Strict height mode rejects
overflow instead. The [terrain contract](../crates/briccraft-terrain/README.md) documents
the node-center and boundary-cell conventions.

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

The current FerroUI browser `Viewport3D` path falls back to a CPU rasterizer. BricCraft therefore uses `CustomEffect`, `EffectImage`, `ShaderEffectDescription::overlay` and `ImmutableCustomEffect` to run its world shader in the explicitly selected Skia backend. The framework also draws all UI and its shadows.

The world effect belongs to a child visual inside a separate parent `BitmapCache`. This distinction matters in the pinned framework: an effect change on the cache root does not dirty its own cached image, while the changing child contributes its dirty bounds to the parent's cache collector. The parent renders its subtree into a smaller GPU surface and composites it below the full-resolution HUD. Camera, water and vehicle changes keep invalidating that world surface; the browser gate checks actual camera-driven pixel changes and unchanged atlas publication counts.

The allocation controller separates image density, shader detail and spatial sample
count. Auto starts at native density, capped at 4,194,304 physical pixels and 4,096
pixels per axis; its floor is 0.75 world pixels per CSS pixel per axis within those
limits. Four presented warmup frames exclude cold shader stalls. Under sustained
pressure, AA samples drop first, then detail, then density. Recovery needs three
seconds of headroom and restores density before detail and samples. Paused/modal
frames do not drive adaptation. Quality fixes native density within an 8,294,400-pixel
cap and honors requested detail/samples. Explicit Performance mode starts at Fast,
one sample and 0.4 CSS density, recovering toward at most 0.75 density. The controller
is framework-independent and tested for resize, device scale, startup, quality
floors, sample order and idle periods. Preferences live in browser storage rather
than saved-world state. Settings exposes actual dimensions/detail/samples.

Fast and Balanced use analytic part intersections; Cinematic retains detailed
signed-distance part shapes. Their separate effect types exclude secondary traversal
from Fast instead of hiding it behind a uniform branch. All support deterministic
1/2/4 subpixel sampling, resolved before the display curve. Analytic pinhole
differentials project onto hit surfaces to filter seams, stud rings, molded normals,
grid lines and moving-body materials without shader derivatives. Grazing footprints
are clamped; GGX highlights include curved-surface variance and material roughness.
Native coverage/error probes and measured CPU costs are recorded in
`crates/briccraft-render/ANTIALIASING.md`.

The renderer retains an exact immutable effect snapshot and returns its existing
`Rc<dyn IEffect>` only when every uniform and atlas-image identity matches. Visual
time stops while paused or modal, so an unchanged world can keep its bitmap while
the native UI remains interactive. Scene changes invalidate both snapshot and atlases;
resolution changes clear only the snapshot and explicitly invalidate the child
visual, because FerroUI compares equal effects structurally. This also repaints a
paused world after a density change. Publication and reuse counters remain separate
from actual presented-frame statistics.

The browser adapter also skips repeated `WebRenderTarget.setSize` calls when its
owned WebGL canvas already has exactly the requested backing dimensions. Real size
changes and all foreign, software, and worker targets retain the framework's
behavior. This avoids redundant width/height assignments without changing the
submodule or DOM prototypes. The browser gate records a same-page A/B/A comparison
of actual presented frames, size calls, world resolution, shader quality and AA samples;
different adaptive configurations are explicitly marked as incomparable.

A conservative hierarchy stores maximum occupied heights for 4-, 16- and 64-stud
columns in 576 RGB texels. Rays skip air only above the full part envelope plus its
studs; roofs, caves and overhangs still use ordinary three-dimensional intersections.
All qualities share the primary traversal and cache coarse occupancy. Fractional-origin
cell calculations prevent a tiny negative ray direction from rounding back onto an
integer camera boundary and exhausting the traversal loop. This fixes missing geometry
as well as unnecessary work.

## Responsive UI and touch

The world occupies the full native viewport. A separate declarative UI layer derives
its usable rectangle from FerroUI `InsetsManager` and `InputPane`, with browser viewport
and coarse-pointer information supplied only as environment signals. The layer adapts
between portrait, wide and short-landscape layouts; scrollable panels remain reachable
above the keyboard and safe areas. Buttons use 44–48-pixel touch targets.

Touch navigation tracks contacts independently. A single finger aims/orbits, two fingers
pan and pinch, and lifting one finger after a pinch cannot place a part. Explicit Place,
Remove and Rotate buttons perform edits; the movement pad and vehicle Brake/Park actions
use the same game commands as desktop input. Modal panels own a full backdrop, and the
world rejects pointer/wheel gestures while a modal is open. Source credits expand into a
native scrollable panel rather than occupying the world with a long attribution string.

The persistent HUD uses compact world/objective chips and a floating grouped tool
bar. Detailed world status and performance diagnostics are opt-in panels/settings.
Drive/Park and touch vertical controls reflect the active vehicle kind; ground
vehicles do not advertise inert rise/descend controls. Shared vector icons, panel
headers and cards are native `ferroui-view` builders. Settings, help, blueprint
previews, saved designs and world details are separate modules.

Places composes native image tiles, selection overlays, zoom controls, presets and
coordinate fields. Pointer capture isolates map gestures from the world and surrounding
scroll panel; tap selection is distinct from dragging/pinching. Desktop uses a map and
options column, while compact layouts scroll the controls. The browser only fetches and
validates 256 × 256 PNG tiles, then supplies immutable images to the native view. Provider
templates are configurable without rebuilding; source attribution stays on the map.

### Reusable construction recipes

`briccraft-design` validates bounded relative placements independently of UI, browser
storage and session history. It preserves catalog identity, color and quarter-turn
rotation, and checks envelope, overlap, connected components, names, IDs and source
metadata. Each world carries an optional/defaulted library of up to 32 recipes, with
256 parts and a 32 × 72 × 32-cell envelope per recipe. Invalid libraries reject the
candidate save before replacing the current session; legacy saves start empty.

Session capture selects a bounded connected assembly or the last complete grouped
construction. Scene/revision/hit/group keys cache the capture assessment. Terrain and
potentially truncated boundary assemblies are rejected. Source metadata keeps exact
64-bit global coordinates. Placement transforms the relative recipe into the current
anchor/rotation and delegates to the existing atomic construction transaction,
economy, streaming edits and grouped history. A blocked target fails without a search
for a different placement; copies keep ordinary workshop/activation behavior.

Portable versioned `.bricdesign` files are limited to 256 KiB. Import validates the
entire recipe, allocates a local ID and resolves duplicate names, preserving source
metadata. Importing a recipe changes neither geometry, money nor objectives. Browser
file handling is a host service; capture, rename, build, import/export and inline
delete confirmation remain native declarative controls. The file callback rejects
an intervening world switch before importing into a different library.

## Simulation and persistence

The session samples held input at 60 Hz. Vehicle integration subdivides to 120 Hz; display frame spikes are bounded. Input is cleared on focus loss, panel interaction and pause. Open modal panels suspend simulation, including inertia, gravity and the follow camera, while incremental streaming/import work can continue. Picking and placement share the same world coordinate conventions as rendering.

Walking uses a reusable upright cylindrical controller over the occupancy grid. Its
0.32-stud radius, 2.5-unit body and 2.2-unit eye height are explicit configuration.
Horizontal sweeps resolve faces and rounded corners, then project remaining movement
along contact planes. Continuous vertical sweeps prevent tunneling through plate floors
or ceilings. A step is at most two plates and needs clear headroom and destination
support. Gravity and acceleration integrate in bounded 1/120-second substeps. Jump is
grounded and press-triggered. Fly and Orbit remain separate; Walk panning cannot bypass
collision. Controller state is ephemeral while the camera's serialized schema remains
compatible. Position belongs to the caller, so streaming can rebase without resetting
velocity. The occupancy contract and bottom safety plane are documented in the character
crate; shaped visual holes are conservative solids and swimming is not implemented.

### Expedition transactions and progression

`briccraft-play` has no world, rendering or UI dependency. A stable catalog price combines
part-envelope material volume with functional-component cost. A `ChangeValue` contains
the actual added and removed catalog values. `Gameplay::approve` produces a non-cloneable,
single-use `ResourcePermit` without mutating the wallet. Session first validates and
commits the world transaction and captures streamed edits; only then does it consume
the permit and record successful gameplay activity. A failed local or global edit does
not consume funds or advance objectives.

History exposes read-only next-change views. Undo and redo quote the actual forward or
inverse change before mutation; an unaffordable inverse leaves geometry and history in
place. Parking changes pose without changing part value. Objective rewards are not
replayed by history. Mode changes clear both local and global construction history to
avoid repricing an earlier free action under the paid ruleset.

Eight objective tracks record committed placements, recycling, structures, mechanical
activation and actual Walk/vehicle movement. Distances are accumulated from simulation
steps, with bounded millistud counters; camera focus, free flight and origin changes do
not add travel. A bitset records rewards already granted. Balance, mode, counters and
claimed rewards persist in a validated optional gameplay envelope; legacy saves default
to Creative. Limits reject unsupported versions and oversized state before replacing
the active session. Switching back from Creative preserves progress and never reissues
the initial supply. Creative still allows unlimited world editing by design.

Workshop display and activation share one preparation/assessment path. The native UI
receives small presentation snapshots and price quotes; it does not calculate prices,
award objectives or duplicate mechanical readiness rules.

Construction transactions validate all removals/additions before committing, so a blueprint, undo, redo or parked assembly cannot be half applied. Save files carry explicit schema versions and reject malformed identities, bounds, colors, overlaps and camera settings before replacing the live session. Browser saves use IndexedDB transactions. Export/import uses the same validated JSON format.

Session version 2 stores either a classic local world or streamed state, plus the
origin, camera, stable world identity, display name, attribution and source report.
Streamed state has an optional mapped-source envelope containing the anchor and retained
regions; older procedural saves remain valid. `WorldInfo::validate` shares persistence
limits with source publication, preventing invalid reports or credits from making a
newly generated world unsaveable. The browser also validates region projection, bounds
and vertical scale before appending source geometry.
Version 1 island saves remain readable. IndexedDB version 2 adds independent named
world records and a lightweight metadata manifest; listing worlds does not deserialize
all their geometry. The legacy `current` record is migrated and retained as a backup.
The host stores Rust's JSON string unchanged: roundtripping through JavaScript numbers
would lose global coordinates and seeds beyond 2^53. World identity changes happen in
Rust, while separate copies and reversible archiving operate on independent records.

## Current boundaries

This release implements a creative construction sandbox with horizontally streamed worlds.
Complete Minecraft gameplay parity—survival/hunger systems, hostile creatures, crafting
recipes, redstone-like logic and networking—is not implemented. Expedition adds a
construction economy and real-action progression. Vertical occupancy is
72 plates (28.8 studs); mesh imports fit uniformly, while area imports expose explicit
vertical fitting. Explicit Overpass imports remain finite datasets; mapped free roam
continues through successive source regions around a fixed local projection. Its
fixed vertical range, bounded protected source store, provider availability and map
coverage remain limits. It is not an unrestricted globe or a photogrammetric tile service.
Source footprint/height fidelity is limited by available tags, generalized vector
geometry, DEM resolution and selected stud scale. Address/name geocoding is not implemented;
the place picker supports map interaction, city presets and coordinates.

The catalogue contains procedural shape families, not a complete manufacturer parts database. Placement occupancy is conservative by part envelope; it does not yet permit physically threading an axle through another part's hole. The mechanical library models gear ratios, shaft constraints and vehicle behavior, but the game does not yet infer and simulate every possible user-built Technic mechanism. Vehicle dynamics are game-oriented approximations rather than a general rigid-body/contact solver. Train movement uses the generated coastal rail route; streamed worlds explicitly reject activation without a guided railway network. Cars and planes retain their bodies while crossing mapped regions. Mapped boat activation is rejected consistently by the workshop and activation path; island/procedural boat behavior is unchanged.

These limits are explicit so future work can extend the appropriate crate without replacing UI, persistence or the renderer.
