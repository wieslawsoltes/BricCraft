mod common;
use briccraft_import::*;
use serde_json::{json, Value};

fn geometry(points: &[[f64; 2]]) -> Vec<Value> {
    points.iter().map(|[x,z]|json!({"lat":(-z/6_378_137.0).to_degrees(),"lon":(x/6_378_137.0).to_degrees()})).collect()
}
fn polygon(id: i64, points: &[[f64; 2]], tags: Value) -> Value {
    json!({"type":"way","id":id,"tags":tags,"geometry":geometry(points)})
}
fn rect(x0: f64, z0: f64, x1: f64, z1: f64) -> [[f64; 2]; 5] {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1], [x0, z0]]
}
fn options(radius: f64) -> OsmOptions {
    let bounds = GeoBounds::around(0.0, 0.0, radius).unwrap();
    let elevation = ElevationGrid {
        width: 2,
        height: 2,
        bounds,
        grid_space: GridSpace::Geographic,
        values: vec![Some(0.0), Some(4.0), Some(0.0), Some(4.0)],
        vertical_datum: VerticalDatum::Egm96,
        source: ElevationSource {
            name: "Controlled survey".into(),
            url: "https://example.test/elevation".into(),
            attribution: "Survey contributors".into(),
            license: "CC0-1.0".into(),
            license_url: String::new(),
            resolution_m: Some(1.0),
            timestamp: Some("2026-01-01".into()),
        },
    };
    OsmOptions {
        bounds,
        meters_per_stud: 1.0,
        origin: [48, 18, 48],
        max_height_plates: Some(54),
        terrain: Some(TerrainOptions {
            elevation,
            vertical_exaggeration: 1.0,
            fit_height: true,
            auto_coarsen: true,
            max_columns: 262_144,
        }),
        ..OsmOptions::default()
    }
}

#[test]
fn full_area_terrain_supports_buildings_draped_roads_landcover_and_inferred_water() {
    let options = options(10.0);
    let source = json!({"elements":[
        polygon(1,&rect(-2.0,-2.0,2.0,2.0),json!({"building":"yes","height":"4"})),
        polygon(2,&[[-8.0,5.0],[8.0,5.0]],json!({"highway":"residential","width":"2"})),
        polygon(3,&rect(-8.0,-8.0,-4.0,-4.0),json!({"natural":"water"})),
        polygon(4,&rect(4.0,-8.0,8.0,-4.0),json!({"leisure":"park"}))
    ]})
    .to_string();
    let plan = import_overpass_json(&source, &options).unwrap();
    let cells = common::cells(&plan);
    let terrain = plan.report.terrain.as_ref().unwrap();
    assert_eq!(
        (
            plan.report.buildings,
            plan.report.roads,
            plan.report.water,
            plan.report.landcover
        ),
        (1, 1, 1, 1)
    );
    assert!(terrain.foundation_columns > 0);
    assert_eq!(terrain.inferred_water_levels, 1);
    assert_eq!(terrain.vertical_datum, VerticalDatum::Egm96);
    assert_eq!(plan.attribution.len(), 2);
    let top = |x, z| {
        cells
            .keys()
            .filter(|p| p[0] == x && p[2] == z)
            .map(|p| p[1])
            .max()
            .unwrap()
    };
    // Every terrain column fills the full vertical interval above the flat canvas.
    for x in plan.report.bounds.min[0]..plan.report.bounds.max[0] {
        for z in plan.report.bounds.min[2]..plan.report.bounds.max[2] {
            assert!(cells.contains_key(&[x, 18, z]));
        }
    }
    assert!(top(54, 53) > top(42, 53));
    assert_eq!(cells.get(&[42, top(42, 53), 53]), Some(&7));
    assert_eq!(cells.get(&[54, top(54, 53), 53]), Some(&7));
    assert_eq!(top(40, 40), top(43, 43));
    assert_eq!(cells.get(&[40, top(40, 40), 40]), Some(&14));
    assert_eq!(cells.get(&[54, top(54, 42), 42]), Some(&12));
    // Building roof is horizontal even though its footprint crosses a slope.
    assert_eq!(top(46, 46), top(49, 49));
    assert!(plan.report.bounds.max[1] <= 72);
    assert!(!plan
        .report
        .warnings
        .iter()
        .any(|w| w.contains("No terrain elevation")));
    let restored: ImportPlan =
        serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    assert_eq!(restored.placements, plan.placements);
    assert_eq!(
        restored.report.terrain.unwrap().source.name,
        "Controlled survey"
    );
}

#[test]
fn full_extent_coarsening_is_deterministic_and_reported_even_without_osm_features() {
    let mut options = options(1500.0);
    options.meters_per_stud = 4.0;
    options.terrain.as_mut().unwrap().max_columns = 4096;
    let source = r#"{"elements":[]}"#;
    let a = import_overpass_json(source, &options).unwrap();
    let b = import_overpass_json(source, &options).unwrap();
    assert_eq!(a.placements, b.placements);
    let terrain = a.report.terrain.as_ref().unwrap();
    assert!(terrain.effective_meters_per_stud > 4.0);
    assert!(terrain.columns <= 4096);
    let projection = a.report.projection.as_ref().unwrap();
    let grid = projection.grid_bounds().unwrap();
    assert_eq!(a.report.bounds.min[0], grid[0]);
    assert_eq!(a.report.bounds.max[0], grid[2]);
    assert_eq!(a.report.bounds.min[2], grid[1]);
    assert_eq!(a.report.bounds.max[2], grid[3]);
    common::cells(&a);
    options.terrain.as_mut().unwrap().auto_coarsen = false;
    assert!(matches!(
        import_overpass_json(source, &options),
        Err(ImportError::BudgetExceeded { .. })
    ));
}

#[test]
fn vertical_fit_is_explicit_and_strict_proportions_reject_overflow() {
    let mut options = options(8.0);
    options.terrain.as_mut().unwrap().elevation.values =
        vec![Some(0.0), Some(600.0), Some(0.0), Some(600.0)];
    let source=json!({"elements":[polygon(1,&rect(-2.0,-2.0,2.0,2.0),json!({"building":"yes","height":"100"}))]}).to_string();
    let plan = import_overpass_json(&source, &options).unwrap();
    let terrain = plan.report.terrain.as_ref().unwrap();
    assert!(terrain.vertical_compression < 0.1);
    assert!(terrain.effective_meters_per_plate > terrain.requested_meters_per_plate);
    assert!(plan.report.bounds.max[1] <= 72);
    common::cells(&plan);
    options.terrain.as_mut().unwrap().fit_height = false;
    assert!(import_overpass_json(&source, &options)
        .unwrap_err()
        .to_string()
        .contains("true-height"));
}

#[test]
fn missing_dem_coverage_and_cancellation_never_return_partial_worlds() {
    let mut options = options(8.0);
    options.terrain.as_mut().unwrap().elevation.values[3] = None;
    assert!(import_overpass_json(r#"{"elements":[]}"#, &options)
        .unwrap_err()
        .to_string()
        .contains("void"));
    options.terrain.as_mut().unwrap().elevation.values[3] = Some(4.0);
    let result =
        osm::import_overpass_json_with_progress(r#"{"elements":[]}"#, &options, &mut |p| {
            p.stage != ImportStage::Pack
        });
    assert!(matches!(result, Err(ImportError::Cancelled)));
    options.terrain.as_mut().unwrap().elevation.bounds.east -= 0.00001;
    assert!(import_overpass_json(r#"{"elements":[]}"#, &options)
        .unwrap_err()
        .to_string()
        .contains("complete requested map"));
}

#[test]
fn terrain_volume_is_not_limited_by_sparse_feature_voxel_allocation() {
    let mut options = options(16.0);
    options.limits.max_voxels = 1;
    let plan = import_overpass_json(r#"{"elements":[]}"#, &options).unwrap();
    assert!(plan.report.voxels > 1000);
    assert_eq!(
        plan.report.voxels,
        plan.report.terrain.unwrap().filled_cells
    );
}

#[test]
fn measured_area_commits_atomically_and_survives_streaming_save_and_undo() {
    use briccraft_stream::{
        BiomePreset, GenerationBudget, GlobalPlacement, GlobalPos, StreamWorld, WindowOrigin,
    };
    let plan = import_overpass_json(r#"{"elements":[]}"#, &options(120.0)).unwrap();
    let placements = plan
        .placements
        .iter()
        .map(|p| {
            GlobalPlacement::new(
                p.part_id,
                p.color,
                GlobalPos {
                    x: p.position[0] as i64,
                    y: p.position[1],
                    z: p.position[2] as i64,
                },
                p.rotation,
            )
        })
        .collect();
    let mut stream = StreamWorld::new(42, BiomePreset::Flat, 64).unwrap();
    let before = stream
        .prepare_window(WindowOrigin { x: -64, z: -64 })
        .unwrap();
    assert!(!before.world.occupied([48, 18, 48]));
    let mut job = stream.begin_import(placements).unwrap();
    let first = stream
        .advance_import(&mut job, GenerationBudget::chunks(1))
        .unwrap();
    assert!(!first.complete);
    assert_eq!(stream.stats().sparse_edits, 0);
    for _ in 0..512 {
        if stream
            .advance_import(&mut job, GenerationBudget::chunks(4))
            .unwrap()
            .complete
        {
            break;
        }
    }
    assert!(job.progress().complete);
    stream.commit_import(job).unwrap();
    assert_eq!(stream.stats().sparse_edits, plan.placements.len());
    for origin in [
        WindowOrigin { x: -64, z: -64 },
        WindowOrigin { x: 64, z: 64 },
    ] {
        let snapshot = stream.prepare_window(origin).unwrap();
        assert!(
            snapshot.boundary_parts.is_empty(),
            "chunk-aligned terrain must not leave omitted boundary slabs"
        );
        for x in 0..96 {
            for z in 0..96 {
                assert!(
                    snapshot.world.occupied([x, 18, z]),
                    "missing imported ground at {origin:?} + {x},{z}"
                );
            }
        }
    }
    let mut restored = StreamWorld::from_json(&stream.to_json().unwrap()).unwrap();
    let snapshot = restored
        .prepare_window(WindowOrigin { x: 64, z: 64 })
        .unwrap();
    assert!(snapshot.world.occupied([48, 18, 48]));
    assert_eq!(restored.stats().sparse_edits, plan.placements.len());
    stream.undo().unwrap().unwrap();
    assert_eq!(stream.stats().sparse_edits, 0);
    assert!(!stream
        .prepare_window(WindowOrigin { x: 64, z: 64 })
        .unwrap()
        .world
        .occupied([48, 18, 48]));
    stream.redo().unwrap().unwrap();
    assert!(stream
        .prepare_window(WindowOrigin { x: 64, z: 64 })
        .unwrap()
        .world
        .occupied([48, 18, 48]));
}
