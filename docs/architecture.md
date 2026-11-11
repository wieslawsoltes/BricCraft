# Architecture

BricCraft is a Rust workspace with a pinned FerroUI submodule. All visible game controls are native FerroUI declarative elements. The HTML host only mounts the application and supplies browser timing, file/IndexedDB operations and procedural audio.

## Crate boundaries

| Crate | Responsibility | Framework dependency |
| --- | --- | --- |
| `briccraft-parts` | Stable catalogue identities, dimensions, colors and connectors | None |
| `briccraft-world` | Seeded world, occupancy, atomic construction/history, picking, blueprints, validated persistence | None |
| `briccraft-mechanics` | Gear constraints, shafts, transmission, differential, steering, springs, rails and vehicle dynamics | None |
| `briccraft-game` | Input state, cameras, construction/vehicle session and session persistence | None |
| `briccraft-render` | GPU ray traversal, procedural part surfaces, lighting, water and animated assemblies | FerroUI Base effects and bitmaps |
| `briccraft-ui` | Declarative HUD, hotbar, parts browser, blueprint chooser, settings, touch controls | FerroUI View, Controls and Base |
| `briccraft-browser` | Application lifecycle, commands, shader snapshots, native pointer input and browser host | FerroUI Browser, Skia and themes |

## Geometry and rendering

World coordinates use studs on X/Z and plates on Y; a plate is 0.4 stud in height. An instance stores a stable catalogue ID, color, plate-grid position and a quarter-turn orientation. Occupancy contains instance identity, so a 2×4 brick remains one part for edits, picking and rendering.

The renderer packs occupancy and part descriptors into immutable data textures. A world revision or active-assembly membership change rebuilds the textures; camera movement, lighting, selection, water and vehicle motion update shader parameters. A conservative coarse occupancy structure skips empty regions. Fine traversal intersects the actual procedural part shape and its studs, rather than drawing a cube for each occupied cell.

An active vehicle has an independent local volume. Each camera ray is transformed into the vehicle's space, intersected there, and transformed back for lighting. Moving the vehicle therefore does not repeatedly rewrite terrain occupancy. Parking converts its current pose back to grid placements in one atomic transaction.

The current FerroUI browser `Viewport3D` path falls back to a CPU rasterizer. BricCraft therefore uses `CustomEffect`, `EffectImage`, `ShaderEffectDescription::overlay` and `ImmutableCustomEffect` to run its world shader in the explicitly selected Skia backend. It does not install a second JavaScript/WebGL renderer. The framework also draws all UI and its shadows. The animated world effect is deliberately not wrapped in `BitmapCache`: the pinned framework does not invalidate cached visual contents for uniform-only effect changes, which would freeze camera and construction updates. Quality controls adjust shader work without caching the moving scene.

## Simulation and persistence

The session samples held input at 60 Hz. Vehicle integration subdivides to 120 Hz; display frame spikes are bounded. Input is cleared on focus loss, panel interaction and pause. Picking and placement share the same world coordinate conventions as rendering.

Construction transactions validate all removals/additions before committing, so a blueprint, undo, redo or parked assembly cannot be half applied. Save files carry explicit schema versions and reject malformed identities, bounds, colors, overlaps and camera settings before replacing the live session. Browser saves use IndexedDB transactions. Export/import uses the same validated JSON format.

## Current boundaries

This release implements a creative construction sandbox. It is not complete Minecraft gameplay parity: survival progression, hostile creatures, crafting recipes, redstone-like logic, networking and an unbounded streaming world are not implemented. The initial world is 96×72×96 cells, or 96×28.8×96 studs.

The catalogue contains procedural shape families, not a complete manufacturer parts database. Placement occupancy is conservative by part envelope; it does not yet permit physically threading an axle through another part's hole. The mechanical library models gear ratios, shaft constraints and vehicle behavior, but the game does not yet infer and simulate every possible user-built Technic mechanism. Vehicle dynamics are game-oriented approximations rather than a general rigid-body/contact solver. Train movement uses the generated coastal rail route.

These limits are explicit so future work can extend the appropriate crate without replacing UI, persistence or the renderer.
