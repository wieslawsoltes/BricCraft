use crate::{BiomePreset, GameAttribution, GlobalPlacement, GlobalPos, Session, WindowOrigin};
use briccraft_import::{import_overpass_json, GeoBounds, OsmOptions};
use briccraft_world::Placement;
use glam::Vec3;

#[test]
fn real_brandenburg_extract_imports_saves_and_undoes_without_terrain_conflicts() {
    let options = OsmOptions {
        bounds: GeoBounds::around(52.5163, 13.3777, 160.0).unwrap(),
        meters_per_stud: 4.0,
        origin: [48, 18, 48],
        max_height_plates: Some(54),
        ..OsmOptions::default()
    };
    let plan = import_overpass_json(
        include_str!("../../briccraft-import/tests/fixtures/brandenburg-osm.json"),
        &options,
    )
    .unwrap();
    let count = plan.placements.len();
    let report = serde_json::to_string(&plan.report).unwrap();
    let source_height = plan
        .report
        .height_evidence
        .iter()
        .find(|e| e.osm_id == 518071791)
        .unwrap();
    assert_eq!(source_height.height_m, 26.0);
    assert!(!source_height.inferred);
    let attributions = plan
        .attribution
        .into_iter()
        .map(|a| GameAttribution {
            label: a.text,
            url: a.url,
            license: a.license,
        })
        .collect();
    let global = plan
        .placements
        .into_iter()
        .map(|p| {
            GlobalPlacement::new(
                p.part_id,
                p.color,
                GlobalPos::new(
                    i64::from(p.position[0]),
                    p.position[1],
                    i64::from(p.position[2]),
                ),
                p.rotation,
            )
        })
        .collect();
    let mut session = Session::new_streamed(42, BiomePreset::Flat).unwrap();
    let terrain_parts = session.world.len();
    session
        .begin_import(global, "Brandenburg Gate district".into(), attributions)
        .unwrap();
    session.world_info.import_report = Some(report.clone());
    let mut completed = None;
    for _ in 0..256 {
        session.tick(0.0);
        if let Some(result) = session.take_import_result() {
            completed = Some(result);
            break;
        }
    }
    assert!(
        completed
            .expect("bounded landmark import completed")
            .is_ok(),
        "{}",
        session.status
    );
    assert_eq!(
        session
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .sparse_edit_count(),
        count
    );
    assert_eq!(session.world.len(), terrain_parts + count);
    let saved_count = session.world.len();
    let json = session.to_json().unwrap();
    let mut restored = Session::new(7);
    restored.import_json(&json).unwrap();
    assert_eq!(restored.world.len(), saved_count);
    assert_eq!(
        restored.world_info.import_report.as_deref(),
        Some(report.as_str())
    );
    assert_eq!(restored.world_info.attributions[0].license, "ODbL-1.0");
    // Import is one transaction in the live global history, even after its snapshot was compacted.
    session.undo();
    assert_eq!(session.world.len(), terrain_parts);
    session.redo();
    assert_eq!(session.world.len(), saved_count);
    // A loaded save begins a fresh undo session; subsequent edits still resolve compacted identities.
    restored
        .world
        .place(Placement::new(2, 0, [42, 69, 42], 0))
        .unwrap();
    restored.capture_streamed_edits().unwrap();
    restored.undo();
    assert_eq!(restored.world.len(), saved_count);
    restored.redo();
    assert_eq!(restored.world.len(), saved_count + 1);
    println!("Brandenburg integration: {count} imported parts, {saved_count} visible parts, {} JSON bytes", json.len());
}

#[test]
fn save_retains_i64_coordinates_and_u64_seed_beyond_javascript_precision() {
    let seed = u64::MAX - 7;
    let mut session = Session::new_streamed(seed, BiomePreset::Flat).unwrap();
    let origin = WindowOrigin::new((1_i64 << 60) + 16, -(1_i64 << 60) - 32);
    let placement = GlobalPlacement::new(2, 1, origin.global_cell([33, 18, 47]).unwrap(), 0);
    let mut state = session.streaming.take().unwrap();
    let key = state.stream.place_parts(&[placement]).unwrap().added_keys()[0];
    let snapshot = state.stream.prepare_window(origin).unwrap();
    session.publish_streamed_snapshot(&mut state, snapshot);
    session.streaming = Some(state);
    session.camera.position = Vec3::new(48.25, 17.5, 51.75);
    session.camera.target = Vec3::new(48.25, 7.5, 61.75);
    let source = session.to_json().unwrap();
    assert!(source.contains(&seed.to_string()));
    assert!(source.contains(&placement.position.x.to_string()));
    assert!(source.contains(&placement.position.z.to_string()));
    let mut restored = Session::new(0);
    restored.import_json(&source).unwrap();
    assert_eq!(restored.seed, seed);
    assert_eq!(restored.world_origin(), origin);
    assert_eq!(restored.camera.position, session.camera.position);
    assert_eq!(
        restored
            .streaming
            .as_ref()
            .unwrap()
            .stream
            .resolve(key)
            .unwrap(),
        Some(placement)
    );
    assert_eq!(restored.to_json().unwrap(), source);
}
