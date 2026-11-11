use crate::{BiomePreset, CameraMode, GameAttribution, GlobalPlacement, GlobalPos, Session};
use briccraft_world::Placement;
use glam::Vec3;
use std::rc::Rc;

fn flat() -> Session {
    Session::new_streamed(42, BiomePreset::Flat).unwrap()
}
fn settle(session: &mut Session) {
    for _ in 0..48 {
        session.tick(0.0);
    }
    assert!(
        session.streaming.as_ref().unwrap().pending_origin.is_none(),
        "{}",
        session.status
    );
}

#[test]
fn rebasing_uses_orbit_target_and_preserves_camera_global_position() {
    let mut session = flat();
    let original = session.world_origin();
    // Orbit camera itself can be near/outside the window; the subject determines its anchor.
    session.camera.distance = 150.0;
    session.camera.orbit(0.0, 0.0);
    settle(&mut session);
    assert_eq!(session.world_origin(), original);
    session.camera.target.x += 128.0;
    session.camera.position.x += 128.0;
    let global = session.global_camera_position();
    settle(&mut session);
    assert_ne!(session.world_origin(), original);
    assert_eq!(session.global_camera_position(), global);
    assert!((32.0..=64.0).contains(&session.camera.target.x));
    assert!(session.scene_generation > 0);
    assert!(session.stream_stats().unwrap().cached_chunks <= 64);
}

#[test]
fn fly_rebase_preserves_fractional_position_and_global_build_history() {
    let mut session = flat();
    let id = session
        .world
        .place(Placement::new(2, 0, [40, 18, 40], 0))
        .unwrap();
    session.capture_streamed_edits().unwrap();
    let key = session
        .streaming
        .as_ref()
        .unwrap()
        .snapshot
        .part_key(id)
        .unwrap();
    session.camera.mode = CameraMode::Fly;
    session.camera.position = Vec3::new(-110.25, 19.75, 140.5);
    let global = session.global_camera_position();
    settle(&mut session);
    assert_eq!(session.global_camera_position(), global);
    session.undo();
    assert!(session
        .streaming
        .as_ref()
        .unwrap()
        .stream
        .resolve(key)
        .unwrap()
        .is_none());
    session.redo();
    assert!(session
        .streaming
        .as_ref()
        .unwrap()
        .stream
        .resolve(key)
        .unwrap()
        .is_some());
}

#[test]
fn streamed_vehicle_survives_eviction_and_parks_with_global_identity() {
    let mut session = flat();
    session.spawn_blueprint("rover");
    session.activate_vehicle("auto");
    let active = session.active_vehicle.as_ref().expect("rover activated");
    let keys = active.part_keys.clone();
    let original_body = Rc::clone(&active.parts);
    let pivot = active.pivot;
    let original: Vec<_> = keys
        .iter()
        .map(|&key| {
            session
                .streaming
                .as_ref()
                .unwrap()
                .stream
                .resolve(key)
                .unwrap()
                .unwrap()
        })
        .collect();
    // Relocate continuously simulated pose to a distant flat destination, then exercise window
    // publication. The actual fixed-step drive tests separately verify propulsion and steering.
    session.active_vehicle.as_mut().unwrap().vehicle.position[0] += 224.0;
    session.active_vehicle.as_mut().unwrap().vehicle.position[2] += 96.0;
    settle(&mut session);
    let active = session.active_vehicle.as_ref().unwrap();
    assert!(Rc::ptr_eq(&original_body, &active.parts));
    assert_eq!(active.pivot, pivot);
    assert!(
        active.instance_ids.is_empty(),
        "original construction must be outside the new window"
    );
    assert!(active.vehicle.position[0] < 96.0);
    session.park_vehicle();
    assert!(session.active_vehicle.is_none(), "{}", session.status);
    assert_eq!(session.last_blueprint.len(), keys.len());
    for (&key, before) in keys.iter().zip(&original) {
        let after = session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .resolve(key)
            .unwrap()
            .unwrap();
        assert_eq!(after.position.x, before.position.x + 224);
        assert_eq!(after.position.z, before.position.z + 96);
    }
    session.undo();
    for (&key, &before) in keys.iter().zip(&original) {
        assert_eq!(
            session
                .streaming
                .as_ref()
                .unwrap()
                .stream
                .resolve(key)
                .unwrap(),
            Some(before)
        );
    }
}

#[test]
fn importing_cancels_without_geometry_or_metadata_and_commits_as_one_undo() {
    let mut session = flat();
    let original_info = session.world_info.clone();
    let before = session.world.len();
    let placements: Vec<_> = (0..120)
        .map(|i| GlobalPlacement::new(2, 2, GlobalPos::new(i * 2, 18, 48), 0))
        .collect();
    let source = GameAttribution {
        label: "Fixture model".into(),
        url: "https://example.org/model".into(),
        license: "CC0".into(),
    };
    session
        .begin_import(
            placements.clone(),
            "Test model".into(),
            vec![source.clone()],
        )
        .unwrap();
    session.world_info.import_report = Some("{\"fixture\":true}".into());
    session.tick(0.0);
    session.cancel_import();
    assert_eq!(session.world.len(), before);
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        0
    );
    assert_eq!(session.world_info, original_info);
    assert!(session.take_import_result().unwrap().is_err());
    session
        .begin_import(placements, "Test model".into(), vec![source.clone()])
        .unwrap();
    session.world_info.import_report = Some("{\"fixture\":true}".into());
    settle(&mut session);
    assert!(session.take_import_result().unwrap().is_ok());
    assert_eq!(session.world_info.attributions, vec![source]);
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        120
    );
    session.undo();
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        0
    );
    session.redo();
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        120
    );
}

#[test]
fn streamed_session_save_roundtrips_sources_origin_and_far_edits() {
    let mut session = flat();
    session.world_info.id = "world-fixture-uuid".into();
    session.world_info.import_report = Some("{\"source_height\":12.5}".into());
    session
        .world
        .place(Placement::new(2, 16, [44, 18, 44], 0))
        .unwrap();
    session.capture_streamed_edits().unwrap();
    session.camera.mode = CameraMode::Fly;
    session.camera.position = Vec3::new(300.25, 18.0, -30.5);
    settle(&mut session);
    let json = session.to_json().unwrap();
    assert!(
        json.len() < 3_000,
        "terrain must not be copied into a sparse save"
    );
    let mut loaded = Session::new(7);
    loaded.import_json(&json).unwrap();
    assert_eq!(loaded.world_info, session.world_info);
    assert_eq!(loaded.world_origin(), session.world_origin());
    assert_eq!(
        loaded.global_camera_position(),
        session.global_camera_position()
    );
    assert_eq!(
        loaded
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        1
    );
    assert!(loaded.scene_generation > 0);
}

#[test]
fn streamed_train_does_not_teleport_to_the_classic_railway() {
    let mut session = flat();
    session.spawn_blueprint("train");
    assert!(!session.last_blueprint.is_empty(), "{}", session.status);
    session.activate_vehicle("train");
    assert!(session.active_vehicle.is_none());
    assert!(session.status.contains("railway"));
}

#[test]
fn continuous_walk_and_flight_cross_multiple_windows_without_a_finite_edge() {
    for mode in [CameraMode::Walk, CameraMode::Fly] {
        let mut session = flat();
        session.camera.mode = mode;
        session.camera.position = Vec3::new(
            48.0,
            if mode == CameraMode::Walk { 9.4 } else { 16.0 },
            48.0,
        );
        session.camera.yaw = std::f32::consts::FRAC_PI_2;
        session.camera.pitch = 0.0;
        session.input.forward = true;
        session.input.fast = true;
        for _ in 0..600 {
            session.tick(1.0 / 60.0);
        }
        let global = session.global_camera_position();
        assert!(
            global[0] > 220.0,
            "{mode:?} stopped at {global:?}: {}",
            session.status
        );
        assert!(session.world_origin().x >= 160);
        assert!((32.0..=66.0).contains(&session.camera.position.x));
        if mode == CameraMode::Walk {
            assert!((session.camera.position.y - 9.4).abs() < 0.001);
        }
        assert!(session.stream_stats().unwrap().cached_chunks <= 64);
    }
}

#[test]
fn committed_import_cannot_be_reported_as_canceled_while_its_window_publishes() {
    let mut session = flat();
    session
        .begin_import(
            vec![GlobalPlacement::new(2, 1, GlobalPos::new(320, 18, 48), 0)],
            "Remote construction".into(),
            Vec::new(),
        )
        .unwrap();
    session.tick(0.0);
    let state = session.streaming.as_ref().unwrap();
    assert!(
        state.pending_import.is_none(),
        "the one-chunk transaction has committed"
    );
    assert!(
        state.import_focus.is_some(),
        "the new distant window is still being prepared"
    );
    assert_eq!(state.stream.sparse_edit_count(), 1);
    assert!(!session.cancel_import());
    assert!(session.status.contains("already committed"));
    assert!(session.take_import_result().is_none());
    settle(&mut session);
    assert!(session.take_import_result().unwrap().is_ok());
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        1
    );
    assert!(
        session.cancel_import(),
        "an idle session permits canceling a host conversion worker"
    );
}
