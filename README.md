# BricCraft

**Small bricks. Endless possibilities.** A browser construction sandbox built in Rust with declarative FerroUI controls, Skia GPU shaders and reusable simulation crates.

[Play BricCraft](https://wieslawsoltes.github.io/BricCraft/) · [Architecture](docs/architecture.md) · [Build status](https://github.com/wieslawsoltes/BricCraft/actions)

## Build a world, then bring it to life

Explore an editable stud-built island with a harbor village, lighthouse, terraced shoreline, woodland, a coastal railway and a building meadow. Place individual pieces, browse an extensive catalogue, or insert complete constructions assembled from the same editable parts.

- **161 procedural parts and 32 colors:** bricks, plates, tiles, slopes, rounded pieces, arches, windows, doors, furniture, plants, liftarms, gears, axles, wheels, power components and transport pieces.
- **Eight construction blueprints:** trail rover, island seaplane, coastal locomotive, lagoon sailboat, harbor cottage, lighthouse, windmill and bridge.
- **Construction tools:** face picking, placement preview with collision feedback, quarter-turn rotation, color selection, eyedropper, grouped undo/redo and a construction grid.
- **Working vehicles:** activate connected constructions, drive or fly with the keyboard, then park them back onto the construction grid. Cars, aircraft, trains and boats have distinct simulation models.
- **Reusable mechanical systems:** connected assembly assessment, motor/power checks, gear speed and torque ratios, locked-loop detection, transmission, differential, steering, spring/damper joints and a sampled rail path.
- **Three cameras:** orbit/pan/zoom for building, free flight and ground-following walking with jumping.
- **Native declarative UI:** responsive HUD, hotbar, searchable/filterable parts library, palette, blueprint chooser, touch controls, settings and controls guide.
- **Persistence:** transactional IndexedDB saves and versioned `.briccraft` file export/import with validation before replacement.
- **Skia rendering:** molded surfaces, real stud silhouettes, part seams, procedural mechanical shapes, directional lighting, contact shadows, water, atmosphere and animated vehicle assemblies.

## Controls

| Action | Control |
| --- | --- |
| Place / remove a part | Click / right-click the world |
| Orbit or look | Drag the world |
| Pan | Shift-drag or middle-drag |
| Zoom | Mouse wheel / trackpad |
| Pick part and color | Middle-click |
| Rotate selected part | R |
| Hotbar | 1–8 |
| Parts library | B or Tab |
| Change camera | F |
| Move | W A S D / arrow keys |
| Ascend / descend in flight | E / Q |
| Jump when walking | Space |
| Faster movement | Shift |
| Activate vehicle | V or Drive in the world panel |
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
CARGO_BUILD_JOBS=1 bash scripts/build-browser.sh
python3 -m http.server 8080 --directory target/browser/briccraft-browser
```

Open `http://localhost:8080`. GitHub Actions builds the optimized browser bundle, runs the framework's browser smoke check and publishes successful `main` builds to GitHub Pages.

```sh
cargo test --locked -p briccraft-parts -p briccraft-world -p briccraft-mechanics -p briccraft-game
cargo check --locked -p briccraft-browser
python3 FerroUI/scripts/apps/sync-workspace.py --check .
```

`briccraft-render` includes focused atlas/camera tests and a Skia shader smoke tool. The application calls FerroUI's `CustomEffect`, `EffectImage` and immutable effect APIs; all game UI uses `ferroui-view` builders and signals. The JavaScript host supplies browser services only.

## Workspace

The seven crates separate the part library, world, mechanics, game state, renderer, UI and browser lifecycle. See [the architecture and current boundaries](docs/architecture.md) for interfaces and implementation details.

This is a creative sandbox release. Complete survival/crafting/mob/multiplayer gameplay, an unbounded streaming world, a complete manufacturer part database and a general solver for arbitrary articulated mechanisms remain future work. The current world is 96×72×96 occupancy cells with one plate equal to 0.4 stud.

## License

MIT. Dependency and asset notices are in [NOTICE.md](NOTICE.md).
