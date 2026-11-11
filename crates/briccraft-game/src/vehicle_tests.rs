use crate::Session;
use briccraft_world::{BuildOp, World};

#[test]
fn streamed_rover_drives_after_blueprint_undo_and_redo() {
    let mut session = Session::new_streamed(42, crate::BiomePreset::Flat).unwrap();
    let baseline = session.world.len();
    session.spawn_blueprint("rover");
    let built = session.world.len();
    assert!(built > baseline);
    session.undo();
    assert_eq!(session.world.len(), baseline);
    session.redo();
    assert_eq!(session.world.len(), built);
    session.activate_vehicle("auto");
    let initial = session
        .active_vehicle
        .as_ref()
        .expect(&session.status)
        .vehicle
        .position;
    assert!(!session.paused);
    session.input.set_key("KeyW", true);
    // Reproduce the CI scenario at its reported 18 Hz display rate.
    for _ in 0..33 {
        session.tick(1.0 / 18.0);
    }
    session.input.set_key("KeyW", false);
    let vehicle = &session.active_vehicle.as_ref().unwrap().vehicle;
    let distance = (vehicle.position[0] - initial[0]).hypot(vehicle.position[2] - initial[2]);
    println!("streamed rover after undo/redo: {initial:?} -> {:?}; distance={distance}; speed={}; motors={}", vehicle.position, vehicle.speed, vehicle.motor_count);
    assert!(
        distance > 0.1,
        "held throttle must advance the restored rover"
    );
}

fn flat_session() -> Session {
    let mut session = Session::new(42);
    session.world = World::empty(42);
    for x in [16, 48] {
        for z in [16, 48] {
            session
                .world
                .place(briccraft_world::Placement::new(34, 12, [x, 6, z], 0))
                .unwrap();
        }
    }
    session
}

#[test]
fn every_vehicle_can_spawn_activate_and_park_transactionally() {
    for name in ["rover", "plane", "train", "boat"] {
        let mut session = Session::new(42);
        session.spawn_blueprint(name);
        assert!(
            !session.last_blueprint.is_empty(),
            "{name}: {}",
            session.status
        );
        let count = session.world.len();
        session.activate_vehicle("auto");
        assert!(
            session.active_vehicle.is_some(),
            "{name}: {}",
            session.status
        );
        session.park_vehicle();
        assert!(
            session.active_vehicle.is_none(),
            "{name}: {}",
            session.status
        );
        assert_eq!(session.world.len(), count);
        session.undo();
        assert_eq!(session.world.len(), count);
    }
}

#[test]
fn a_disconnected_or_unpowered_vehicle_does_not_drive() {
    let mut session = flat_session();
    session.spawn_blueprint("rover");
    let battery = session
        .last_blueprint
        .iter()
        .copied()
        .find(|id| {
            session
                .world
                .instance(*id)
                .is_some_and(|p| p.part_id == 160)
        })
        .unwrap();
    session
        .history
        .execute(&mut session.world, &[BuildOp::Remove(battery)])
        .unwrap();
    session.activate_vehicle("auto");
    assert!(session.active_vehicle.is_none());
    assert!(session.status.contains("battery"));
}

#[test]
fn driving_changes_pose_without_rewriting_world_geometry() {
    let mut session = flat_session();
    session.spawn_blueprint("rover");
    session.activate_vehicle("auto");
    let revision = session.world.revision();
    let position = session.active_vehicle.as_ref().unwrap().vehicle.position;
    session.input.forward = true;
    for _ in 0..120 {
        session.tick(1.0 / 60.0);
    }
    let current = session.active_vehicle.as_ref().unwrap().vehicle.position;
    assert!(current[2] > position[2] + 0.1);
    assert_eq!(revision, session.world.revision());
}

#[test]
fn a_default_boat_launches_on_water_and_can_move_and_park() {
    let mut session = Session::new(42);
    session.spawn_blueprint("boat");
    session.activate_vehicle("auto");
    let active = session.active_vehicle.as_ref().expect("boat activated");
    let initial = active.vehicle.position;
    assert!(
        session
            .world
            .surface_height_excluding(initial[0], initial[2], &active.instance_ids)
            < briccraft_world::SEA_LEVEL,
        "boat spawned on land at {initial:?}"
    );
    session.input.forward = true;
    for _ in 0..180 {
        session.tick(1.0 / 60.0);
    }
    let position = session.active_vehicle.as_ref().unwrap().vehicle.position;
    assert!(
        (position[2] - initial[2]).abs() > 1.0,
        "boat did not move: {position:?}"
    );
    session.park_vehicle();
    assert!(session.active_vehicle.is_none(), "{}", session.status);
}

#[test]
fn a_default_train_starts_on_its_track_without_teleporting() {
    let mut session = Session::new(42);
    session.spawn_blueprint("train");
    session.activate_vehicle("auto");
    let initial = session
        .active_vehicle
        .as_ref()
        .expect("train activated")
        .vehicle
        .position;
    session.tick(1.0 / 60.0);
    let position = session.active_vehicle.as_ref().unwrap().vehicle.position;
    let displacement = (0..3)
        .map(|i| (position[i] - initial[i]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(
        displacement < 0.25,
        "train teleported {displacement} studs from {initial:?} to {position:?}"
    );
    session.input.forward = true;
    for _ in 0..120 {
        session.tick(1.0 / 60.0);
    }
    let position = session.active_vehicle.as_ref().unwrap().vehicle.position;
    assert!((position[2] - initial[2]).abs() > 0.25);
    session.park_vehicle();
    assert!(session.active_vehicle.is_none(), "{}", session.status);
}

#[test]
fn park_undo_keeps_vehicle_selection_usable() {
    let mut session = flat_session();
    session.spawn_blueprint("rover");
    let original = session.last_blueprint.clone();
    session.activate_vehicle("auto");
    session.park_vehicle();
    session.undo();
    assert!(original
        .iter()
        .all(|id| session.world.instance(*id).is_some()));
    session.hit = None;
    session.activate_vehicle("auto");
    assert!(session.active_vehicle.is_some(), "{}", session.status);
}

#[test]
fn rotated_vehicles_drive_forward_and_park_without_double_rotation() {
    for rotation in 0..4 {
        let mut session = flat_session();
        session.rotation = rotation;
        session.spawn_blueprint("rover");
        let original_ids = session.last_blueprint.clone();
        let original: Vec<_> = original_ids
            .iter()
            .map(|id| session.world.instance(*id).unwrap().clone())
            .collect();
        session.activate_vehicle("auto");
        let active = session
            .active_vehicle
            .as_ref()
            .expect("rotated rover activates");
        let initial = active.vehicle.position;
        let heading = rotation as f32 * std::f32::consts::FRAC_PI_2;
        assert!((active.vehicle.yaw - heading).abs() < 0.001);
        assert!(active.render_yaw().abs() < 0.001);
        session.park_vehicle();
        assert!(session.active_vehicle.is_none(), "{}", session.status);
        for before in &original {
            assert_eq!(session.world.instance(before.id), Some(before));
        }
        session.activate_vehicle("auto");
        session.input.forward = true;
        for _ in 0..60 {
            session.tick(1.0 / 60.0);
        }
        let position = session.active_vehicle.as_ref().unwrap().vehicle.position;
        let forward_distance =
            (position[0] - initial[0]) * heading.sin() + (position[2] - initial[2]) * heading.cos();
        assert!(
            forward_distance > 0.1,
            "rotation{rotation}: {initial:?} -> {position:?}"
        );
        session.park_vehicle();
        assert!(
            session.active_vehicle.is_none(),
            "rotation{rotation}: {}",
            session.status
        );
        let mut selected = session.last_blueprint.clone();
        selected.sort_unstable();
        let mut expected = original_ids;
        expected.sort_unstable();
        assert_eq!(selected, expected);
    }
}

#[test]
fn a_boat_stops_at_the_world_edge_and_remains_parkable() {
    let mut session = Session::new(42);
    session.spawn_blueprint("boat");
    session.activate_vehicle("auto");
    session.input.forward = true;
    for _ in 0..600 {
        session.tick(1.0 / 60.0);
    }
    let active = session.active_vehicle.as_ref().unwrap();
    assert!(active.vehicle.position[2] + active.half_extents_xz[1] <= 96.0001);
    assert!(active.vehicle.speed.abs() < 0.001);
    assert!(session.status.contains("World edge"));
    session.park_vehicle();
    assert!(session.active_vehicle.is_none(), "{}", session.status);
}

#[test]
fn boundary_clamp_accounts_for_the_rotated_assembly_footprint() {
    let mut session = flat_session();
    session.spawn_blueprint("rover");
    session.activate_vehicle("auto");
    let active = session.active_vehicle.as_mut().unwrap();
    active.vehicle.position = [95.5, 3.0, 95.5];
    active.vehicle.yaw = 0.7;
    active.vehicle.speed = 8.0;
    session.tick(1.0 / 60.0);
    let active = session.active_vehicle.as_ref().unwrap();
    let (sin, cos) = active.render_yaw().sin_cos();
    let [x, z] = active.half_extents_xz;
    assert!(active.vehicle.position[0] + x * cos.abs() + z * sin.abs() <= 96.0001);
    assert!(active.vehicle.position[2] + x * sin.abs() + z * cos.abs() <= 96.0001);
    assert_eq!(active.vehicle.speed, 0.0);
}
