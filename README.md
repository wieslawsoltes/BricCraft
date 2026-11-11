# BricCraft

**Small bricks. Endless possibilities.** A browser construction sandbox built in Rust with declarative FerroUI controls, Skia GPU shaders and reusable simulation crates.

[Play BricCraft](https://wieslawsoltes.github.io/BricCraft/) · [Architecture](docs/architecture.md) · [Map and model imports](crates/briccraft-import/README.md) · [Build status](https://github.com/wieslawsoltes/BricCraft/actions)

## Build a world, then bring it to life

Create a collection of editable brick worlds: open meadows, alpine landscapes, desert dunes, coastlines, a flat construction studio, or the original harbor island. Travel through streamed terrain, turn real OpenStreetMap geometry into bricks, and bring imported 3D models into your constructions. Every generated and imported piece uses the same catalog, placement, picking and undo system.

- **Open worlds:** deterministic 16-stud chunks, signed 64-bit global coordinates, a floating origin, bounded terrain residency, sparse persistent edits and global undo across region changes.
- **World library:** named browser saves, automatic save before switching, separate copies, reversible archiving and migration of the original single-world save.
- **Real places:** an included Brandenburg Gate OSM extract; coordinate-based Overpass imports; local JSON extracts; mapped footprints, height tags, roads and water; persistent source attribution and reports identifying inferred dimensions.
- **Real 3D models:** GLB, embedded glTF and OBJ conversion in a background Web Worker. Scene transforms, vertex/material colors and embedded PNG/JPEG base-color textures are sampled into bricks. Open a model world or add a model to an existing mapped world using its meters-per-stud scale.
- **161 procedural parts and 32 colors:** bricks, plates, tiles, slopes, rounded pieces, arches, windows, doors, furniture, plants, liftarms, gears, axles, wheels, power components and transport pieces.
- **Eight construction blueprints:** trail rover, island hopper, coastal locomotive, lagoon sailboat, harbor cottage, lighthouse, windmill and bridge.
- **Construction tools:** face picking, placement preview with collision feedback, quarter-turn rotation, color selection, eyedropper, grouped undo/redo and a construction grid.
- **Working vehicles:** activate connected constructions, drive or fly with the keyboard, then park them back onto the construction grid. Cars, aircraft, trains and boats have distinct simulation models.
- **Reusable mechanical systems:** connected assembly assessment, motor/power checks, gear speed and torque ratios, locked-loop detection, transmission, differential, steering, spring/damper joints and a sampled rail path.
- **Three cameras:** orbit/pan/zoom for building, free flight and ground-following walking with jumping.
- **Native declarative UI:** responsive HUD, hotbar, searchable/filterable parts library, palette, blueprint chooser, touch controls, settings and controls guide.
- **Persistence:** transactional IndexedDB saves and versioned `.briccraft` file export/import with validation before replacement.
- **Skia rendering:** molded surfaces, real stud silhouettes, part seams, procedural mechanical shapes, directional lighting, contact shadows, water, atmosphere and animated vehicle assemblies.

## Real-world studio

Open **Worlds → Build a real place or 3D model**. **Brandenburg Gate** uses the included real OSM extract and works without contacting a map service. To import another place, enter latitude, longitude, area radius and meters per stud, then request that area once. Public Overpass availability varies; repeat or production use should use your own endpoint or locally prepared extracts. Panning never triggers network map requests.

Model conversion runs in a separate WASM worker and can be canceled. The complete placement is validated in bounded chunk batches and committed as one undo action. Choose **Add a model to this world** to combine source geometry with your current build; the selected construction target is its anchor. Source reports and attribution travel with `.briccraft` saves.

## Controls

| Action | Control |
| --- | --- |
| Place / remove a part | Click / right-click the world |
| Orbit or look | Drag the world |
| Pan | Shift-drag or middle-drag |
| Zoom | Mouse wheel / trackpad / two-finger pinch |
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

The current framework is pinned at `e1b36bb743a44277d27d63aa461ee671e54e9ec5`, version `0.5.0-preview.1`. The browser toolchain is pinned by that submodule: Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129.

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
  -p briccraft-game -p briccraft-stream -p briccraft-import \
  -p briccraft-import-worker -p briccraft-render-data
cargo check --locked -p briccraft-browser
python3 FerroUI/scripts/apps/sync-workspace.py --check .
```

`briccraft-render-data` supplies independent atlas tests and a reproducible packing benchmark. `briccraft-render` includes camera tests and a Skia shader smoke tool. The application calls FerroUI's `CustomEffect`, `EffectImage` and immutable effect APIs; all game UI uses `ferroui-view` builders and signals. The JavaScript host supplies browser services only.

## Workspace

Eleven reusable crates separate the part library, local world, streaming, map/model conversion, worker bindings, mechanics, game state, atlas packing, renderer, UI and browser lifecycle. See [the architecture and current boundaries](docs/architecture.md) for interfaces and implementation details.

This is a creative construction sandbox. Horizontal worlds stream beyond the visible region; the current GPU working window is 96×72×96 occupancy cells, with one plate equal to 0.4 stud. Source imports retain uniform proportions and must fit the 72-plate vertical budget. OSM footprints and tags are not facade scans or terrain elevation data.

Complete survival/crafting/mob/multiplayer gameplay, a complete manufacturer part database and a general solver for arbitrary articulated mechanisms remain future work. Cars, aircraft and boats can cross streamed regions; trains currently follow the classic island railway. See the architecture document for precise boundaries.

## License

MIT. Dependency and asset notices are in [NOTICE.md](NOTICE.md).
