use crate::{BiomePreset, CameraMode, GlobalPlacement, GlobalPos, Session};
use briccraft_play::{ChangeValue, STARTER_STUDS};
use briccraft_world::{BuildOp, Placement, World};

fn spend(session: &mut Session, amount: u64) {
    let permit = session
        .gameplay
        .approve(ChangeValue {
            added: amount,
            removed: 0,
        })
        .unwrap();
    session.gameplay.commit(permit).unwrap();
}
fn progress(session: &Session, id: &str) -> u64 {
    session
        .gameplay
        .objectives()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
        .progress
}

#[test]
fn unaffordable_and_colliding_real_targets_change_neither_world_nor_wallet() {
    let mut game = Session::with_world(World::empty(42));
    game.set_expedition(true);
    spend(&mut game, STARTER_STUDS);
    let placement = Placement::new(3, 0, [10, 5, 10], 0);
    assert!(game.world.can_place(&placement).is_ok());
    assert!(game
        .execute_construction(&[BuildOp::Place(placement)])
        .unwrap_err()
        .contains("more studs"));
    assert!(game.world.is_empty());
    assert_eq!(game.gameplay.studs(), 0);
    assert!(!game.can_undo());
    let mut game = Session::with_world(World::empty(42));
    game.set_expedition(true);
    game.execute_construction(&[BuildOp::Place(placement)])
        .unwrap();
    let before = game.gameplay.studs();
    let revision = game.world.revision();
    assert!(game
        .execute_construction(&[BuildOp::Place(placement)])
        .is_err());
    assert_eq!(game.gameplay.studs(), before);
    assert_eq!(game.world.revision(), revision);
    assert_eq!(progress(&game, "build"), 1);
}

#[test]
fn classic_undo_redo_reverses_costs_without_replaying_objective_rewards() {
    let mut game = Session::with_world(World::empty(42));
    game.set_expedition(true);
    let operations: Vec<_> = (0..24)
        .map(|x| BuildOp::Place(Placement::new(20, 0, [x, 0, 0], 0)))
        .collect();
    game.execute_construction(&operations).unwrap();
    let built_balance = STARTER_STUDS - 24 + 180;
    assert_eq!(game.gameplay.studs(), built_balance);
    for _ in 0..4 {
        game.undo();
        assert!(game.world.is_empty());
        assert_eq!(game.gameplay.studs(), built_balance + 24);
        game.redo();
        assert_eq!(game.world.len(), 24);
        assert_eq!(game.gameplay.studs(), built_balance);
    }
    assert_eq!(progress(&game, "build"), 24);
}

#[test]
fn harvested_parts_cannot_be_restored_after_their_refund_was_spent() {
    let mut world = World::empty(42);
    let id = world.place(Placement::new(20, 0, [4, 0, 4], 0)).unwrap();
    let mut game = Session::with_world(world);
    game.set_expedition(true);
    game.execute_construction(&[BuildOp::Remove(id)]).unwrap();
    assert_eq!(game.gameplay.studs(), STARTER_STUDS + 1);
    let balance = game.gameplay.studs();
    spend(&mut game, balance);
    game.undo();
    assert!(game.world.is_empty());
    assert_eq!(game.gameplay.studs(), 0);
    assert!(game.status.contains("more studs"));
    assert!(game.can_undo());
}

#[test]
fn a_failed_global_capture_rolls_back_geometry_before_any_charge_or_reward() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.streaming
        .as_mut()
        .unwrap()
        .stream
        .place_parts(&[GlobalPlacement::new(34, 0, GlobalPos::new(-16, 18, 20), 0)])
        .unwrap();
    game.refresh_streamed_world().unwrap();
    assert!(!game
        .streaming
        .as_ref()
        .unwrap()
        .snapshot
        .boundary_parts
        .is_empty());
    game.set_expedition(true);
    let before = game.gameplay.studs();
    let count = game.world.len();
    let placement = Placement::new(20, 0, [0, 18, 20], 0);
    assert!(game.world.can_place(&placement).is_ok());
    assert!(game
        .execute_construction(&[BuildOp::Place(placement)])
        .is_err());
    assert_eq!(game.world.len(), count);
    assert!(!game.world.occupied([0, 18, 20]));
    assert_eq!(game.gameplay.studs(), before);
    assert_eq!(progress(&game, "build"), 0);
}

#[test]
fn streamed_rover_purchase_history_activation_and_drive_progress_are_real() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.set_expedition(true);
    let quote = game.blueprint_quote("rover").unwrap();
    assert!(quote.affordable);
    assert!(quote.charged);
    let baseline = game.world.len();
    game.spawn_blueprint("rover");
    assert!(game.world.len() > baseline, "{}", game.status);
    let balance = game.gameplay.studs();
    let count = game.world.len();
    println!(
        "Expedition rover: cost={}, parts={}, balance_after_build={}, build_goal_progress={}",
        quote.studs,
        count - baseline,
        balance,
        progress(&game, "build")
    );
    for _ in 0..3 {
        game.undo();
        assert_eq!(game.world.len(), baseline);
        assert_eq!(game.gameplay.studs(), balance + quote.studs);
        game.redo();
        assert_eq!(game.world.len(), count);
        assert_eq!(game.gameplay.studs(), balance);
    }
    assert!(game.workshop_status().ready);
    game.activate_vehicle("auto");
    assert!(game.active_vehicle.is_some(), "{}", game.status);
    assert_eq!(progress(&game, "workshop"), 1);
    assert_eq!(game.gameplay.studs(), balance + 260);
    game.input.forward = true;
    for _ in 0..120 {
        game.tick(1.0 / 60.0);
    }
    game.input.clear();
    assert!(progress(&game, "drive") > 5);
    game.park_vehicle();
    assert!(game.active_vehicle.is_none(), "{}", game.status);
    let final_balance = game.gameplay.studs();
    game.undo();
    game.redo();
    assert_eq!(game.gameplay.studs(), final_balance);
}

#[test]
fn powered_plane_takeoff_records_flight_and_reactivation_cannot_repeat_the_reward() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.set_expedition(true);
    game.spawn_blueprint("plane");
    assert!(game.workshop_status().ready, "{}", game.status);
    let before_activation = game.gameplay.studs();
    game.activate_vehicle("auto");
    assert!(game.active_vehicle.is_some(), "{}", game.status);
    assert_eq!(game.gameplay.studs(), before_activation + 260);
    game.park_vehicle();
    assert!(game.active_vehicle.is_none(), "{}", game.status);
    game.activate_vehicle("auto");
    assert!(game.active_vehicle.is_some(), "{}", game.status);
    assert_eq!(game.gameplay.studs(), before_activation + 260);
    game.input.forward = true;
    game.input.up = true;
    for _ in 0..1800 {
        game.tick(1.0 / 60.0);
        if progress(&game, "flight") >= 5 {
            break;
        }
    }
    game.input.clear();
    let plane = &game.active_vehicle.as_ref().unwrap().vehicle;
    assert_eq!(plane.kind, briccraft_mechanics::VehicleKind::Plane);
    assert!(plane.position[1] > 9.0, "Plane did not take off: {plane:?}");
    assert!(
        progress(&game, "flight") >= 5,
        "Powered airborne movement was not recorded: {plane:?}"
    );
    assert_eq!(game.gameplay.studs(), before_activation + 260);
}

#[test]
fn walk_tracks_displacement_but_orbit_flight_and_streaming_shifts_do_not() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.set_expedition(true);
    game.camera.set_mode(CameraMode::Walk);
    game.camera.position = glam::Vec3::new(48.0, 9.4, 48.0);
    game.camera.yaw = 0.0;
    game.camera.pitch = 0.0;
    game.input.forward = true;
    for _ in 0..180 {
        game.tick(1.0 / 60.0);
    }
    game.input.clear();
    assert!(progress(&game, "walk") >= 10);
    let walked = progress(&game, "walk");
    game.camera.position.x += 128.0;
    for _ in 0..36 {
        game.tick(0.0);
    }
    assert_eq!(progress(&game, "walk"), walked);
    game.camera.set_mode(CameraMode::Fly);
    game.input.forward = true;
    for _ in 0..60 {
        game.tick(1.0 / 60.0);
    }
    game.input.clear();
    assert_eq!(progress(&game, "walk"), walked);
    assert_eq!(progress(&game, "flight"), 0);
}

#[test]
fn saves_preserve_mode_wallet_rewards_and_legacy_saves_default_to_creative() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.set_expedition(true);
    game.spawn_blueprint("rover");
    game.activate_vehicle("auto");
    game.park_vehicle();
    let balance = game.gameplay.studs();
    let completed = game
        .gameplay
        .objectives()
        .iter()
        .filter(|o| o.completed)
        .count();
    let source = game.to_json().unwrap();
    let mut loaded = Session::with_world(World::empty(1));
    loaded.import_json(&source).unwrap();
    assert!(loaded.gameplay.is_expedition());
    assert_eq!(loaded.gameplay.studs(), balance);
    assert_eq!(
        loaded
            .gameplay
            .objectives()
            .iter()
            .filter(|o| o.completed)
            .count(),
        completed
    );
    loaded.set_expedition(false);
    loaded.set_expedition(true);
    assert_eq!(loaded.gameplay.studs(), balance);
    assert!(!loaded.can_undo());
    assert!(!loaded.can_redo());
    let mut legacy: serde_json::Value = serde_json::from_str(&source).unwrap();
    legacy.as_object_mut().unwrap().remove("gameplay");
    loaded.import_json(&legacy.to_string()).unwrap();
    assert!(!loaded.gameplay.is_expedition());
    assert!(!loaded.gameplay.started());
    let original = loaded.world.len();
    legacy["gameplay"] = serde_json::to_value(game.gameplay.clone()).unwrap();
    legacy["gameplay"]["studs"] = serde_json::json!(u64::MAX);
    assert!(loaded.import_json(&legacy.to_string()).is_err());
    assert_eq!(loaded.world.len(), original);
    assert!(!loaded.gameplay.is_expedition());
}

#[test]
fn creative_builds_stay_unlimited_and_expediton_cannot_import_uncharged_geometry() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    assert!(!game.blueprint_quote("rover").unwrap().charged);
    game.spawn_blueprint("rover");
    assert_eq!(game.gameplay.studs(), STARTER_STUDS);
    assert_eq!(progress(&game, "build"), 0);
    game.set_expedition(true);
    let count = game.world.len();
    assert!(game
        .begin_import(
            vec![GlobalPlacement::new(20, 0, GlobalPos::new(20, 18, 20), 0)],
            "import".into(),
            vec![]
        )
        .unwrap_err()
        .to_string()
        .contains("Creative"));
    assert_eq!(game.world.len(), count);
    assert_eq!(game.gameplay.studs(), STARTER_STUDS);
}
