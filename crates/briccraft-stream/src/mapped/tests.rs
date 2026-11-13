use super::*;
use crate::{GlobalPlacement, GlobalPos, StreamWorld, WindowOrigin};

fn anchor() -> MapAnchor {
    MapAnchor {
        latitude: 50.0916,
        longitude: 18.2199,
        meters_per_stud: 2.0,
        origin: [64, 0, 64],
        reference_elevation_m: 180.0,
        meters_per_plate: 0.8,
        region_size: 128,
        terrain: true,
    }
}

fn region(x: i64, z: i64) -> MapRegion {
    MapRegion {
        bounds: [x, z, x + 128, z + 128],
        placements: (0..16)
            .flat_map(|row| {
                (0..16).map(move |column| {
                    GlobalPlacement::new(31, 12, GlobalPos::new(x + column * 8, 0, z + row * 8), 0)
                })
            })
            .collect(),
    }
}

#[test]
fn unknown_source_is_pending_and_never_generates_procedural_ground() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    let origin = WindowOrigin::new(16, 16);
    world.request_window(origin).unwrap();
    let progress = world.advance_generation(36).unwrap();
    assert_eq!(progress.generated, 0);
    assert_eq!(progress.pending, 36);
    assert!(!progress.complete);
    assert!(world.snapshot(origin).is_err());
    assert_eq!(world.terrain_at(64, 64).height_plates, 0);
    assert!(
        world
            .place_parts(&[GlobalPlacement::new(0, 1, GlobalPos::new(64, 1, 64), 0)])
            .is_err()
    );
    assert_eq!(world.sparse_edit_count(), 0);
}

#[test]
fn sources_are_atomic_order_independent_and_do_not_enter_construction_history() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    let source = region(0, 0);
    assert!(world.publish_map_region(source.clone()).unwrap());
    assert!(!world.can_undo());
    assert_eq!(world.sparse_edit_count(), 0);
    let mut reordered = source.clone();
    reordered.placements.reverse();
    assert!(!world.publish_map_region(reordered).unwrap());
    let before = world.to_json().unwrap();
    let mut overlap = region(128, 0);
    overlap.placements.push(overlap.placements[0]);
    assert!(world.publish_map_region(overlap).is_err());
    assert_eq!(world.to_json().unwrap(), before);
    let mut changed = source;
    changed.placements[0].color = 1;
    assert!(world.publish_map_region(changed).is_err());
    assert_eq!(world.to_json().unwrap(), before);
}

#[test]
fn cross_chunk_source_parts_keep_identity_from_both_sides_and_after_save_load() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    let mut source = region(0, 0);
    let crossing = GlobalPlacement::new(3, 1, GlobalPos::new(15, 1, 40), 0);
    source.placements.push(crossing);
    world.publish_map_region(source).unwrap();
    let first = world.prepare_window(WindowOrigin::new(0, 0)).unwrap();
    let id = first
        .world
        .instances()
        .find(|p| p.position == [15, 1, 40])
        .unwrap()
        .id;
    let key = first.part_key(id).unwrap();
    assert_eq!(world.resolve(key).unwrap(), Some(crossing));
    // The owner chunk is outside this window, yet its complete identity is indexed
    // into the neighboring chunk and recorded as a boundary part.
    let second = world.prepare_window(WindowOrigin::new(16, 0)).unwrap();
    assert!(second.boundary_parts.iter().any(|p| p.key == key));
    world.remove_parts(&[key]).unwrap();
    let saved = world.to_json().unwrap();
    let mut restored = StreamWorld::from_json(&saved).unwrap();
    assert_eq!(restored.map_anchor(), Some(&anchor()));
    assert_eq!(restored.resolve(key).unwrap(), None);
    let snapshot = restored.prepare_window(WindowOrigin::new(0, 0)).unwrap();
    assert!(
        snapshot
            .world
            .instances()
            .all(|p| p.position != [15, 1, 40])
    );
}

#[test]
fn arrival_preserves_user_history_and_pending_window_publishes_when_complete() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    world.publish_map_region(region(0, 0)).unwrap();
    world.prepare_window(WindowOrigin::new(16, 16)).unwrap();
    let building = GlobalPlacement::new(3, 2, GlobalPos::new(64, 1, 64), 0);
    let change = world.place_parts(&[building]).unwrap();
    world.request_window(WindowOrigin::new(64, 16)).unwrap();
    assert!(!world.advance_generation(36).unwrap().complete);
    world.publish_map_region(region(128, 0)).unwrap();
    assert!(world.advance_generation(36).unwrap().complete);
    world.snapshot(WindowOrigin::new(64, 16)).unwrap();
    world.undo().unwrap();
    assert_eq!(world.resolve(change.added_keys()[0]).unwrap(), None);
    assert_eq!(world.mapped_regions().len(), 2);
    world.redo().unwrap();
    assert_eq!(
        world.resolve(change.added_keys()[0]).unwrap(),
        Some(building)
    );
}

#[test]
fn cache_evicts_only_unedited_sources_and_edited_geometry_survives_many_regions() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    world.publish_map_region(region(0, 0)).unwrap();
    let first = world.prepare_window(WindowOrigin::new(16, 16)).unwrap();
    let key = first.visible_keys().next().unwrap().1;
    world.remove_parts(&[key]).unwrap();
    for tile in 1..38 {
        world.publish_map_region(region(tile * 128, 0)).unwrap();
        world
            .prepare_window(WindowOrigin::new(tile * 128 + 16, 16))
            .unwrap();
        assert!(world.mapped_regions().len() <= MAX_MAP_REGIONS);
    }
    assert!(world.map_region_loaded([0, 0, 128, 128]));
    assert_eq!(world.resolve(key).unwrap(), None);
    assert!(world.mapped_source_parts() <= MAX_MAP_SOURCE_PARTS);
    let mut restored = StreamWorld::from_json(&world.to_json().unwrap()).unwrap();
    assert_eq!(restored.resolve(key).unwrap(), None);
    restored.prepare_window(WindowOrigin::new(16, 16)).unwrap();
}

#[test]
fn demand_uses_euclidean_negative_regions_and_does_not_refetch_loaded_regions() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    world.publish_map_region(region(0, 0)).unwrap();
    assert!(world.map_demands([64.0, 0.0, 64.0]).unwrap().is_empty());
    assert!(
        world.map_demands([-1.0, 0.0, 64.0]).unwrap().is_empty(),
        "camera prediction alone must not fetch a source"
    );
    world
        .request_window(WindowOrigin::around(GlobalPos::new(-1, 0, 64)).unwrap())
        .unwrap();
    assert_eq!(
        world.map_demands([-1.0, 0.0, 64.0]).unwrap(),
        vec![[-128, 0, 0, 128]]
    );
    world
        .request_window(WindowOrigin::around(GlobalPos::new(-1, 0, -1)).unwrap())
        .unwrap();
    let positions = world.map_demands([-1.0, 0.0, -1.0]).unwrap();
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0], [-128, -128, 0, 0]);
    assert!(world.map_demands([f64::NAN, 0.0, 0.0]).is_err());
}

#[test]
fn direct_region_demands_match_all_intersecting_resident_chunks() {
    use std::collections::BTreeSet;
    for size in [96, 112, 128, 256, 512] {
        let mut anchor = anchor();
        anchor.region_size = size;
        let mut world = StreamWorld::new_mapped(42, anchor.clone(), 64).unwrap();
        world
            .publish_map_region(MapRegion {
                bounds: [0, 0, i64::from(size), i64::from(size)],
                placements: Vec::new(),
            })
            .unwrap();
        for x in [-512, -144, -112, -96, -16, 0, 16, 96, 112, 128, 512] {
            for z in [-512, -112, -96, -16, 0, 16, 96, 128, 512] {
                let origin = WindowOrigin::new(x, z);
                world.request_window(origin).unwrap();
                let focus = [x as f64 + 48.5, 0.0, z as f64 + 48.5];
                let actual = world.map_demands(focus).unwrap();
                let mut expected = BTreeSet::new();
                for view in [world.origin(), origin] {
                    for chunk in view.chunks().unwrap() {
                        let p = chunk.origin().unwrap();
                        let bounds = anchor.region_at(p.x, p.z).unwrap();
                        if !world.map_region_loaded(bounds) {
                            expected.insert(bounds);
                        }
                    }
                }
                assert_eq!(
                    actual.iter().copied().collect::<BTreeSet<_>>(),
                    expected,
                    "size={size}, origin={origin:?}"
                );
                let distance = |b: &[i64; 4]| {
                    let dx = b[0] as i128 + i128::from(size) / 2 - focus[0].floor() as i128;
                    let dz = b[1] as i128 + i128::from(size) / 2 - focus[2].floor() as i128;
                    dx * dx + dz * dz
                };
                assert!(
                    actual
                        .windows(2)
                        .all(|pair| distance(&pair[0]) <= distance(&pair[1]))
                );
            }
        }
    }
}

#[test]
fn source_arrival_still_validates_existing_player_cells_atomically() {
    // Normal edits cannot be placed into an unloaded region. Exercise the
    // defensive publication check directly, as needed for malformed saves.
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    let key = crate::PartKey::User(1);
    let part = GlobalPlacement::new(0, 2, GlobalPos::new(32, 0, 32), 0);
    world.edits.insert(key, Some(part));
    world
        .edit_chunks
        .entry(part.position.chunk())
        .or_default()
        .insert(key);
    let before = world.to_json().unwrap();
    assert!(matches!(
        world.publish_map_region(region(0, 0)),
        Err(crate::StreamError::Occupied(_))
    ));
    assert_eq!(world.to_json().unwrap(), before);
    assert_eq!(world.mapped_source_parts(), 0);
    assert!(world.mapped_regions().is_empty());
}

#[test]
fn source_arrival_preserves_non_overlapping_player_additions() {
    let mut world = StreamWorld::new_mapped(42, anchor(), 64).unwrap();
    let key = crate::PartKey::User(1);
    let part = GlobalPlacement::new(0, 2, GlobalPos::new(32, 3, 32), 0);
    world.edits.insert(key, Some(part));
    world
        .edit_chunks
        .entry(part.position.chunk())
        .or_default()
        .insert(key);
    world.publish_map_region(region(0, 0)).unwrap();
    assert_eq!(world.resolve(key).unwrap(), Some(part));
    assert_eq!(world.sparse_edit_count(), 1);
    assert!(!world.can_undo());
    let snapshot = world.prepare_window(WindowOrigin::default()).unwrap();
    assert!(snapshot.local_id(key).is_some());
}
