use crate::{BiomePreset, GlobalPlacement, GlobalPos, StreamError, StreamWorld, WindowOrigin};
use briccraft_world::{Instance, Placement, World};

fn stream() -> StreamWorld {
    StreamWorld::new(42, BiomePreset::Flat, 36).unwrap()
}

#[test]
fn unchanged_capture_retains_shared_storage_without_history_or_identity_changes() {
    let mut stream = stream();
    let mut snapshot = stream.prepare_window(WindowOrigin::default()).unwrap();
    let current = snapshot.world.clone();
    let occupancy = snapshot.world.occupancy().as_ptr();
    let saved = stream.to_json().unwrap();
    let keys = snapshot.visible_keys().collect::<Vec<_>>();
    for _ in 0..128 {
        assert!(
            stream
                .capture_edits(&mut snapshot, &current)
                .unwrap()
                .is_empty()
        );
        assert_eq!(snapshot.world.occupancy().as_ptr(), occupancy);
        assert!(snapshot.world.shares_snapshot(&current));
    }
    assert_eq!(stream.to_json().unwrap(), saved);
    assert_eq!(snapshot.visible_keys().collect::<Vec<_>>(), keys);
    assert!(!stream.can_undo());
}

#[test]
fn replacing_the_public_world_cannot_replace_the_private_capture_baseline() {
    let mut stream = stream();
    let mut snapshot = stream.prepare_window(WindowOrigin::default()).unwrap();
    let mut parts = snapshot.world.instances().cloned().collect::<Vec<_>>();
    let id = parts.len() as u32;
    parts.push(Instance {
        id,
        part_id: 0,
        color: 2,
        position: [44, 18, 46],
        rotation: 0,
    });
    // The replacement has the same local revision as its unrelated baseline.
    let current = World::from_instances(42, &parts).unwrap();
    assert_eq!(snapshot.world.revision(), current.revision());
    snapshot.world = current.clone();
    let change = stream.capture_edits(&mut snapshot, &current).unwrap();
    assert_eq!(change.changes.len(), 1);
    let key = snapshot.part_key(id).unwrap();
    assert_eq!(change.added_keys(), vec![key]);
    assert_eq!(
        stream.resolve(key).unwrap(),
        Some(GlobalPlacement::new(0, 2, GlobalPos::new(44, 18, 46), 0))
    );
    assert!(stream.can_undo());
    assert!(
        stream
            .capture_edits(&mut snapshot, &current)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mutable_public_world_is_repaired_even_when_current_is_unchanged() {
    let mut stream = stream();
    let mut snapshot = stream.prepare_window(WindowOrigin::default()).unwrap();
    let current = snapshot.world.clone();
    snapshot
        .world
        .place(Placement::new(0, 2, [44, 18, 46], 0))
        .unwrap();
    assert!(
        stream
            .capture_edits(&mut snapshot, &current)
            .unwrap()
            .is_empty()
    );
    assert!(snapshot.world.shares_snapshot(&current));
    assert!(!stream.can_undo());
}

#[test]
fn stale_stream_revision_is_rejected_before_the_shared_snapshot_fast_path() {
    let mut stream = stream();
    let mut snapshot = stream.prepare_window(WindowOrigin::default()).unwrap();
    let current = snapshot.world.clone();
    stream
        .place_parts(&[GlobalPlacement::new(0, 2, GlobalPos::new(44, 18, 46), 0)])
        .unwrap();
    assert!(matches!(
        stream.capture_edits(&mut snapshot, &current),
        Err(StreamError::StaleSnapshot)
    ));
}
