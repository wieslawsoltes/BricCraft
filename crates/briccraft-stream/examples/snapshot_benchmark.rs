//! Native CPU microbenchmark for the immutable resident-world publication path.
//! Run the identical source on both revisions; this does not measure GPU or FPS.
#![forbid(unsafe_code)]

use briccraft_stream::{
    GlobalPlacement, GlobalPos, MapAnchor, MapRegion, StreamWorld, WindowOrigin,
};
use std::hint::black_box;
use std::time::Instant;

fn anchor() -> MapAnchor {
    MapAnchor {
        latitude: 52.51628,
        longitude: 13.37770,
        meters_per_stud: 2.0,
        origin: [64, 0, 64],
        reference_elevation_m: 0.0,
        meters_per_plate: 0.8,
        region_size: 128,
        terrain: false,
    }
}

fn region() -> MapRegion {
    let mut placements = Vec::new();
    for z in (0..128).step_by(8) {
        for x in (0..128).step_by(8) {
            placements.push(GlobalPlacement::new(31, 12, GlobalPos::new(x, 0, z), 0));
        }
    }
    for z in (0..128).step_by(2) {
        for x in (0..128).step_by(2) {
            placements.push(GlobalPlacement::new(0, 2, GlobalPos::new(x, 1, z), 0));
        }
    }
    MapRegion {
        bounds: [0, 0, 128, 128],
        placements,
    }
}

fn sample(iterations: usize, mut run: impl FnMut()) -> serde_json::Value {
    for _ in 0..16 {
        run();
    }
    let mut ns_per_operation = Vec::new();
    for _ in 0..7 {
        let started = Instant::now();
        for _ in 0..iterations {
            run();
        }
        ns_per_operation.push(started.elapsed().as_nanos() as f64 / iterations as f64);
    }
    let mut ordered = ns_per_operation.clone();
    ordered.sort_by(f64::total_cmp);
    serde_json::json!({ "median_ns": ordered[ordered.len() / 2], "samples_ns": ns_per_operation })
}

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(256)
        .clamp(1, 10_000);
    let source = region();
    let mut stream = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    stream.publish_map_region(source.clone()).unwrap();
    let mut snapshot = stream.prepare_window(WindowOrigin::new(16, 16)).unwrap();
    let current = snapshot.world.clone();
    let retained = snapshot.world.clone();
    let clone_reuses_occupancy = retained.occupancy().as_ptr() == current.occupancy().as_ptr();
    stream.capture_edits(&mut snapshot, &current).unwrap();
    let capture_reuses_occupancy =
        snapshot.world.occupancy().as_ptr() == current.occupancy().as_ptr();
    let clone = sample(iterations, || {
        black_box(black_box(&current).clone());
    });
    let capture = sample(iterations, || {
        let change = stream
            .capture_edits(black_box(&mut snapshot), black_box(&current))
            .unwrap();
        assert!(change.is_empty());
        black_box(change);
    });
    stream.request_window(WindowOrigin::new(80, 80)).unwrap();
    let demands = sample(iterations, || {
        black_box(stream.map_demands(black_box([128.0, 0.0, 128.0])).unwrap());
    });
    let publication = sample(iterations.min(32), || {
        let mut candidate = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
        black_box(
            candidate
                .publish_map_region(black_box(source.clone()))
                .unwrap(),
        );
        black_box(candidate);
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "fixture": "dense synthetic map source, 128 × 128 studs",
            "source_parts": source.placements.len(),
            "resident_parts": current.len(),
            "resident_occupancy_bytes": std::mem::size_of_val(current.occupancy()),
            "iterations_per_sample": iterations,
            "publication_iterations_per_sample": iterations.min(32),
            "clone_reuses_occupancy": clone_reuses_occupancy,
            "unchanged_capture_reuses_occupancy": capture_reuses_occupancy,
            "world_clone": clone,
            "unchanged_capture": capture,
            "map_demands": demands,
            "source_publication": publication
        }))
        .unwrap()
    );
}
