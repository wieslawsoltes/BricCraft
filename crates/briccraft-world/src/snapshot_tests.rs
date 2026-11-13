use crate::{BuildOp, History, Placement, World};

fn construction() -> World {
    let mut world = World::empty(42);
    world.place(Placement::new(0, 1, [12, 3, 12], 0)).unwrap();
    world.place(Placement::new(0, 2, [20, 3, 20], 0)).unwrap();
    world
}

#[test]
fn cloning_keeps_a_shared_immutable_snapshot_until_a_valid_edit() {
    let original = construction();
    let before = original.to_json().unwrap();
    let mut current = original.clone();
    assert!(original.shares_snapshot(&current));
    assert_eq!(original.occupancy().as_ptr(), current.occupancy().as_ptr());
    current.remove(0).unwrap();
    assert!(!original.shares_snapshot(&current));
    assert_ne!(original.occupancy().as_ptr(), current.occupancy().as_ptr());
    assert_eq!(original.to_json().unwrap(), before);
    assert!(original.cell([12, 3, 12]).is_some());
    assert!(current.cell([12, 3, 12]).is_none());
}

#[test]
fn failed_and_empty_transactions_do_not_detach_snapshots() {
    let original = construction();
    let mut current = original.clone();
    assert!(
        current
            .apply(&[
                BuildOp::Remove(0),
                BuildOp::Place(Placement::new(0, 3, [20, 3, 20], 0)),
            ])
            .is_err()
    );
    assert!(original.shares_snapshot(&current));
    current.apply(&[]).unwrap();
    current.restore(&[]).unwrap();
    assert!(original.shares_snapshot(&current));
}

#[test]
fn independent_worlds_and_changed_metadata_are_never_the_same_snapshot() {
    let original = construction();
    let rebuilt = World::from_json(&original.to_json().unwrap()).unwrap();
    assert_eq!(original.len(), rebuilt.len());
    assert!(!original.shares_snapshot(&rebuilt));
    let mut changed = original.clone();
    changed.seed += 1;
    assert!(!original.shares_snapshot(&changed));
    changed = original.clone();
    changed.revision = changed.revision.wrapping_add(1);
    assert!(!original.shares_snapshot(&changed));
    changed = original.clone();
    changed.size[0] -= 1;
    assert!(!original.shares_snapshot(&changed));
    changed = original.clone();
    changed.active -= 1;
    assert!(!original.shares_snapshot(&changed));
}

#[test]
fn history_replays_detach_only_the_world_being_edited() {
    let original = construction();
    let mut current = original.clone();
    let mut history = History::default();
    let mut moved = current.instance(0).unwrap().clone();
    moved.position[0] += 3;
    history.transform(&mut current, &[moved]).unwrap();
    let moved_snapshot = current.clone();
    assert!(history.undo(&mut current).unwrap());
    assert_eq!(current.to_json().unwrap(), original.to_json().unwrap());
    assert_ne!(
        current.to_json().unwrap(),
        moved_snapshot.to_json().unwrap()
    );
    let undone_snapshot = current.clone();
    assert!(history.redo(&mut current).unwrap());
    assert_eq!(
        current.to_json().unwrap(),
        moved_snapshot.to_json().unwrap()
    );
    assert_eq!(
        undone_snapshot.to_json().unwrap(),
        original.to_json().unwrap()
    );
}
