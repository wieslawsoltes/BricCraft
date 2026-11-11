use crate::*;

#[test]
fn failed_transaction_is_atomic() {
    let mut world = World::empty(1);
    let placed = world.place(Placement::new(3, 0, [10, 10, 10], 0)).unwrap();
    let revision = world.revision();
    assert!(
        world
            .apply(&[
                BuildOp::Remove(placed),
                BuildOp::Place(Placement::new(2, 1, [95, 0, 0], 0))
            ])
            .is_err()
    );
    assert!(world.instance(placed).is_some());
    assert_eq!(world.revision(), revision);
}

#[test]
fn rotation_changes_footprint_and_rejects_overlap() {
    let mut world = World::empty(1);
    world.place(Placement::new(3, 0, [10, 0, 10], 1)).unwrap();
    assert!(world.cell([13, 2, 11]).is_some());
    assert!(world.cell([10, 2, 12]).is_none());
    assert!(world.place(Placement::new(0, 1, [13, 0, 11], 0)).is_err());
}

#[test]
fn history_preserves_identity_and_grouping() {
    let mut world = World::empty(1);
    let mut history = History::default();
    let ids = history
        .execute(
            &mut world,
            &[
                BuildOp::Place(Placement::new(2, 0, [0, 0, 0], 0)),
                BuildOp::Place(Placement::new(2, 1, [2, 0, 0], 0)),
            ],
        )
        .unwrap();
    assert_eq!(world.len(), 2);
    assert!(history.undo(&mut world).unwrap());
    assert_eq!(world.len(), 0);
    assert!(history.redo(&mut world).unwrap());
    assert_eq!(world.instance(ids[1]).unwrap().color, 1);
    world.remove(ids[0]).unwrap();
    assert_eq!(history.undo(&mut world), Err(BuildError::HistoryConflict));
    assert_eq!(world.len(), 1);
}

#[test]
fn raycast_accounts_for_plate_height_and_external_entry() {
    let mut world = World::empty(1);
    let id = world.place(Placement::new(2, 0, [20, 6, 20], 0)).unwrap();
    let hit = world
        .raycast([20.5, 15.0, 20.5], [0.0, -1.0, 0.0], 30.0)
        .unwrap();
    assert_eq!(hit.id, id);
    assert_eq!(hit.normal, [0, 1, 0]);
    assert!((hit.position[1] - 3.6).abs() < 0.001);
    assert!((hit.distance - 11.4).abs() < 0.001);
    assert_eq!(
        world
            .raycast([20.5, 3.0, -10.0], [0.0, 0.0, 1.0], 50.0)
            .unwrap()
            .normal,
        [0, 0, -1]
    );
    assert!(
        world
            .raycast([f32::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], 50.0)
            .is_none()
    );
}

#[test]
fn every_blueprint_places_atomically_at_every_orientation() {
    let mut errors = Vec::new();
    for blueprint in Blueprint::ALL {
        for rotation in 0..4 {
            let mut world = World::empty(1);
            let result = world.spawn_blueprint(blueprint, [30, 0, 30], rotation);
            match result {
                Ok(ids) => assert_eq!(ids.len(), blueprint.placements([0; 3], rotation).len()),
                Err(error) => errors.push(format!("{blueprint:?} rotation{rotation}: {error:?}")),
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn serialization_roundtrip_and_corrupt_input() {
    let mut world = World::empty(9);
    let id = world.place(Placement::new(3, 2, [10, 0, 10], 2)).unwrap();
    let restored = World::from_json(&world.to_json().unwrap()).unwrap();
    assert_eq!(restored.instance(id), world.instance(id));
    assert_eq!(restored.occupancy(), world.occupancy());
    let bad = world
        .to_json()
        .unwrap()
        .replace("\"color\":2", "\"color\":255");
    assert!(matches!(
        World::from_json(&bad),
        Err(BuildError::InvalidColor(255))
    ));
}

#[test]
fn seeded_island_is_deterministic_and_editable() {
    let a = World::generate(42);
    let b = World::generate(42);
    assert_eq!(a.occupancy(), b.occupancy());
    assert!(a.len() > 5_000);
    assert!(a.len() < 30_000);
    assert!((a.surface_height(52.0, 45.0) - 7.2).abs() < 0.01);
    assert!(a.to_json().unwrap().len() < 5_000_000);
}

#[test]
fn all_u64_seeds_are_supported_at_boundary_values() {
    for seed in [0, 1, 42, 1234, u64::MAX] {
        let world = World::generate(seed);
        assert!(world.len() > 5_000, "seed {seed}");
    }
}
