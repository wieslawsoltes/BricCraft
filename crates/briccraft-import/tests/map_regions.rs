use briccraft_import::{
    osm::MapRegionOptions, GeoBounds, GeoReference, OsmOptions, TerrainOptions,
};
use briccraft_terrain::{ElevationGrid, ElevationSource, GridSpace, VerticalDatum, VerticalScale};
use std::collections::BTreeMap;

fn source() -> ElevationSource {
    ElevationSource {
        name: "Synthetic slope".into(),
        url: "https://example.test".into(),
        attribution: "Generated deterministic test fixture".into(),
        license: "CC0".into(),
        license_url: "https://example.test/license".into(),
        resolution_m: Some(1.0),
        timestamp: None,
    }
}

fn options(clip: [i32; 4], fixed: Option<VerticalScale>) -> OsmOptions {
    let area = GeoBounds::around(52.0, 13.0, 400.0).unwrap();
    let projection = GeoReference::new(area, 2.0, [16, 0, 16]).unwrap();
    let northwest = projection
        .unproject(clip[0] as f64, clip[1] as f64)
        .unwrap();
    let southeast = projection
        .unproject(clip[2] as f64, clip[3] as f64)
        .unwrap();
    OsmOptions {
        region: Some(MapRegionOptions {
            latitude: 52.0,
            longitude: 13.0,
            clip,
            vertical_scale: fixed,
        }),
        bounds: GeoBounds {
            south: southeast[0],
            west: northwest[1],
            north: northwest[0],
            east: southeast[1],
        },
        meters_per_stud: 2.0,
        origin: [16, 0, 16],
        max_height_plates: Some(72),
        terrain: Some(TerrainOptions {
            elevation: ElevationGrid {
                width: 5,
                height: 5,
                bounds: area,
                grid_space: GridSpace::Geographic,
                values: (0..25)
                    .map(|i| Some(100.0 + (i % 5) as f32 * 12.0))
                    .collect(),
                vertical_datum: VerticalDatum::Unknown,
                source: source(),
            },
            vertical_exaggeration: 1.0,
            fit_height: true,
            auto_coarsen: false,
            max_columns: 65536,
        }),
        ..OsmOptions::default()
    }
}

fn columns(plan: &briccraft_import::ImportPlan) -> BTreeMap<[i32; 2], i32> {
    let mut columns = BTreeMap::<[i32; 2], i32>::new();
    for p in &plan.placements {
        let s = p.size().map(i32::from);
        for x in p.position[0]..p.position[0] + s[0] {
            for z in p.position[2]..p.position[2] + s[2] {
                columns
                    .entry([x, z])
                    .and_modify(|y| *y = (*y).max(p.position[1] + s[1]))
                    .or_insert(p.position[1] + s[1]);
            }
        }
    }
    columns
}

#[test]
fn adjacent_regions_share_integer_edges_geography_and_vertical_datum() {
    let empty = r#"{"elements":[]}"#;
    let first =
        briccraft_import::import_overpass_json(empty, &options([0, 0, 32, 32], None)).unwrap();
    let report = first.report.terrain.as_ref().unwrap();
    let fixed = VerticalScale {
        reference_elevation_m: report.reference_elevation_m,
        requested_meters_per_plate: report.requested_meters_per_plate,
        effective_meters_per_plate: report.effective_meters_per_plate,
    };
    let next =
        briccraft_import::import_overpass_json(empty, &options([32, 0, 64, 32], Some(fixed)))
            .unwrap();
    let complete =
        briccraft_import::import_overpass_json(empty, &options([0, 0, 64, 32], Some(fixed)))
            .unwrap();
    let mut combined = columns(&first);
    combined.extend(columns(&next));
    assert_eq!(combined.len(), 64 * 32);
    assert_eq!(combined, columns(&complete));
    assert_eq!(
        next.report.terrain.as_ref().unwrap().reference_elevation_m,
        fixed.reference_elevation_m
    );
    for (plan, clip) in [(&first, [0, 0, 32, 32]), (&next, [32, 0, 64, 32])] {
        for part in &plan.placements {
            let s = part.size().map(i32::from);
            assert!(part.position[0] >= clip[0] && part.position[2] >= clip[1]);
            assert!(part.position[0] + s[0] <= clip[2] && part.position[2] + s[2] <= clip[3]);
        }
    }
}

#[test]
fn fixed_scale_overflow_and_mismatched_geography_fail_without_refitting() {
    let fixed = VerticalScale {
        reference_elevation_m: 0.0,
        requested_meters_per_plate: 0.8,
        effective_meters_per_plate: 0.8,
    };
    let error = briccraft_import::import_overpass_json(
        r#"{"elements":[]}"#,
        &options([0, 0, 32, 32], Some(fixed)),
    )
    .unwrap_err();
    assert!(error.to_string().contains("fixed vertical range"));
    let mut bad = options([-32, 0, 0, 32], None);
    bad.region.as_mut().unwrap().longitude += 0.01;
    assert!(briccraft_import::import_overpass_json(r#"{"elements":[]}"#, &bad).is_err());
}

#[test]
fn buildings_and_water_crossing_regions_use_the_same_heights_as_one_complete_conversion() {
    let projection = GeoReference::new(
        GeoBounds::around(52.0, 13.0, 400.0).unwrap(),
        2.0,
        [16, 0, 16],
    )
    .unwrap();
    let ring = |points: &[[f64; 2]]| {
        points
            .iter()
            .map(|p| {
                let [lat, lon] = projection.unproject(p[0], p[1]).unwrap();
                serde_json::json!({"lat":lat,"lon":lon})
            })
            .collect::<Vec<_>>()
    };
    let source = serde_json::json!({"elements":[
        {"type":"way","id":1,"tags":{"building":"yes","height":"6"},
            "geometry":ring(&[[20.0,4.0],[44.0,4.0],[44.0,12.0],[20.0,12.0],[20.0,4.0]])},
        {"type":"way","id":2,"tags":{"natural":"water"},
            "geometry":ring(&[[0.0,16.0],[64.0,16.0],[64.0,32.0],[0.0,32.0],[0.0,16.0]])}
    ]})
    .to_string();
    let first =
        briccraft_import::import_overpass_json(&source, &options([0, 0, 32, 32], None)).unwrap();
    let t = first.report.terrain.as_ref().unwrap();
    let scale = VerticalScale {
        reference_elevation_m: t.reference_elevation_m,
        requested_meters_per_plate: t.requested_meters_per_plate,
        effective_meters_per_plate: t.effective_meters_per_plate,
    };
    let second =
        briccraft_import::import_overpass_json(&source, &options([32, 0, 64, 32], Some(scale)))
            .unwrap();
    let full =
        briccraft_import::import_overpass_json(&source, &options([0, 0, 64, 32], Some(scale)))
            .unwrap();
    let mut joined = columns(&first);
    joined.extend(columns(&second));
    assert_eq!(joined, columns(&full));
    assert_eq!(
        joined[&[30, 8]],
        joined[&[33, 8]],
        "The roof stays level over a region seam"
    );
    assert_eq!(
        first.report.terrain.as_ref().unwrap().inferred_water_levels,
        0
    );
}
