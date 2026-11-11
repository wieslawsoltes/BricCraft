# BricCraft render data

Backend-independent atlas packing for the browser's Skia renderer. This crate
has no FerroUI dependency, so geometry, streaming, and budget tests run without
building a native UI framework.

`SceneAtlas::build(&world, current_vehicle_ids, detached_parts)` produces two
opaque RGBA8 data images. Static cells use the active World's local coordinates.
Detached bodies are rebased to their own minimum and assigned a separate packed
ID range, so old body IDs cannot shadow new static IDs after streaming.

The global world may contain many regions; this crate only packs the supplied
96 × 72 × 96 active window. Empty coarse tiles use one occupancy texel each.
A 100,000-part unit-plate fixture uses 4,288,512 bytes across both atlas images.
The compact source-ID lookup uses another 800,000 bytes in that fixture.
`AtlasStats::staging_bytes` reports the owned pixel buffers plus this lookup;
World storage, temporary inputs, FerroUI image copies, and driver resources are
separate allocations.

## Integration

```rust
use briccraft_render_data::{AtlasError, SceneAtlas};
use briccraft_world::World;

fn main() -> Result<(), AtlasError> {
    let world = World::generate(42);
    let atlas = SceneAtlas::build(&world, &[], None)?;
    assert_eq!(atlas.stats.active_cells, 96 * 72 * 96);
    Ok(())
}
```

`briccraft-render` owns the image publication and immutable effect snapshots.
Packing does not require initializing Skia. `PixelAtlas` exposes its dimensions
and opaque pixel bytes for any backend; `PartLookup::get` maps a source ID to its
packed descriptor ID without allocating an array indexed by sparse source IDs.

## Limits

Each texture is 1,024 texels wide and at most 4,096 texels high. Four descriptor
texels per part leave a capacity of 1,048,575 packed parts. The smaller active
world and moving-body limits normally constrain the scene first. A moving body
contains at most 256 parts and fits within the active window's dimensions. Invalid
geometry and budget overruns return `AtlasError` before publishing textures.

## Validation and measurements

```sh
cargo test -p briccraft-render-data
cargo run --release -p briccraft-render-data --example benchmark
python3 crates/briccraft-render-data/tools/compare_atlas.py --baseline-ref 2c14d12
```

The benchmark uses two warmups and twenty measured rebuilds. It reports elapsed
latency and, on Linux, process CPU time to distinguish packing work from shared
runner scheduling delays. Scene construction is outside the timed section.
The comparison compiles the older packer from Git against the same domain code
and fixtures. `--reuse-libraries` can reuse existing release domain libraries
when another Cargo build holds the target-directory lock.

See [BENCHMARKS.md](BENCHMARKS.md) for measured results and their limits.
