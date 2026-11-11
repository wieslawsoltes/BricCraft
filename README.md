# BricCraft

**Small bricks. Endless possibilities.** A browser construction sandbox built in Rust with declarative FerroUI controls, Skia GPU shaders and reusable simulation crates.

[Play BricCraft](https://wieslawsoltes.github.io/BricCraft/) · [Architecture](docs/architecture.md) · [Map and model imports](crates/briccraft-import/README.md) · [Build status](https://github.com/wieslawsoltes/BricCraft/actions)

## Build a world, then bring it to life

Create a collection of editable brick worlds: open meadows, alpine landscapes, desert dunes, coastlines, a flat construction studio, or the original harbor island. Travel through streamed terrain, turn real OpenStreetMap geometry into bricks, and bring imported 3D models into your constructions. Every generated and imported piece uses the same catalog, placement, picking and undo system.

- **Open worlds:** deterministic 16-stud chunks, signed 64-bit global coordinates, a floating origin, bounded terrain residency, sparse persistent edits and global undo across region changes.
- **World library:** named browser saves, automatic save before switching, separate copies, reversible archiving and migration of the original single-world save.
- **Complete mapped areas:** coordinate-based OSM imports combine measured terrain elevation, mapped buildings, roads, water and land cover. Roads follow the ground; buildings receive level foundations. The included Telegraph Hill area contains real San Francisco elevation and 1,351 mapped building features, and works offline after the app assets load. A smaller Brandenburg Gate example remains available.
- **Real 3D models:** GLB, embedded glTF and OBJ conversion in a background Web Worker. Scene transforms, vertex/material colors and embedded PNG/JPEG base-color textures are sampled into bricks. Open a model world or add a model to an existing mapped world using its meters-per-stud scale.
- **161 procedural parts and 32 colors:** bricks, plates, tiles, slopes, rounded pieces, arches, windows, doors, furniture, plants, liftarms, gears, axles, wheels, power components and transport pieces.
- **Eight construction blueprints:** trail rover, island hopper, coastal locomotive, lagoon sailboat, harbor cottage, lighthouse, windmill and bridge.
- **Construction tools:** face picking, placement preview with collision feedback, quarter-turn rotation, color selection, eyedropper, grouped undo/redo and a construction grid.
- **Working vehicles:** activate connected constructions, drive or fly with the keyboard, then park them back onto the construction grid. Cars, aircraft, trains and boats have distinct simulation models.
- **Reusable mechanical systems:** connected assembly assessment, motor/power checks, gear speed and torque ratios, locked-loop detection, transmission, differential, steering, spring/damper joints and a sampled rail path.
- **Three cameras:** orbit/pan/zoom for building, free flight and ground-following walking with jumping.
- **Native declarative UI:** responsive HUD, hotbar, searchable/filterable parts library, palette, blueprint chooser, touch controls, settings and source credits. Phone layouts account for safe areas, the software keyboard and short landscape screens; world gestures aim and navigate while explicit buttons place and remove parts.
- **Persistence:** transactional IndexedDB saves and versioned `.briccraft` file export/import with validation before replacement.
- **Adaptive Skia rendering:** molded surfaces, real stud silhouettes, part seams, mechanical shapes, lighting, water, atmosphere and animated vehicle assemblies. A separate GPU world layer adapts resolution and shader detail while native controls remain sharp.

## Real-world studio

Open **Worlds → Build a real place or 3D model → Generate a real area**. Select **San Francisco hills** to build the included Telegraph Hill area from its real OSM extract and four original Terrarium elevation tiles. **Brandenburg Gate** provides a smaller flat-ground example. Both use the production Web Worker conversion pipeline and need no live map service.

For another location, enter latitude, longitude, radius (20–1,500 metres) and metres per stud. **Use real elevation** requests bounded public elevation tiles alongside the map. Choose **1x** relief for the source scale or **2x** to emphasize the hills. **Fit terrain and building height** fits both together into the current vertical window and records the exact compression; disable it to reject a source that cannot fit at the selected vertical scale. A local OSM extract uses the entered location fields for its elevation request.

The complete requested rectangle is generated, including its ground. If a resource budget requires a lower horizontal resolution, conversion retries the whole area at a recorded coarser scale; it does not silently crop to the first features. Missing required elevation samples fail explicitly. Source reports retain measured ranges, effective scales, vertical datum, inferred building heights, foundation adjustments and water levels. The bundled sample retains all supported source features and their complete geometry; see its [provenance and reference result](crates/briccraft-browser/wwwroot/assets/maps/telegraph-hill.README.md).

Public Overpass availability varies; repeat or production use should use your own endpoint or locally prepared extracts. Elevation requests use at most 16 tiles, three concurrent downloads and a bounded local cache. Panning never triggers network map requests. Source credits are available from the HUD and travel with saved worlds.

Model conversion runs in a separate WASM worker and can be canceled. The complete placement is validated in bounded chunk batches and committed as one undo action. Choose **Add a model to this world** to combine source geometry with your current build; the selected construction target is its anchor. Source reports and attribution travel with `.briccraft` saves.

## Controls

| Action | Control |
| --- | --- |
| Place / remove a part | Click / right-click the world |
| Orbit or look | Drag the world |
| Pan | Shift-drag or middle-drag |
| Zoom | Mouse wheel / trackpad / two-finger pinch |
| Touch navigation | One-finger look/orbit; two-finger pan and pinch; movement pad |
| Touch construction | Tap to aim, then Place / Remove / Rotate |
| Pick part and color | Middle-click |
| Rotate selected part | R |
| Hotbar | 1–8 |
| Parts library | B |
| World library | G |
| Change camera | Tab |
| Move | W A S D / arrow keys |
| Ascend / descend in flight | E / Q |
| Jump when walking | Space |
| Faster movement | Shift |
| Activate vehicle | F / V or Drive in the world panel |
| Throttle / steer | W S / A D |
| Aircraft lift / vehicle brake | E Q / B |
| Park vehicle / close panel | Escape |
| Undo / redo / save | Ctrl/Cmd Z / Y / S |
| Reset camera / help | Home / H |

## Build

```sh
git clone --recurse-submodules https://github.com/wieslawsoltes/BricCraft.git
cd BricCraft
```

The current framework is pinned at `1c51d201a74fb835c065ab43d1bf091429f92392`, version `0.5.0-preview.1`. The browser toolchain is pinned by that submodule: Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129.

```sh
# Install the exact browser tools once.
bash FerroUI/scripts/browser/setup.sh ../browser-tools
source ../browser-tools/env.sh

# One build job keeps the framework's host/wasm compilations within smaller RAM budgets.
CARGO_BUILD_JOBS=1 CARGO_PROFILE_BROWSER_CODEGEN_UNITS=8 bash scripts/build-browser.sh
python3 -m http.server 8080 --directory target/browser/briccraft-browser
```

Open `http://localhost:8080`. The wrapper also builds the separate `wasm32-unknown-unknown` import engine. GitHub Actions builds both optimized WASM modules, exercises the shipped conversion ABI, runs the real Chrome/Skia game regressions and the framework smoke check, and publishes successful `main` builds to GitHub Pages.

```sh
cargo test --locked -p briccraft-parts -p briccraft-world -p briccraft-mechanics \
  -p briccraft-game -p briccraft-stream -p briccraft-terrain -p briccraft-import \
  -p briccraft-import-worker -p briccraft-render-data
cargo check --locked -p briccraft-browser
python3 FerroUI/scripts/apps/sync-workspace.py --check .
```

`briccraft-render-data` supplies independent atlas tests and a reproducible packing benchmark. `briccraft-render` includes camera tests and a Skia shader smoke tool. The application calls FerroUI's `CustomEffect`, `EffectImage` and immutable effect APIs; all game UI uses `ferroui-view` builders and signals. The JavaScript host supplies browser services only.

Interactive rendering uses **Skia WebGL2**. The pinned framework requests rejection of WebGL contexts with a major performance caveat, which can select CPU Skia even when the browser permits a software WebGL driver. A small browser adapter scopes the game's context preference to its own canvas and permits that browser-approved WebGL2 path. Other canvases and context types keep their original options; the framework submodule is unchanged. When WebGL2 is unavailable, startup explains the graphics requirement. The CI game gate verifies the actual presented target and OpenGL ES version, and the FPS display counts FerroUI's presented frames over elapsed wall time. The world begins with a bounded GPU surface and adjusts its resolution and shader detail to observed presentation performance. Fast, Balanced and Cinematic set the maximum detail; UI stays at the browser's full device resolution. CI compiles and renders all three production shader variants and tests camera repainting and real vehicle movement.

## Workspace

Twelve reusable crates separate the part library, local world, streaming, elevation and terrain, map/model conversion, worker bindings, mechanics, game state, atlas packing, renderer, UI and browser lifecycle. The framework-independent [terrain crate](crates/briccraft-terrain/README.md) exposes geographic grids, Terrarium/HGT decoding, sampling, vertical fitting and solid-ground packing. See [the architecture and current boundaries](docs/architecture.md) for interfaces and implementation details.

This is a creative construction sandbox. Horizontal worlds stream beyond the visible region; the current GPU working window is 96×72×96 occupancy cells, with one plate equal to 0.4 stud. Model imports retain uniform proportions. Mapped areas can explicitly fit their vertical scale while preserving horizontal georeferencing; every adjustment is reported. Elevation comes from a separate DEM source, whose resolution and survey age vary. OSM footprints and height tags do not supply facade scans, and a finite imported area is not a continuously streamed planet service.

Complete survival/crafting/mob/multiplayer gameplay, a complete manufacturer part database and a general solver for arbitrary articulated mechanisms remain future work. Cars, aircraft and boats can cross streamed regions; trains currently follow the classic island railway. See the architecture document for precise boundaries.

## License

MIT. Dependency and asset notices are in [NOTICE.md](NOTICE.md).
