use briccraft_import_worker::convert_map_region;
use briccraft_terrain::{GeoBounds, GeoReference};
use serde_json::json;

fn fixture() -> (serde_json::Value, Vec<u8>) {
    let a = include_bytes!("../../../tests/fixtures/map-regions/berlin-14-8800-5373.mvt");
    let b = include_bytes!("../../../tests/fixtures/map-regions/berlin-14-8801-5373.mvt");
    let mut bytes = a.to_vec();
    bytes.extend(b);
    let anchor = GeoReference::new(
        GeoBounds::around(52.516275, 13.377704, 256.0).unwrap(),
        4.0,
        [64, 0, 64],
    )
    .unwrap();
    let nw = anchor.unproject(0.0, 0.0).unwrap();
    let se = anchor.unproject(128.0, 128.0).unwrap();
    (
        json!({"latitude":52.516275,"longitude":13.377704,"meters_per_stud":4,
        "origin":[64,0,64],"clip":[0,0,128,128],"terrain":false,
        "bounds":{"south":se[0],"west":nw[1],"north":nw[0],"east":se[1]},
        "tiles":[{"z":14,"x":8800,"y":5373,"offset":0,"length":a.len()},
            {"z":14,"x":8801,"y":5373,"offset":a.len(),"length":b.len()}]}),
        bytes,
    )
}

#[test]
fn real_shortbread_buildings_and_roads_convert_with_exact_region_ownership() {
    let (metadata, bytes) = fixture();
    let plan = convert_map_region(&metadata.to_string(), &bytes, "", &[], &mut |_| true).unwrap();
    println!(
        "Real Berlin region: {} buildings, {} roads, {} parts",
        plan.report.buildings, plan.report.roads, plan.report.parts
    );
    assert!(plan.report.buildings > 10);
    assert!(plan.report.roads > 10);
    assert_eq!(plan.report.inferred_heights, plan.report.buildings);
    assert!(plan.report.terrain.is_none());
    assert_eq!(plan.report.bounds.min[0], 0);
    assert_eq!(plan.report.bounds.max[0], 128);
    assert_eq!(plan.report.bounds.min[2], 0);
    assert_eq!(plan.report.bounds.max[2], 128);
    for p in &plan.placements {
        let s = p.size().map(i32::from);
        assert!(
            p.position[0] >= 0
                && p.position[2] >= 0
                && p.position[0] + s[0] <= 128
                && p.position[2] + s[2] <= 128
        );
    }
}

#[test]
fn worker_rejects_incomplete_or_cancelled_regions_before_publication() {
    let (metadata, bytes) = fixture();
    assert!(convert_map_region(
        &metadata.to_string(),
        &bytes[..bytes.len() - 1],
        "",
        &[],
        &mut |_| true
    )
    .is_err());
    let mut missing = metadata.clone();
    missing["tiles"].as_array_mut().unwrap().pop();
    let first_length = missing["tiles"][0]["length"].as_u64().unwrap() as usize;
    let error = convert_map_region(
        &missing.to_string(),
        &bytes[..first_length],
        "",
        &[],
        &mut |_| true,
    )
    .unwrap_err();
    assert!(error.contains("required for the complete region is missing"));
    assert!(convert_map_region(&metadata.to_string(), &bytes, "", &[], &mut |_| false).is_err());
    let mut terrain = metadata;
    terrain["terrain"] = true.into();
    assert!(convert_map_region(&terrain.to_string(), &bytes, "", &[], &mut |_| true).is_err());
}

#[test]
fn measured_terrain_passes_through_the_worker_and_retains_its_scale_in_adjacent_regions() {
    let (mut metadata, bytes) = fixture();
    metadata["terrain"] = true.into();
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, 256, 256);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[128, 100, 0].repeat(256 * 256))
            .unwrap();
    }
    let elevation = json!({"bounds":{"south":52.49,"west":13.34,"north":52.54,"east":13.43},
        "width":17,"height":17,"tiles":[{"z":0,"x":0,"y":0,"offset":0,"length":png_bytes.len()}],
        "vertical_datum":"Unknown","source":{"name":"Synthetic 100 m DEM","url":"https://example.test",
            "attribution":"Deterministic test DEM, not surveyed terrain","license":"CC0","license_url":"https://example.test/license"}});
    let first = convert_map_region(
        &metadata.to_string(),
        &bytes,
        &elevation.to_string(),
        &png_bytes,
        &mut |_| true,
    )
    .unwrap();
    let terrain = first.report.terrain.as_ref().unwrap();
    assert!((terrain.elevation_min_m - 100.0).abs() < 1e-9);
    assert!((terrain.elevation_max_m - 100.0).abs() < 1e-9);
    assert_eq!(terrain.columns, 128 * 128);
    assert!(terrain.terrain_parts > 0);
    assert_eq!(first.attribution.len(), 2);
    metadata["vertical_scale"] = json!({"reference_elevation_m":terrain.reference_elevation_m,
        "requested_meters_per_plate":terrain.requested_meters_per_plate,
        "effective_meters_per_plate":terrain.effective_meters_per_plate});
    metadata["clip"] = json!([128, 0, 256, 128]);
    let projection = first.report.projection.as_ref().unwrap();
    let nw = projection.unproject(128.0, 0.0).unwrap();
    let se = projection.unproject(256.0, 128.0).unwrap();
    metadata["bounds"] = json!({"south":se[0],"west":nw[1],"north":nw[0],"east":se[1]});
    let next = convert_map_region(
        &metadata.to_string(),
        &bytes,
        &elevation.to_string(),
        &png_bytes,
        &mut |_| true,
    )
    .unwrap();
    assert_eq!(
        next.report.terrain.as_ref().unwrap().reference_elevation_m,
        terrain.reference_elevation_m
    );
    assert_eq!(
        next.report
            .terrain
            .as_ref()
            .unwrap()
            .effective_meters_per_plate,
        terrain.effective_meters_per_plate
    );
    assert!(next
        .placements
        .iter()
        .all(|p| p.position[0] >= 128 && p.position[0] + i32::from(p.size()[0]) <= 256));
}

#[test]
fn berlin_walking_regression_has_a_clear_surface_route_across_the_next_region() {
    let (mut metadata, bytes) = fixture();
    metadata["meters_per_stud"] = 2.into();
    let projection = GeoReference::new(
        GeoBounds::around(52.516275, 13.377704, 128.0).unwrap(),
        2.0,
        [64, 0, 64],
    )
    .unwrap();
    let mut tops = std::collections::BTreeMap::<i32, i32>::new();
    for clip in [[0, 0, 128, 128], [128, 0, 256, 128]] {
        metadata["clip"] = json!(clip);
        let nw = projection
            .unproject(clip[0] as f64, clip[1] as f64)
            .unwrap();
        let se = projection
            .unproject(clip[2] as f64, clip[3] as f64)
            .unwrap();
        metadata["bounds"] = json!({"south":se[0],"west":nw[1],"north":nw[0],"east":se[1]});
        let plan =
            convert_map_region(&metadata.to_string(), &bytes, "", &[], &mut |_| true).unwrap();
        for part in plan.placements {
            let size = part.size().map(i32::from);
            if part.position[2] <= 64 && part.position[2] + size[2] > 64 {
                for x in part.position[0]..part.position[0] + size[0] {
                    tops.entry(x)
                        .and_modify(|v| *v = (*v).max(part.position[1] + size[1]))
                        .or_insert(part.position[1] + size[1]);
                }
            }
        }
    }
    for x in 88..=140 {
        let top = *tops.get(&x).expect("the route must have mapped ground");
        assert!(
            top <= 2,
            "a building blocks the browser test route at {x},64: {top} plates"
        );
    }
}
