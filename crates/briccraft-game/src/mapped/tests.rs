use super::*;
use crate::{GlobalPlacement, GlobalPos};
use briccraft_world::Placement;

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
fn session() -> Session {
    Session::new_mapped(
        42,
        anchor(),
        region(0, 0),
        "Racibórz".into(),
        vec![GameAttribution {
            label: "© OpenStreetMap contributors".into(),
            url: "https://www.openstreetmap.org/copyright".into(),
            license: "ODbL-1.0".into(),
        }],
        true,
    )
    .unwrap()
}

#[test]
fn mapped_walk_waits_safely_for_source_and_resumes_across_the_same_world() {
    let mut session = session();
    assert_eq!(session.camera.mode, CameraMode::Walk);
    assert_eq!(session.world_origin(), WindowOrigin::new(16, 16));
    let initial_id = session.world_info.id.clone();
    session.camera.yaw = std::f32::consts::FRAC_PI_2;
    session.camera.pitch = 0.0;
    session.input.forward = true;
    session.input.fast = true;
    for _ in 0..900 {
        session.tick(1.0 / 60.0);
    }
    let pending = session.global_camera_position();
    assert!(session.map_waiting());
    assert!(pending[0] >= 110.0 && pending[0] <= 120.1, "{pending:?}");
    assert!(
        (pending[1] - 2.6).abs() < 0.001,
        "must not fall into missing source: {pending:?}"
    );
    assert!(session.map_demands().unwrap().contains(&[128, 0, 256, 128]));
    let camera = session.global_camera_position();
    session.append_map_region(region(128, 0)).unwrap();
    assert_eq!(
        session.global_camera_position(),
        camera,
        "arrival must not teleport"
    );
    for _ in 0..300 {
        session.tick(1.0 / 60.0);
    }
    assert!(
        session.global_camera_position()[0] > 190.0,
        "{:?}",
        session.global_camera_position()
    );
    assert!((session.global_camera_position()[1] - 2.6).abs() < 0.001);
    assert_eq!(session.world_info.id, initial_id);
    assert!(
        !session.can_undo(),
        "source generation is not a player action"
    );
}

#[test]
fn mapped_spawn_clears_the_walking_footprint_and_headroom_before_publication() {
    // The pin's centre column is low ground, but its circular footprint reaches
    // the adjacent wall or overhang. A point-only spawn would jump onto its roof.
    for layers in [&[1, 4, 7, 10][..], &[6][..]] {
        let mut source = region(0, 0);
        for &y in layers {
            source
                .placements
                .push(GlobalPlacement::new(0, 1, GlobalPos::new(63, y, 64), 0));
        }
        let mut session = Session::new_mapped(
            42,
            anchor(),
            source,
            "Clear walking spawn".into(),
            vec![],
            true,
        )
        .unwrap();
        let before = session.global_camera_position();
        assert!(
            (before[1] - 2.6).abs() < 0.001,
            "prefer nearby ground: {before:?}"
        );
        assert_ne!([before[0], before[2]], [64.0, 64.0]);
        session.tick(1.0 / 60.0);
        let after = session.global_camera_position();
        assert!(
            (after[1] - before[1]).abs() < 0.001,
            "no roof recovery: {after:?}"
        );
        assert_eq!([after[0], after[2]], [before[0], before[2]]);
        assert!(session.camera.grounded());
    }
}

#[test]
fn map_append_and_saved_reopen_preserve_construction_identity_and_georeferencing() {
    let mut session = session();
    let id = session
        .world
        .place(Placement::new(3, 1, [44, 1, 44], 0))
        .unwrap();
    session.capture_streamed_edits().unwrap();
    let key = session
        .streaming
        .as_ref()
        .unwrap()
        .snapshot
        .part_key(id)
        .unwrap();
    let placement = session
        .streaming
        .as_ref()
        .unwrap()
        .stream
        .resolve(key)
        .unwrap();
    session.append_map_region(region(128, 0)).unwrap();
    session.undo();
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .resolve(key)
            .unwrap(),
        None
    );
    session.redo();
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .resolve(key)
            .unwrap(),
        placement
    );
    let saved = session.to_json().unwrap();
    let mut reopened = Session::new(7);
    reopened.import_json(&saved).unwrap();
    assert_eq!(reopened.map_anchor(), Some(&anchor()));
    assert_eq!(reopened.mapped_regions(), session.mapped_regions());
    assert_eq!(reopened.world_info, session.world_info);
    assert_eq!(
        reopened.global_camera_position(),
        session.global_camera_position()
    );
    assert_eq!(
        reopened
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .resolve(key)
            .unwrap(),
        placement
    );
    assert_eq!(reopened.camera.mode, CameraMode::Walk);
}

#[test]
fn malformed_or_partial_regions_leave_live_geography_and_camera_unchanged() {
    let mut session = session();
    let before = session.to_json().unwrap();
    let mut invalid = region(128, 0);
    invalid.placements[0].position.x = 127;
    assert!(session.append_map_region(invalid).is_err());
    assert_eq!(session.to_json().unwrap(), before);
    let mut bad_anchor = anchor();
    bad_anchor.meters_per_plate = f64::NAN;
    assert!(
        Session::new_mapped(42, bad_anchor, region(0, 0), "Invalid".into(), vec![], true).is_err()
    );
    assert!(Session::new_mapped(
        42,
        anchor(),
        region(128, 0),
        "Wrong initial area".into(),
        vec![],
        true
    )
    .is_err());
}

#[test]
fn mapped_vehicle_waits_for_loaded_ground_then_resumes_without_losing_its_parts() {
    let mut session = Session::new_mapped(
        42,
        anchor(),
        region(0, 0),
        "Mapped road".into(),
        vec![],
        false,
    )
    .unwrap();
    session.spawn_blueprint("rover");
    session.activate_vehicle("auto");
    let active = session.active_vehicle.as_mut().expect("rover activated");
    active.vehicle.yaw = std::f32::consts::FRAC_PI_2;
    let keys = active.part_keys.clone();
    let body = std::rc::Rc::clone(&active.parts);
    session.input.forward = true;
    for _ in 0..1_800 {
        session.tick(1.0 / 60.0);
    }
    let active = session.active_vehicle.as_ref().unwrap();
    let pending = session
        .world_origin()
        .global_position(active.vehicle.position);
    assert!(session.map_waiting(), "{pending:?}: {}", session.status);
    assert!((110.0..=120.1).contains(&pending[0]), "{pending:?}");
    assert!(
        (pending[1] - 0.4).abs() < 0.01,
        "vehicle must retain loaded ground support: {pending:?}"
    );
    assert_eq!(active.vehicle.speed, 0.0);
    assert!(std::rc::Rc::ptr_eq(&body, &active.parts));
    session.append_map_region(region(128, 0)).unwrap();
    for _ in 0..900 {
        session.tick(1.0 / 60.0);
    }
    let active = session.active_vehicle.as_ref().unwrap();
    let traveled = session
        .world_origin()
        .global_position(active.vehicle.position);
    assert!(traveled[0] > 145.0, "{traveled:?}");
    assert!((traveled[1] - 0.4).abs() < 0.01);
    assert_eq!(active.part_keys, keys);
    assert!(std::rc::Rc::ptr_eq(&body, &active.parts));
    // Backing away remains possible when the next unavailable region is reached.
    session.input.forward = false;
    session.input.backward = true;
    let before = traveled[0];
    for _ in 0..240 {
        session.tick(1.0 / 60.0);
    }
    let active = session.active_vehicle.as_ref().unwrap();
    let after = session
        .world_origin()
        .global_position(active.vehicle.position);
    assert!(
        after[0] < before - 1.0,
        "reverse must remain usable at a pending edge"
    );
}

#[test]
fn returning_to_the_loaded_view_withdraws_an_obsolete_map_demand() {
    let mut session = session();
    session.camera.position.x = 84.0;
    session.tick(0.0);
    assert!(session.map_waiting());
    assert!(session.map_demands().unwrap().contains(&[128, 0, 256, 128]));
    session.camera.position.x = 48.0;
    let before = session.global_camera_position();
    session.tick(0.0);
    assert!(!session.map_waiting());
    assert!(session.map_demands().unwrap().is_empty());
    assert_eq!(session.global_camera_position(), before);
}

#[test]
fn mapped_boat_workshop_and_activation_report_the_same_water_support_boundary() {
    let mut session = session();
    session.spawn_blueprint("boat");
    assert!(
        !session.last_blueprint.is_empty(),
        "boat remains an editable construction: {}",
        session.status
    );
    let before = session.world.len();
    let workshop = session.workshop_status();
    assert!(!workshop.ready);
    assert!(workshop.detail.contains("island sea"));
    session.activate_vehicle("auto");
    assert!(session.active_vehicle.is_none());
    assert_eq!(session.status, workshop.detail);
    assert_eq!(session.world.len(), before);
}
