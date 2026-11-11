# BricCraft

**Small bricks. Endless possibilities.** A browser construction sandbox built in Rust with declarative FerroUI controls, Skia GPU shaders and reusable simulation crates.

[Play BricCraft](https://wieslawsoltes.github.io/BricCraft/) · [Architecture](docs/architecture.md) · [Map and model imports](crates/briccraft-import/README.md) · [Build status](https://github.com/wieslawsoltes/BricCraft/actions)

## Build a world, then bring it to life

Create a collection of editable brick worlds: open meadows, alpine landscapes, desert dunes, coastlines, a flat construction studio, or the original harbor island. Travel through streamed terrain, turn real OpenStreetMap geometry into bricks, and bring imported 3D models into your constructions. Every generated and imported piece uses the same catalog, placement, picking and undo system.

- **Open worlds:** deterministic 16-stud chunks, signed 64-bit global coordinates, a floating origin, bounded terrain residency, sparse persistent edits and global undo across region changes.
- **World library:** named browser saves, automatic save before switching, separate copies, reversible archiving and migration of the original single-world save.
- **Expedition gameplay:** an optional per-world Stud budget, harvesting/recycling, catalog and blueprint prices, eight rewarded objectives, and a workshop that assesses the actual connected construction. Creative remains unlimited; both modes preserve your world and expedition progress.
- **Complete mapped areas:** coordinate-based OSM imports combine measured terrain elevation, mapped buildings, roads, water and land cover. Roads follow the ground; buildings receive level foundations. The included Telegraph Hill area contains real San Francisco elevation and 1,351 mapped building features, and works offline after the app assets load. A smaller Brandenburg Gate example remains available.
- **Real 3D models:** GLB, embedded glTF and OBJ conversion in a background Web Worker. Scene transforms, vertex/material colors and embedded PNG/JPEG base-color textures are sampled into bricks. Open a model world or add a model to an existing mapped world using its meters-per-stud scale.
- **161 procedural parts and 32 colors:** bricks, plates, tiles, slopes, rounded pieces, arches, windows, doors, furniture, plants, liftarms, gears, axles, wheels, power components and transport pieces.
- **Eight construction blueprints:** trail rover, island hopper, coastal locomotive, lagoon sailboat, harbor cottage, lighthouse, windmill and bridge.
- **Your own design library:** capture a connected build or the last complete construction group, name it, and place reusable copies. Transfer validated `.bricdesign` files between worlds; custom designs share the same prices, grouped undo, collision checks and vehicle activation as other constructions.
- **Construction tools:** face picking, placement preview with collision feedback, quarter-turn rotation, color selection, eyedropper, grouped undo/redo and a construction grid.
- **Working vehicles:** activate connected constructions, drive or fly with the keyboard, then park them back onto the construction grid. Cars, aircraft, trains and boats have distinct simulation models.
- **Reusable mechanical systems:** connected assembly assessment, motor/power checks, gear speed and torque ratios, locked-loop detection, transmission, differential, steering, spring/damper joints and a sampled rail path.
- **Three cameras:** orbit/pan/zoom for building, free flight, and collision-aware walking with wall sliding, gravity, grounded jumping, ceiling clearance and steps over low terrain.
- **Native declarative UI:** compact world and objective chips, a floating construction toolbar, contextual Drive/Park controls, a searchable parts library, palette, design library and dedicated world details. Phone layouts account for safe areas, the software keyboard and short landscape screens; world gestures aim and navigate while explicit buttons place and remove parts. Performance diagnostics are optional in Settings.
- **Persistence:** transactional IndexedDB saves and versioned `.briccraft` file export/import with validation before replacement.
- **Antialiased Skia rendering:** 1/2/4 spatial samples, filtered molded details and highlights, real stud silhouettes, part seams, mechanical shapes, water, atmosphere and animated vehicle assemblies. Auto starts at native density with a display-relative sharpness floor; Quality locks the selected density/detail/samples, and Performance offers an explicit lower-cost mode. Native controls always retain display resolution.

## Real-world studio

Open **Worlds → Build a real place or 3D model → Generate a real area**. Select **San Francisco hills** to build the included Telegraph Hill area from its real OSM extract and four original Terrarium elevation tiles. **Brandenburg Gate** provides a smaller flat-ground example. Both use the production Web Worker conversion pipeline and need no live map service.

For another location, enter latitude, longitude, radius (20–1,500 metres) and metres per stud. **Use real elevation** requests bounded public elevation tiles alongside the map. Choose **1x** relief for the source scale or **2x** to emphasize the hills. **Fit terrain and building height** fits both together into the current vertical window and records the exact compression; disable it to reject a source that cannot fit at the selected vertical scale. A local OSM extract uses the entered location fields for its elevation request.

The complete requested rectangle is generated, including its ground. If a resource budget requires a lower horizontal resolution, conversion retries the whole area at a recorded coarser scale; it does not silently crop to the first features. Missing required elevation samples fail explicitly. Source reports retain measured ranges, effective scales, vertical datum, inferred building heights, foundation adjustments and water levels. The bundled sample retains all supported source features and their complete geometry; see its [provenance and reference result](crates/briccraft-browser/wwwroot/assets/maps/telegraph-hill.README.md).

Public Overpass availability varies; repeat or production use should use your own endpoint or locally prepared extracts. Elevation requests use at most 16 tiles, three concurrent downloads and a bounded local cache. Panning never triggers network map requests. Source credits are available from the HUD and travel with saved worlds.

Model conversion runs in a separate WASM worker and can be canceled. The complete placement is validated in bounded chunk batches and committed as one undo action. Choose **Add a model to this world** to combine source geometry with your current build; the selected construction target is its anchor. Source reports and attribution travel with `.briccraft` saves.

## Play an expedition

Open **Play / Goals** or press **O**, then select **Expedition**. Each world starts once
with **1,200 Studs**. Placing a part or blueprint spends its quoted catalog value;
removing a part recycles that value into the balance. Colors and rotations do not
change its price. The native controls show exact prices and disable unaffordable
builds; game rules also enforce the balance for keyboard and public API commands.

A trail rover costs **377 Studs** and contains **23 editable parts**. Build it on
clear ground, inspect the **Workshop** tab, and activate it when the actual power,
running gear and assembly checks pass. Drive with W/S and A/D, or the touch movement
pad; Brake and Park are available on mobile. The workshop shares its preparation
logic with activation, so its reasons describe the construction you will use.

Objectives use successful construction and actual simulated movement:

| Objective | Requirement | Stud reward |
| --- | --- | ---: |
| Gather supplies | Recycle 8 parts | 140 |
| Make your mark | Place 24 parts, including blueprint pieces | 180 |
| Find your footing | Walk 120 studs | 120 |
| Establish a base | Build a cottage, lighthouse, windmill or bridge | 220 |
| Bring it to life | Activate a functional connected vehicle | 260 |
| Follow the trail | Travel 240 studs in a land/water vehicle | 320 |
| Take to the sky | Fly a powered airplane 160 studs clear of terrain | 420 |
| A world of possibilities | Travel 1,000 studs by walking or vehicle | 500 |

Rewards are automatic, one-time, and saved with the world. Camera orbiting, free
flight, focusing a blueprint and stream-origin shifts do not earn travel progress.
Undo/redo reverses the actual construction value without replaying objective rewards;
restoring a recycled part requires repaying its refund. Failed validation changes
neither the wallet nor progression. Saving and switching worlds retain this state.

You can return to Creative whenever the vehicle is parked and an import is complete.
Mode changes clear construction history while preserving geometry, balance and goals.
Creative can assist an Expedition: freely built, generated and imported parts can later
be recycled. This is a local sandbox ruleset, not a competitive anti-cheat boundary.
New mapped/model worlds start in Creative; switch that world to Expedition when ready.
Appending imported geometry to an existing world requires Creative.

## Save a construction as a design

Open **Plans → My designs** after selecting a connected construction or completing a
blueprint/model placement. The capture card explains what will be saved and reports
the part count. Name the design, then save it. The library retains part IDs, colors,
quarter-turn rotations and relative positions; the original source world and global
anchor are preserved as metadata.

Use **Place design** to place a complete copy at the current construction target. Every part
is validated before one atomic transaction, so a collision or insufficient balance
leaves the world unchanged. In Expedition, the card displays the exact total Stud cost.
Undo and redo operate on the complete copy. Functional vehicle copies can be activated
through the same Workshop and Drive controls.

Use **Export** and **Import** to move a `.bricdesign` recipe between worlds. Importing a
recipe creates a library entry; building it is a separate action. Rename and delete
operate on the saved recipe, and deleting a recipe leaves its existing world copies
in place. Each world stores up to 32 designs, with at most 256 parts and a
32 × 72 × 32-cell envelope per design. Portable files are limited to 256 KiB and
validated before being accepted. Capturing rejects terrain and incomplete assemblies
at a streamed window boundary; active vehicles must be parked first.

## Image quality

Open **Settings → Display**. Density and antialiasing choices are retained by the
browser independently of the selected world; surface detail remains part of a world's
saved settings.

| Setting | Behavior |
| --- | --- |
| Auto, the default | Starts at native display density, with two AA samples. Sustained slow presentation reduces samples, then shader detail, then density. Density stays at or above 0.75 world pixels per CSS pixel on each axis, subject to allocation limits. |
| Quality | Uses native display density within an 8,294,400-pixel / 4,096-pixel-per-axis allocation limit, and honors the selected detail and AA sample count. It does not silently lower them to reach a frame-rate target. |
| Performance | Starts with Fast detail, one sample and 0.4 world pixels per CSS pixel on each axis. It can recover toward 0.75 density when presentation has sustained headroom. |
| Antialiasing | One, two or four subpixel ray samples. In Auto/Performance this is the requested ceiling; in Quality it is the actual count. |
| Detail | Fast and Balanced use analytic intersections; Cinematic uses the detailed signed-distance shapes. All share filtered materials and real geometry. |

At a 1280 × 800 desktop viewport, Auto starts with a 1280 × 800 world and retains a
roughly 960 × 600 density floor. Auto also limits allocations to 4,194,304 physical pixels
and 4,096 pixels per axis. The UI remains native resolution in every mode. Settings
reports the actual world dimensions, active sample count and effective detail.
Cold shader compilation and idle menus are excluded from adaptive measurements.

Filtering integrates subpixel seams, stud rings and surface patterns over their
projected footprint; narrow molded highlights broaden at distance. Four samples
reduced geometry coverage error by approximately 60% in the committed native fixture.
Higher density and more samples cost rendering work. Reusing the exact world effect
while paused or in an unchanged menu avoids rerendering that world layer. See the
[AA measurements and methodology](crates/briccraft-render/ANTIALIASING.md) for the
reference images, CPU costs and limits; these are not hardware GPU frame-rate claims.

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
| Play, objectives and workshop | O |
| Change camera | Tab |
| Move | W A S D / arrow keys |
| Ascend / descend in flight | E / Q |
| Jump when walking | Space |
| Faster movement | Shift |
| Activate vehicle | F / V or the contextual Drive control |
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

The current framework is pinned at `8281ef991022abb139c092bae75d4e652361a315`, version `0.5.0-preview.1`. The browser toolchain is pinned by that submodule: Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129.

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
  -p briccraft-import-worker -p briccraft-render-data -p briccraft-play -p briccraft-character \
  -p briccraft-design
cargo test --locked -p briccraft-render --lib
cargo check --locked -p briccraft-browser
python3 FerroUI/scripts/apps/sync-workspace.py --check .
```

`briccraft-render-data` supplies independent atlas tests and a reproducible packing benchmark. `briccraft-render` includes camera tests and a Skia shader smoke tool. The application calls FerroUI's `CustomEffect`, `EffectImage` and immutable effect APIs; all game UI uses `ferroui-view` builders and signals. The JavaScript host supplies browser services only.

Interactive rendering uses **Skia WebGL2**. The pinned framework requests rejection of WebGL contexts with a major performance caveat, which can select CPU Skia even when the browser permits a software WebGL driver. A small browser adapter scopes the game's context preference to its own canvas and permits that browser-approved WebGL2 path. Other canvases and context types keep their original options; the framework submodule is unchanged. When WebGL2 is unavailable, startup explains the graphics requirement. The CI game gate verifies the actual presented target and OpenGL ES version; optional FPS diagnostics count FerroUI's presented frames over elapsed wall time. The long functional journey explicitly selects Performance for CI's software GPU, while a separate gate verifies native-density Auto/Quality, actual four-sample world pixels, paused-world reuse and desktop/phone screenshots. CI also compiles all three production shader variants and checks geometry coverage, material filtering, camera repainting and vehicle movement.

## Workspace

Fifteen reusable crates separate the part library, local world, streaming, elevation and terrain, map/model conversion, worker bindings, mechanics, character motion, expedition rules, saved designs, game state, atlas packing, renderer, UI and browser lifecycle. The framework-independent [terrain crate](crates/briccraft-terrain/README.md) exposes geographic grids, Terrarium/HGT decoding, sampling, vertical fitting and solid-ground packing. [Character motion](crates/briccraft-character/README.md), [expedition rules](crates/briccraft-play/README.md) and [saved designs](crates/briccraft-design/README.md) are also independent of the UI and renderer. See [the architecture and current boundaries](docs/architecture.md) for interfaces and implementation details.

This is a creative construction sandbox. Horizontal worlds stream beyond the visible region; the current GPU working window is 96×72×96 occupancy cells, with one plate equal to 0.4 stud. Model imports retain uniform proportions. Mapped areas can explicitly fit their vertical scale while preserving horizontal georeferencing; every adjustment is reported. Elevation comes from a separate DEM source, whose resolution and survey age vary. OSM footprints and height tags do not supply facade scans, and a finite imported area is not a continuously streamed planet service.

Complete survival/crafting/mob/multiplayer gameplay, a complete manufacturer part database and a general solver for arbitrary articulated mechanisms remain future work. Cars, aircraft and boats can cross streamed regions; trains currently follow the classic island railway. See the architecture document for precise boundaries.

## License

MIT. Dependency and asset notices are in [NOTICE.md](NOTICE.md).
