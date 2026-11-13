# Snapshot and map streaming CPU measurements

`examples/snapshot_benchmark.rs` measures the CPU work associated with retaining
the current world, capturing unchanged edits, finding missing map regions, and
publishing one validated map source. It does not measure the renderer, network,
browser scheduling, or frame rate.

Run from the repository root:

```sh
cargo run --release -p briccraft-stream --example snapshot_benchmark -- 256
```

## 2026-11-13: shared immutable resident worlds

Baseline: `4ab92c9297cd1b7dfbeee4b3e20d8d0aa751bc23`. The identical example was
built against baseline and candidate world/stream crates with the same dependency
lockfile, Rust 1.100.0, optimization level 3, thin LTO, and aborting panics. Both
executables ran on one allowed logical CPU of an AMD EPYC 9V74 shared runner in
baseline/candidate/baseline order. Each operation has seven batches and sixteen
warm-up calls; batches use 256 operations, or 32 for source publication.

The deterministic fixture has 4,352 source parts within a 128 × 128-stud region
and 2,448 parts inside its 96 × 72 × 96-cell resident world.

| Operation | Baseline A median | Candidate median | Baseline B median |
| --- | ---: | ---: | ---: |
| Clone resident world | 61.58 µs | 0.009 µs | 66.44 µs |
| Capture unchanged world | 169.19 µs | 0.013 µs | 187.94 µs |
| Query missing map regions | 1.762 µs | 0.148 µs | 1.992 µs |
| Prepare and publish one source region | 674.40 µs | 549.03 µs | 717.20 µs |

The principal result is structural: cloning and unchanged captures now share
the **2,654,208-byte occupancy array** instead of copying it. Instance arrays
also remain shared. A validated edit detaches only the mutated world, and failed
or empty edits retain the original arrays. Capture uses a private baseline so a
caller replacing the publicly exposed snapshot world cannot fool the identity
check. Separately, map demand queries enumerate intersecting region rectangles
directly; source publication repeats source/player collision work only for
chunks that actually contain player additions.

These small operation timings are local microbenchmarks, not proportional game
or FPS improvements. Complete resident-window construction, map conversion,
atlas updates, and GPU rendering still perform their own work. Real mutations
still need to copy shared arrays before changing them. The retained snapshot
reduces copying on travel, source arrival, and saves without altering the save
format or removing transaction validation.

Raw batches and environment information:
[`benchmarks/2026-11-13-snapshot.json`](benchmarks/2026-11-13-snapshot.json).
