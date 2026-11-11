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

#[test]
fn vehicle_tires_do_not_snap_to_the_ground() {
    let mut world = World::empty(1);
    let ground = world.place(Placement::new(34, 12, [20, 0, 20], 0)).unwrap();
    let ids = world
        .spawn_blueprint(Blueprint::Rover, [22, 1, 22], 0)
        .unwrap();
    let assembly = world.connected_assembly(ids[0], 256);
    assert_eq!(assembly.len(), ids.len());
    assert!(!assembly.contains(&ground));
}

#[test]
fn assembly_transform_preserves_ids_and_rolls_back_on_collision() {
    let mut world = World::empty(1);
    let ids = world
        .spawn_blueprint(Blueprint::Rover, [20, 0, 20], 0)
        .unwrap();
    let original: Vec<_> = ids
        .iter()
        .map(|id| world.instance(*id).unwrap().clone())
        .collect();
    let mut moved = original.clone();
    for instance in &mut moved {
        instance.position[0] += 12;
    }
    let mut history = History::default();
    assert_eq!(history.transform(&mut world, &moved).unwrap(), ids);
    assert_eq!(world.instance(ids[0]), Some(&moved[0]));
    history.undo(&mut world).unwrap();
    assert_eq!(world.instance(ids[0]), Some(&original[0]));
    history.redo(&mut world).unwrap();
    assert_eq!(world.instance(ids[0]), Some(&moved[0]));
    let revision = world.revision();
    moved[0].position = [95, 0, 95];
    assert!(history.transform(&mut world, &moved).is_err());
    assert_eq!(world.revision(), revision);
}

#[test]
fn blueprint_quarter_turns_match_renderer_positive_yaw() {
    for blueprint in Blueprint::ALL {
        let original = blueprint.placements([0; 3], 0);
        let rotated = blueprint.placements([0; 3], 1);
        let size = blueprint.size(0);
        for (before, after) in original.iter().zip(rotated) {
            // +90deg yaw maps a +Z offset to +X, and +X to -Z.
            assert_eq!(after.position[0], before.position[2]);
            assert_eq!(
                after.position[2],
                size[0] - before.position[0] - before.size()[0] as i32
            );
            assert_eq!(after.rotation, (before.rotation + 1) & 3);
        }
    }
}

#[test]
fn generated_rails_support_the_locomotive_gauge_all_around_the_route() {
    let world = World::generate(42);
    for distance in (0..(COASTAL_RAIL_LENGTH * 2.0) as i32).map(|n| n as f32 * 0.5) {
        let sample = coastal_rail_sample(distance);
        for side in [-1.0, 1.0] {
            let x = sample.position[0] + sample.tangent[2] * side * COASTAL_RAIL_GAUGE * 0.5;
            let z = sample.position[2] - sample.tangent[0] * side * COASTAL_RAIL_GAUGE * 0.5;
            assert!(
                world
                    .cell([x.floor() as i32, 19, z.floor() as i32])
                    .is_some_and(|instance| instance.part_id == 40),
                "missing rail at distance{distance}, side{side}, [{x},{z}]"
            );
        }
    }
}

#[test]
fn cottage_roof_has_outward_pitches_and_fully_supported_cap_and_chimney() {
    // Nominal solid height, excluding decorative studs and tiny molded edge recesses.
    // A slope's occupied cuboid alone cannot reveal a reversed pitch or a floating ridge.
    fn solid_top(part: &Placement, x: f32, z: f32) -> Option<f32> {
        let definition = briccraft_parts::part(part.part_id).unwrap();
        let size = part.size().map(f32::from);
        let px = x - part.position[0] as f32;
        let pz = z - part.position[2] as f32;
        if px < 0.0 || px >= size[0] || pz < 0.0 || pz >= size[2] {
            return None;
        }
        let height = if definition.shape == briccraft_parts::Shape::Slope {
            let dx = px - size[0] * 0.5;
            let dz = pz - size[2] * 0.5;
            let canonical_z = match part.rotation {
                0 => dz,
                1 => dx,
                2 => -dz,
                _ => -dx,
            };
            size[1] * (0.5 - canonical_z / f32::from(definition.size[2]))
        } else {
            size[1]
        };
        Some(part.position[1] as f32 + height)
    }

    let parts = Blueprint::House.placements([0; 3], 0);
    assert_eq!(Blueprint::House.size(0), [8, 29, 8]);
    let profile: Vec<_> = (0..8)
        .map(|z| {
            parts
                .iter()
                .filter_map(|part| solid_top(part, 3.5, z as f32 + 0.5))
                .fold(f32::NEG_INFINITY, f32::max)
        })
        .collect();
    for index in 0..3 {
        assert!(
            profile[index + 1] > profile[index] + 0.1,
            "front roof must rise toward its ridge: {profile:?}"
        );
        assert!(
            profile[6 - index] > profile[7 - index] + 0.1,
            "back roof must rise toward its ridge: {profile:?}"
        );
    }
    assert!((profile[3] - profile[4]).abs() < 0.001);
    for index in 0..4 {
        assert!((profile[index] - profile[7 - index]).abs() < 0.001);
    }

    // Every stud column below the raised cap and chimney must end at their underside.
    // This checks the actual slope plane; bounding-box occupancy would accept the old gaps.
    for raised in parts.iter().filter(|part| part.position[1] >= 22) {
        let size = raised.size().map(i32::from);
        for x in raised.position[0]..raised.position[0] + size[0] {
            for z in raised.position[2]..raised.position[2] + size[2] {
                let support = parts
                    .iter()
                    .filter(|part| part.position[1] < raised.position[1])
                    .filter_map(|part| solid_top(part, x as f32 + 0.5, z as f32 + 0.5))
                    .fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    (support - raised.position[1] as f32).abs() < 0.001,
                    "floating part {} at {:?}: support height {support}",
                    raised.part_id,
                    raised.position
                );
            }
        }
    }

    for rotation in 0..4 {
        let mut world = World::empty(1);
        let ids = world
            .spawn_blueprint(Blueprint::House, [24, 0, 24], rotation)
            .unwrap();
        assert_eq!(
            world.connected_assembly(ids[0], ids.len() + 1).len(),
            ids.len(),
            "all cottage parts must remain connected at rotation {rotation}"
        );
    }
}
