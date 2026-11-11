use crate::*;
use briccraft_world::{Blueprint, BuildOp, Placement};

fn stream(preset: BiomePreset) -> StreamWorld {
    StreamWorld::new(42, preset, 36).unwrap()
}
fn flat_part(x: i64, z: i64) -> GlobalPlacement {
    GlobalPlacement::new(2, 0, GlobalPos::new(x, 18, z), 0)
}

#[test]
fn signed_chunk_lattice_and_far_rebase_preserve_exact_cells() {
    assert_eq!(ChunkCoord::containing(-1, -17), ChunkCoord::new(-1, -2));
    let origin = WindowOrigin::new(1_i64 << 56, -(1_i64 << 56));
    let position = origin.global_cell([51, 18, 37]).unwrap();
    assert_eq!(
        origin.local_position(position).unwrap(),
        [51.0, 7.2000003, 37.0]
    );
    let rebase = origin.rebase_around([78.25, 12.0, 43.5]).unwrap().unwrap();
    assert_eq!(rebase.origin.x, origin.x + 16);
    assert_eq!(rebase.translate([78.25, 12.0, 43.5]), [62.25, 12.0, 59.5]);
}

#[test]
fn atomic_snapshot_waits_for_budgeted_complete_window() {
    let mut s = stream(BiomePreset::Meadow);
    s.request_window(WindowOrigin::default()).unwrap();
    assert!(matches!(
        s.snapshot(WindowOrigin::default()),
        Err(StreamError::NotReady { remaining: 36 })
    ));
    let no_work = s
        .advance_generation_with_budget(GenerationBudget {
            max_chunks: 2,
            max_cells: CHUNK_CELLS - 1,
        })
        .unwrap();
    assert_eq!(no_work.generated, 0);
    for step in 1..=18 {
        let progress = s.advance_generation(2).unwrap();
        assert_eq!(progress.ready, step * 2);
        assert_eq!(progress.generated_cells, CHUNK_CELLS * 2);
    }
    let snapshot = s.snapshot(WindowOrigin::default()).unwrap();
    assert!(snapshot.world.len() > 1_000);
    assert_eq!(snapshot.world.occupancy().len(), 96 * 72 * 96);
}

#[test]
fn biome_chunks_are_deterministic_collision_free_and_chunk_contained() {
    for preset in BiomePreset::ALL {
        for seed in [0, 1, 42, u64::MAX] {
            for coord in [
                ChunkCoord::new(0, 0),
                ChunkCoord::new(-31, 47),
                ChunkCoord::new(1_i64 << 48, -(1_i64 << 48)),
            ] {
                let a = generation::generate(seed, preset, coord).unwrap();
                let b = generation::generate(seed, preset, coord).unwrap();
                assert_eq!(a.parts, b.parts);
                let origin = coord.origin().unwrap();
                let instances: Vec<_> = a
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(id, part)| {
                        let p = part.placement;
                        let x = p.position.x - origin.x;
                        let z = p.position.z - origin.z;
                        let size = p.size().unwrap();
                        assert!(x >= 0 && x + i64::from(size[0]) <= 16);
                        assert!(z >= 0 && z + i64::from(size[2]) <= 16);
                        briccraft_world::Instance {
                            id: id as u32,
                            part_id: p.part_id,
                            color: p.color,
                            position: [x as i32, p.position.y, z as i32],
                            rotation: p.rotation,
                        }
                    })
                    .collect();
                briccraft_world::World::from_instances(seed, &instances).unwrap();
            }
        }
    }
}

#[test]
fn sparse_builds_survive_eviction_rebase_and_global_history() {
    let mut s = stream(BiomePreset::Flat);
    let mut snapshot = s.prepare_window(WindowOrigin::default()).unwrap();
    let mut world = snapshot.world.clone();
    let local_id = world.place(Placement::new(2, 0, [44, 18, 46], 0)).unwrap();
    let change = s.capture_edits(&mut snapshot, &world).unwrap();
    let key = snapshot.part_key(local_id).unwrap();
    assert_eq!(change.added_keys(), vec![key]);
    assert!(s.capture_edits(&mut snapshot, &world).unwrap().is_empty());
    for offset in [16, 128, -160, 0] {
        let view = s.prepare_window(WindowOrigin::new(offset, 0)).unwrap();
        assert!(s.stats().cached_chunks <= 36);
        if offset == 0 {
            assert!(view.local_id(key).is_some());
        }
    }
    s.undo().unwrap().unwrap();
    assert!(s.resolve(key).unwrap().is_none());
    s.redo().unwrap().unwrap();
    assert_eq!(s.resolve(key).unwrap(), Some(flat_part(44, 46)));
    assert!(s.stats().cache_evictions > 0);
}

#[test]
fn removal_tombstones_and_restoration_remain_sparse() {
    let mut s = stream(BiomePreset::Flat);
    let mut snap = s.prepare_window(WindowOrigin::default()).unwrap();
    let id = snap.world.cell([40, 17, 40]).unwrap().id;
    let key = snap.part_key(id).unwrap();
    let mut current = snap.world.clone();
    let removed = current.remove(id).unwrap();
    s.capture_edits(&mut snap, &current).unwrap();
    assert_eq!(s.sparse_edit_count(), 1);
    current.restore(&[removed]).unwrap();
    s.capture_edits(&mut snap, &current).unwrap();
    assert!(s.resolve(key).unwrap().is_some());
    assert_eq!(s.sparse_edit_count(), 0);
}

#[test]
fn global_vehicle_transform_can_move_from_an_evicted_window_atomically() {
    let mut s = stream(BiomePreset::Flat);
    let rover: Vec<_> = Blueprint::Rover
        .placements([40, 18, 40], 1)
        .into_iter()
        .map(|p| GlobalPlacement::from_local(WindowOrigin::default(), p).unwrap())
        .collect();
    let keys = s.place_parts(&rover).unwrap().added_keys();
    s.prepare_window(WindowOrigin::new(320, -160)).unwrap();
    let moved: Vec<_> = rover
        .iter()
        .map(|p| GlobalPlacement {
            position: GlobalPos::new(p.position.x + 320, p.position.y, p.position.z - 160),
            ..*p
        })
        .collect();
    s.move_parts(&keys, &moved).unwrap();
    let view = s.snapshot(WindowOrigin::new(320, -160)).unwrap();
    for &key in &keys {
        assert!(view.local_id(key).is_some());
    }
    let invalid: Vec<_> = moved
        .iter()
        .map(|p| GlobalPlacement {
            position: GlobalPos::new(p.position.x, 0, p.position.z),
            ..*p
        })
        .collect();
    assert!(s.move_parts(&keys, &invalid).is_err());
    for (&key, &p) in keys.iter().zip(&moved) {
        assert_eq!(s.resolve(key).unwrap(), Some(p));
    }
    s.undo().unwrap();
    for (&key, &p) in keys.iter().zip(&rover) {
        assert_eq!(s.resolve(key).unwrap(), Some(p));
    }
}

#[test]
fn cross_boundary_parts_keep_whole_identity_without_destructive_clipping() {
    let mut s = stream(BiomePreset::Flat);
    let placement = GlobalPlacement::new(34, 2, GlobalPos::new(-8, 18, 32), 0);
    let key = s.place_parts(&[placement]).unwrap().added_keys()[0];
    let boundary = s.prepare_window(WindowOrigin::default()).unwrap();
    assert_eq!(boundary.boundary_parts, vec![GlobalPart { key, placement }]);
    let full = s.prepare_window(WindowOrigin::new(-32, 0)).unwrap();
    assert!(full.local_id(key).is_some());
    assert!(full.boundary_parts.is_empty());
    assert_eq!(s.resolve(key).unwrap(), Some(placement));
}

#[test]
fn bulk_import_is_budgeted_cancelable_and_one_global_undo() {
    let mut s = stream(BiomePreset::Flat);
    let parts: Vec<_> = (0..128).map(|i| flat_part(i * 2, 40)).collect();
    let original = s.to_json().unwrap();
    let mut discarded = s.begin_import(parts.clone()).unwrap();
    assert_eq!(
        s.advance_import(&mut discarded, GenerationBudget::chunks(1))
            .unwrap()
            .validated_chunks,
        1
    );
    drop(discarded);
    assert_eq!(s.to_json().unwrap(), original);
    let mut job = s.begin_import(parts).unwrap();
    while !job.progress().complete {
        s.advance_import(&mut job, GenerationBudget::chunks(2))
            .unwrap();
    }
    let keys = s.commit_import(job).unwrap().added_keys();
    assert_eq!(keys.len(), 128);
    assert_eq!(s.stats().undo_steps, 1);
    s.undo().unwrap();
    assert_eq!(s.sparse_edit_count(), 0);
    s.redo().unwrap();
    assert_eq!(s.sparse_edit_count(), 128);
}

#[test]
fn import_collision_and_vertical_bounds_never_publish_partial_data() {
    let mut s = stream(BiomePreset::Flat);
    let before = s.to_json().unwrap();
    let bad = GlobalPlacement::new(17, 3, GlobalPos::new(200, 60, 200), 0);
    assert!(s.begin_import(vec![flat_part(0, 0), bad]).is_err());
    let mut job = s
        .begin_import(vec![flat_part(0, 0), flat_part(0, 0)])
        .unwrap();
    assert!(
        s.advance_import(&mut job, GenerationBudget::chunks(4))
            .is_err()
    );
    assert_eq!(s.to_json().unwrap(), before);
    s.place_parts(&[flat_part(90, 90)]).unwrap();
    let mut stale = s.begin_import(vec![flat_part(0, 0)]).unwrap();
    s.place_parts(&[flat_part(80, 80)]).unwrap();
    assert_eq!(
        s.advance_import(&mut stale, GenerationBudget::chunks(1)),
        Err(StreamError::StaleSnapshot)
    );
}

#[test]
fn save_is_sparse_validated_and_regenerates_the_same_global_scene() {
    let mut s = stream(BiomePreset::Flat);
    let mut snapshot = s.prepare_window(WindowOrigin::new(-32, 16)).unwrap();
    let mut world = snapshot.world.clone();
    let remove_id = world.cell([42, 17, 42]).unwrap().id;
    world
        .apply(&[
            BuildOp::Remove(remove_id),
            BuildOp::Place(Placement::new(2, 18, [40, 21, 40], 0)),
        ])
        .unwrap();
    s.capture_edits(&mut snapshot, &world).unwrap();
    let json = s.to_json().unwrap();
    assert!(json.len() < 1_000);
    let mut loaded = StreamWorld::from_json(&json).unwrap();
    let origin = loaded.origin();
    let restored = loaded.prepare_window(origin).unwrap();
    let placements = |w: &briccraft_world::World| {
        let mut p: Vec<_> = w
            .instances()
            .map(|i| (i.part_id, i.color, i.position, i.rotation))
            .collect();
        p.sort();
        p
    };
    assert_eq!(placements(&world), placements(&restored.world));
    assert_eq!(loaded.to_json().unwrap(), json);
    let corrupted = json.replace("\"version\":1", "\"version\":999");
    assert!(StreamWorld::from_json(&corrupted).is_err());
}
