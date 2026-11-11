mod common;
use briccraft_import::{
    import_overpass_json, osm, GeoBounds, GeoReference, ImportError, ImportStage, OsmOptions,
};
use serde_json::{json, Value};

fn geometry(points: &[[f64; 2]]) -> Vec<Value> {
    points.iter().map(|[x,z]| json!({"lat":(-z / 6_378_137.0).to_degrees(), "lon":(x / 6_378_137.0).to_degrees()})).collect()
}
fn options() -> OsmOptions {
    OsmOptions {
        bounds: GeoBounds::around(0.0, 0.0, 8.0).unwrap(),
        meters_per_stud: 1.0,
        origin: [1000, 18, -1000],
        include_ground: false,
        ..OsmOptions::default()
    }
}
fn building(points: &[[f64; 2]], height: &str) -> String {
    json!({"elements":[{"type":"way","id":1,"tags":{"building":"yes","height":height},"geometry":geometry(points)}]}).to_string()
}

#[test]
fn actual_brandenburg_extract_retains_tagged_height_and_attribution() {
    let options = OsmOptions {
        bounds: GeoBounds::around(52.516275, 13.377704, 160.0).unwrap(),
        meters_per_stud: 4.0,
        origin: [48, 18, 48],
        max_height_plates: Some(54),
        ..OsmOptions::default()
    };
    let plan =
        import_overpass_json(include_str!("fixtures/brandenburg-osm.json"), &options).unwrap();
    // Same sample center as the shipped worker ABI and browser gallery.
    assert_eq!(plan.report.parts, 169);
    assert_eq!(plan.report.voxels, 7329);
    assert_eq!(plan.report.buildings, 22);
    assert_eq!(plan.report.roads, 17);
    let gate = plan
        .report
        .height_evidence
        .iter()
        .find(|h| h.osm_id == 518071791)
        .unwrap();
    assert_eq!(gate.height_m, 26.0);
    assert!(!gate.inferred);
    assert!(plan.report.inferred_heights > 0);
    assert_eq!(plan.attribution[0].license, "ODbL-1.0");
    assert!(plan.report.bounds.max[1] <= 72);
    let occupied = common::cells(&plan);
    assert_eq!(occupied.get(&[48, 18, 48]), Some(&29));
    println!(
        "Brandenburg: {} parts, {} cells, {} buildings, {} roads, bounds {:?}",
        plan.report.parts,
        plan.report.voxels,
        plan.report.buildings,
        plan.report.roads,
        plan.report.bounds
    );
    let json = serde_json::to_string(&plan).unwrap();
    let restored: briccraft_import::ImportPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.placements, plan.placements);
}

#[test]
fn concave_footprint_is_not_replaced_by_its_bounding_rectangle() {
    let source = building(
        &[
            [-6.0, -6.0],
            [6.0, -6.0],
            [6.0, -2.0],
            [-2.0, -2.0],
            [-2.0, 6.0],
            [-6.0, 6.0],
            [-6.0, -6.0],
        ],
        "4",
    );
    let plan = import_overpass_json(&source, &options()).unwrap();
    let occupied = common::cells(&plan);
    assert!(occupied.contains_key(&[995, 19, -995]));
    assert!(!occupied.contains_key(&[1004, 19, -996]));
    assert!(plan.report.bounds.min[0] > 96);
    assert!(plan.report.bounds.max[2] < 0);
    assert_eq!(
        plan.batches(3).map(|b| b.len()).sum::<usize>(),
        plan.placements.len()
    );
}

#[test]
fn relation_stitching_preserves_courtyards_and_reversed_segments() {
    let outer_a = geometry(&[[-6.0, -6.0], [6.0, -6.0], [6.0, 6.0]]);
    let outer_b = geometry(&[[-6.0, -6.0], [-6.0, 6.0], [6.0, 6.0]]);
    let inner = geometry(&[
        [-2.0, -2.0],
        [2.0, -2.0],
        [2.0, 2.0],
        [-2.0, 2.0],
        [-2.0, -2.0],
    ]);
    let source=json!({"elements":[{"type":"relation","id":9,"tags":{"type":"multipolygon","building":"yes","height":"4"},"members":[
        {"type":"way","ref":1,"role":"outer","geometry":outer_a},
        {"type":"way","ref":2,"role":"outer","geometry":outer_b},
        {"type":"way","ref":3,"role":"inner","geometry":inner}
    ]}]}).to_string();
    let plan = import_overpass_json(&source, &options()).unwrap();
    let occupied = common::cells(&plan);
    assert!(occupied.contains_key(&[995, 19, -1000]));
    assert!(!occupied.keys().any(|p| p[0] == 1000 && p[2] == -1000));
    assert_eq!(plan.report.height_evidence[0].osm_type, "relation");
}

#[test]
fn missing_inner_geometry_cannot_fill_an_unknown_courtyard() {
    let source=json!({"elements":[{"type":"relation","id":9,"tags":{"type":"multipolygon","building":"yes"},"members":[
        {"type":"way","ref":1,"role":"outer","geometry":geometry(&[[-6.0,-6.0],[6.0,-6.0],[6.0,6.0],[-6.0,6.0],[-6.0,-6.0]])},
        {"type":"way","ref":2,"role":"inner"}
    ]}]}).to_string();
    assert!(matches!(
        import_overpass_json(&source, &options()),
        Err(ImportError::EmptyGeometry)
    ));
}

#[test]
fn roads_follow_their_polyline_and_explicit_width() {
    let source=json!({"elements":[{"type":"way","id":3,"tags":{"highway":"residential","width":"2"},"geometry":geometry(&[[-6.0,-6.0],[0.0,0.0],[6.0,-6.0]])}]}).to_string();
    let plan = import_overpass_json(&source, &options()).unwrap();
    let occupied = common::cells(&plan);
    assert!(occupied.contains_key(&[1000, 19, -1000]));
    assert!(!occupied.contains_key(&[1000, 19, -1005]));
    assert_eq!(plan.report.inferred_widths, 0);
}

#[test]
fn excessive_heights_fail_without_squashing_the_map() {
    let source = building(
        &[
            [-4.0, -4.0],
            [4.0, -4.0],
            [4.0, 4.0],
            [-4.0, 4.0],
            [-4.0, -4.0],
        ],
        "100",
    );
    let mut options = options();
    options.max_height_plates = Some(54);
    let error = import_overpass_json(&source, &options)
        .unwrap_err()
        .to_string();
    assert!(error.contains("meters per stud"));
    assert!(
        import_overpass_json(r#"{"remark":"runtime timeout","elements":[]}"#, &options).is_err()
    );
    assert!(import_overpass_json(r##"{"elements":[{"type":"way","id":1,"tags":{"building":"yes","building:colour":"#中文"},"geometry":[] }]}"##,&options).is_err());
}

#[test]
fn projection_and_progress_have_explicit_direction_and_cancellation() {
    let options = options();
    let projection = GeoReference::new(options.bounds, 1.0, options.origin).unwrap();
    assert!(projection.project(0.0, 0.00001).unwrap()[0] > 1000.0);
    assert!(projection.project(0.00001, 0.0).unwrap()[1] < -1000.0);
    let source = building(
        &[
            [-4.0, -4.0],
            [4.0, -4.0],
            [4.0, 4.0],
            [-4.0, 4.0],
            [-4.0, -4.0],
        ],
        "4",
    );
    let mut stages = Vec::new();
    let result = osm::import_overpass_json_with_progress(&source, &options, &mut |p| {
        stages.push(p.stage);
        p.stage != ImportStage::Pack
    });
    assert!(matches!(result, Err(ImportError::Cancelled)));
    assert!(stages.contains(&ImportStage::Project));
}
