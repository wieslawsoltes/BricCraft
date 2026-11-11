use crate::{BiomePreset, CaptureKind, GlobalPlacement, GlobalPos, Session, WindowOrigin};
use briccraft_play::{ChangeValue, STARTER_STUDS};
use briccraft_world::{Blueprint, Placement, RayHit, World};

fn fixture() -> Session {
    let mut game = Session::with_world(World::empty(7));
    game.last_blueprint = [
        Placement::new(20, 1, [10, 4, 10], 0),
        Placement::new(20, 4, [11, 4, 10], 1),
    ]
    .into_iter()
    .map(|placement| game.world.place(placement).unwrap())
    .collect();
    game.hit = None;
    game
}

fn progress(game: &Session, id: &str) -> u64 {
    game.gameplay
        .objectives()
        .into_iter()
        .find(|objective| objective.id == id)
        .unwrap()
        .progress
}

#[test]
fn design_copy_uses_atomic_cost_history_and_successful_part_progress() {
    let mut game = fixture();
    game.set_expedition(true);
    let id = game.capture_design("Two-tone tile").unwrap();
    assert_eq!(game.gameplay.studs(), STARTER_STUDS);
    assert_eq!(progress(&game, "build"), 0);
    assert_eq!(game.design_quote(id).unwrap().studs, 2);
    let before = game.world.len();
    let ids = game.place_design_at(id, [20, 4, 20], 1).unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(game.world.len(), before + 2);
    assert_eq!(game.gameplay.studs(), STARTER_STUDS - 2);
    assert_eq!(progress(&game, "build"), 2);
    assert_eq!(
        progress(&game, "home"),
        0,
        "copying a recipe does not invent a structure reward"
    );
    for _ in 0..3 {
        game.undo();
        assert_eq!(game.world.len(), before);
        assert_eq!(game.gameplay.studs(), STARTER_STUDS);
        game.redo();
        assert_eq!(game.world.len(), before + 2);
        assert_eq!(game.gameplay.studs(), STARTER_STUDS - 2);
    }
    let revision = game.world.revision();
    assert!(game.place_design_at(id, [20, 4, 20], 1).is_err());
    assert_eq!(game.world.revision(), revision);
    assert_eq!(game.gameplay.studs(), STARTER_STUDS - 2);
    assert_eq!(progress(&game, "build"), 2);
    let permit = game
        .gameplay
        .approve(ChangeValue {
            added: STARTER_STUDS - 2,
            removed: 0,
        })
        .unwrap();
    game.gameplay.commit(permit).unwrap();
    assert!(game.place_design_at(id, [40, 4, 40], 0).is_err());
    assert_eq!(game.world.len(), before + 2);
    assert_eq!(game.gameplay.studs(), 0);
}

#[test]
fn face_and_ground_anchors_use_the_rotated_envelope_without_relocation() {
    let mut game = fixture();
    let id = game.capture_design("Anchor test").unwrap();
    let source = game.world.instance(game.last_blueprint[1]).unwrap().clone();
    game.rotation = 1;
    game.hit = Some(RayHit {
        id: source.id,
        position: [12.0, 1.7, 10.5],
        cell: source.position,
        normal: [1, 0, 0],
        distance: 2.0,
    });
    let ids = game.place_design(id).unwrap();
    let minimum: [i32; 3] = std::array::from_fn(|axis| {
        ids.iter()
            .map(|id| game.world.instance(*id).unwrap().position[axis])
            .min()
            .unwrap()
    });
    assert_eq!(minimum, [12, 4, 10]);
    game.hit = None;
    game.camera.target = glam::Vec3::new(40.0, 0.0, 40.0);
    let ids = game.place_design(id).unwrap();
    assert!(ids
        .iter()
        .all(|id| game.world.instance(*id).unwrap().position[1] == 0));
    game.hit = None;
    game.camera.target = glam::Vec3::new(95.0, 0.0, 95.0);
    let before = game.world.len();
    assert!(game.place_design(id).is_err());
    assert_eq!(game.world.len(), before);
}

#[test]
fn names_and_recipe_deletion_leave_placed_parts_wallet_and_rewards_unchanged() {
    let mut game = fixture();
    game.set_expedition(true);
    let id = game.capture_design("First name").unwrap();
    let parts = game.designs()[0].relative_parts().to_vec();
    let source = game.designs()[0].metadata().clone();
    game.rename_design(id, "New name").unwrap();
    assert_eq!(game.designs()[0].id, id);
    assert_eq!(game.designs()[0].relative_parts(), parts);
    assert_eq!(game.designs()[0].metadata(), &source);
    let count = game.world.len();
    game.delete_design(id).unwrap();
    assert!(game.designs().is_empty());
    assert_eq!(game.world.len(), count);
    assert_eq!(game.gameplay.studs(), STARTER_STUDS);
    assert_eq!(progress(&game, "build"), 0);
    assert!(game.place_design(id).is_err());
}

#[test]
fn terrain_capture_is_bounded_and_latest_explicit_construction_is_an_honest_fallback() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    let terrain = game.world.cell([48, 17, 48]).unwrap().clone();
    game.hit = Some(RayHit {
        id: terrain.id,
        position: [48.5, 7.2, 48.5],
        cell: [48, 17, 48],
        normal: [0, 1, 0],
        distance: 10.0,
    });
    assert!(!game.design_capture_status().ready);
    assert!(game.capture_design("Whole landscape").is_err());
    game.last_blueprint = [
        Placement::new(20, 1, [40, 18, 40], 0),
        Placement::new(20, 3, [41, 18, 40], 0),
    ]
    .into_iter()
    .map(|placement| game.world.place(placement).unwrap())
    .collect();
    game.capture_streamed_edits().unwrap();
    let ready = game.design_capture_status();
    assert!(ready.ready, "{}", ready.detail);
    assert_eq!(ready.part_count, 2);
    let id = game.capture_design("Two plates").unwrap();
    assert_eq!(
        game.designs()
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .metadata()
            .capture,
        CaptureKind::ConstructionGroup
    );
}

#[test]
fn saves_validate_designs_atomically_and_legacy_worlds_start_with_an_empty_library() {
    let mut game = fixture();
    game.capture_design("Saved colors").unwrap();
    let source = game.to_json().unwrap();
    let mut loaded = Session::with_world(World::empty(99));
    loaded.import_json(&source).unwrap();
    assert_eq!(loaded.designs(), game.designs());
    assert_eq!(loaded.world.len(), game.world.len());
    let original = loaded.designs().to_vec();
    let count = loaded.world.len();
    let mut corrupt: serde_json::Value = serde_json::from_str(&source).unwrap();
    corrupt["designs"]["designs"][0]["parts"][0]["part_id"] = u16::MAX.into();
    assert!(loaded.import_json(&corrupt.to_string()).is_err());
    assert_eq!(loaded.designs(), original);
    assert_eq!(loaded.world.len(), count);
    let mut legacy: serde_json::Value = serde_json::from_str(&source).unwrap();
    legacy.as_object_mut().unwrap().remove("designs");
    loaded.import_json(&legacy.to_string()).unwrap();
    assert!(loaded.designs().is_empty());
}

#[test]
fn designs_survive_floating_origin_changes_and_still_use_global_construction_history() {
    let mut game = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    game.spawn_blueprint("rover");
    game.hit = None;
    let id = game.capture_design("Travel rover").unwrap();
    let original = game.designs()[0].clone();
    let mut state = game.streaming.take().unwrap();
    let origin = WindowOrigin::new(128, -128);
    let snapshot = state.stream.prepare_window(origin).unwrap();
    game.publish_streamed_snapshot(&mut state, snapshot);
    game.streaming = Some(state);
    assert_eq!(game.designs()[0], original);
    let before = game.world.len();
    let ids = game.place_design_at(id, [40, 18, 40], 1).unwrap();
    assert_eq!(ids.len(), original.part_count());
    assert_eq!(game.world_origin(), origin);
    game.undo();
    assert_eq!(game.world.len(), before);
    game.redo();
    assert_eq!(game.world.len(), before + original.part_count());
    let source = game.to_json().unwrap();
    let mut loaded = Session::with_world(World::empty(1));
    loaded.import_json(&source).unwrap();
    assert_eq!(loaded.designs(), game.designs());
    assert_eq!(loaded.world_origin(), origin);
}

#[test]
fn active_vehicles_and_staged_imports_cannot_be_captured_or_edited_as_designs() {
    let mut game = Session::with_world(World::empty(7));
    game.last_blueprint = game
        .world
        .spawn_blueprint(Blueprint::Rover, [10, 5, 10], 0)
        .unwrap();
    let id = game.capture_design("Parked rover").unwrap();
    game.activate_vehicle("auto");
    assert!(game.active_vehicle.is_some(), "{}", game.status);
    assert!(!game.design_capture_status().ready);
    assert!(game.capture_design("Moving rover").is_err());
    assert!(game.place_design_at(id, [40, 5, 40], 0).is_err());
    let mut game = Session::new_streamed(7, BiomePreset::Flat).unwrap();
    game.begin_import(
        vec![GlobalPlacement::new(20, 0, GlobalPos::new(40, 18, 40), 0)],
        "fixture".into(),
        vec![],
    )
    .unwrap();
    assert!(!game.design_capture_status().ready);
    assert!(game.capture_design("Pending import").is_err());
}

#[test]
fn portable_design_import_is_library_only_and_uses_the_destination_budget_on_placement() {
    let mut source = fixture();
    source.world_info.id = "source-world".into();
    let source_id = source.capture_design("Portable colors").unwrap();
    let document = source.export_design(source_id).unwrap();
    let mut destination = fixture();
    destination.world_info.id = "destination-world".into();
    destination.capture_design("Portable colors").unwrap();
    destination.set_expedition(true);
    let before = destination.world.len();
    let imported_id = destination.import_design(&document).unwrap();
    let imported = destination
        .designs()
        .iter()
        .find(|d| d.id == imported_id)
        .unwrap();
    assert_ne!(imported_id, source_id);
    assert_eq!(imported.name, "Portable colors (2)");
    assert_eq!(imported.metadata().source_world_id, "source-world");
    assert_eq!(
        imported.relative_parts(),
        source.designs()[0].relative_parts()
    );
    assert_eq!(destination.world.len(), before);
    assert_eq!(destination.gameplay.studs(), STARTER_STUDS);
    assert_eq!(progress(&destination, "build"), 0);
    destination
        .place_design_at(imported_id, [40, 4, 40], 0)
        .unwrap();
    assert_eq!(destination.world.len(), before + 2);
    assert_eq!(destination.gameplay.studs(), STARTER_STUDS - 2);
    assert_eq!(progress(&destination, "build"), 2);
}

#[test]
fn malformed_or_oversized_portable_files_leave_library_world_and_balance_unchanged() {
    let mut game = fixture();
    let id = game.capture_design("Unchanged").unwrap();
    game.set_expedition(true);
    let source = game.export_design(id).unwrap();
    let library = game.designs().to_vec();
    let world_count = game.world.len();
    let mut invalid: serde_json::Value = serde_json::from_str(&source).unwrap();
    invalid["design"]["parts"][0]["color"] = u8::MAX.into();
    let invalid_format = source.replace("briccraft.design", "untrusted.format");
    for candidate in [
        invalid.to_string(),
        invalid_format,
        "not JSON".into(),
        " ".repeat(briccraft_design::MAX_DESIGN_FILE_BYTES + 1),
    ] {
        assert!(game.import_design(&candidate).is_err());
        assert_eq!(game.designs(), library);
        assert_eq!(game.world.len(), world_count);
        assert_eq!(game.gameplay.studs(), STARTER_STUDS);
        assert_eq!(progress(&game, "build"), 0);
    }
}
