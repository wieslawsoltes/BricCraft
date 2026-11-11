use briccraft_import::{ElevationGrid, ImportStage, OsmOptions, TerrainOptions, VerticalDatum};
use serde_json::{json, Value};
use std::{collections::BTreeSet, time::Instant};

const MAP: &str = include_str!("../../briccraft-browser/wwwroot/assets/maps/telegraph-hill.json");
const PROVENANCE: &str =
    include_str!("../../briccraft-browser/wwwroot/assets/maps/telegraph-hill.provenance.json");
const MANIFEST: &str =
    include_str!("../../briccraft-browser/wwwroot/assets/terrain/telegraph-hill.json");
const DEM: &[u8] =
    include_bytes!("../../briccraft-browser/wwwroot/assets/terrain/telegraph-hill.dem");
const REFERENCE: &str =
    include_str!("../../briccraft-browser/wwwroot/assets/maps/telegraph-hill.reference.json");

#[test]
fn complete_telegraph_hill_source_graph_and_real_elevation_convert_through_worker() {
    let source: Value = serde_json::from_str(MAP).unwrap();
    let provenance: Value = serde_json::from_str(PROVENANCE).unwrap();
    let elements = source["elements"].as_array().unwrap();
    let identities: BTreeSet<_> = elements
        .iter()
        .map(|e| (e["type"].as_str().unwrap(), e["id"].as_i64().unwrap()))
        .collect();
    assert_eq!(elements.len(), 20_209);
    assert_eq!(identities.len(), elements.len());
    for kind in ["way", "relation"] {
        for id in provenance["supported_root_ids"][kind].as_array().unwrap() {
            assert!(
                identities.contains(&(kind, id.as_i64().unwrap())),
                "lost supported original {kind} {id}"
            );
        }
    }
    assert_eq!(
        provenance["supported_root_ids"]["way"]
            .as_array()
            .unwrap()
            .len(),
        2579
    );
    assert_eq!(
        provenance["supported_root_ids"]["relation"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    for element in elements {
        if let Some(nodes) = element["nodes"].as_array() {
            for node in nodes {
                assert!(identities.contains(&("node", node.as_i64().unwrap())));
            }
        }
        if let Some(members) = element["members"].as_array() {
            for member in members {
                assert!(identities.contains(&(
                    member["type"].as_str().unwrap(),
                    member["ref"].as_i64().unwrap()
                )));
            }
        }
    }
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let elevation: ElevationGrid = serde_json::from_str(
        &briccraft_import_worker::elevation_grid_json(&manifest["metadata"].to_string(), DEM)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(elevation.vertical_datum, VerticalDatum::Unknown);
    assert!(elevation.values.iter().all(Option::is_some));
    let mut options: OsmOptions = serde_json::from_str(
        &briccraft_import_worker::map_options_json(37.8024, -122.4058, 450.0, 8.0).unwrap(),
    )
    .unwrap();
    options.origin = [48, 18, 48];
    options.max_height_plates = Some(54);
    options.limits.max_parts = 120_000;
    options.limits.max_features = 24_000;
    options.terrain = Some(TerrainOptions {
        elevation,
        vertical_exaggeration: 1.0,
        fit_height: true,
        auto_coarsen: true,
        max_columns: 262_144,
    });
    let mut completed = false;
    let started = Instant::now();
    let plan = briccraft_import_worker::convert(
        "osm",
        MAP.as_bytes(),
        &serde_json::to_string(&options).unwrap(),
        &mut |p| {
            completed |= p.stage == ImportStage::Complete;
            true
        },
    )
    .unwrap();
    assert!(completed);
    let terrain = plan.report.terrain.as_ref().unwrap();
    let reference: Value = serde_json::from_str(REFERENCE).unwrap();
    let expected = &reference["expected"];
    assert_eq!(
        plan.report.parts,
        expected["parts"].as_u64().unwrap() as usize
    );
    assert_eq!(
        plan.report.voxels,
        expected["voxels"].as_u64().unwrap() as usize
    );
    assert_eq!(
        plan.report.buildings,
        expected["buildings"].as_u64().unwrap() as usize
    );
    assert_eq!(
        plan.report.roads,
        expected["roads"].as_u64().unwrap() as usize
    );
    assert_eq!(
        plan.report.water,
        expected["water"].as_u64().unwrap() as usize
    );
    assert_eq!(
        plan.report.landcover,
        expected["landcover"].as_u64().unwrap() as usize
    );
    assert!((terrain.elevation_min_m - expected["elevation_min_m"].as_f64().unwrap()).abs() < 1e-6);
    assert!((terrain.elevation_max_m - expected["elevation_max_m"].as_f64().unwrap()).abs() < 1e-6);
    assert!(
        (terrain.effective_meters_per_plate
            - expected["effective_meters_per_plate"].as_f64().unwrap())
        .abs()
            < 1e-8
    );
    assert_eq!(terrain.effective_meters_per_stud, 8.0);
    assert_eq!(terrain.vertical_datum, VerticalDatum::Unknown);
    assert!(terrain.elevation_max_m > 50.0 && terrain.elevation_max_m < 120.0);
    assert!(terrain.elevation_min_m < terrain.elevation_max_m - 30.0);
    assert!(plan.report.buildings > 100 && plan.report.roads > 50 && plan.report.landcover > 0);
    assert!(plan.report.bounds.min[1] == 18 && plan.report.bounds.max[1] <= 72);
    let projected = plan
        .report
        .projection
        .as_ref()
        .unwrap()
        .grid_bounds()
        .unwrap();
    assert_eq!(
        [
            plan.report.bounds.min[0],
            plan.report.bounds.min[2],
            plan.report.bounds.max[0],
            plan.report.bounds.max[2]
        ],
        projected
    );
    assert_eq!(
        terrain.columns,
        ((projected[2] - projected[0]) * (projected[3] - projected[1])) as usize
    );
    assert_eq!(plan.attribution[0].license, "ODbL-1.0");
    assert!(plan.attribution[1].text.contains("U.S. Geological Survey"));
    assert!(!plan
        .report
        .warnings
        .iter()
        .any(|w| w.contains("lack full node geometry")
            || w.contains("lack complete node geometry")
            || w.contains("multipolygon rings are incomplete")));
    // Exact non-overlap and cell count use the actual catalog part dimensions.
    let mut occupied = BTreeSet::new();
    for part in &plan.placements {
        let [sx, sy, sz] = part.size();
        for x in 0..sx as i32 {
            for y in 0..sy as i32 {
                for z in 0..sz as i32 {
                    assert!(occupied.insert([
                        part.position[0] + x,
                        part.position[1] + y,
                        part.position[2] + z
                    ]));
                }
            }
        }
    }
    assert_eq!(occupied.len(), plan.report.voxels);
    for x in projected[0]..projected[2] {
        for z in projected[1]..projected[3] {
            assert!(
                occupied.contains(&[x, 18, z]),
                "missing mapped terrain column {x},{z}"
            );
        }
    }
    let summary = json!({
        "parts":plan.report.parts,"voxels":plan.report.voxels,"buildings":plan.report.buildings,
        "roads":plan.report.roads,"water":plan.report.water,"landcover":plan.report.landcover,
        "inferred_heights":plan.report.inferred_heights,"bounds":plan.report.bounds,
        "terrain":terrain,"native_debug_seconds":started.elapsed().as_secs_f64()
    });
    println!(
        "TELEGRAPH_HILL_REPORT {}",
        serde_json::to_string(&summary).unwrap()
    );
}
