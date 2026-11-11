use briccraft_terrain::*;

fn source() -> ElevationSource {
    ElevationSource {
        name: "Measured test grid".into(),
        url: "https://example.test/dem".into(),
        attribution: "Test survey".into(),
        license: "CC0-1.0".into(),
        license_url: String::new(),
        resolution_m: Some(5.0),
        timestamp: Some("2026-01-01".into()),
    }
}
fn grid() -> ElevationGrid {
    ElevationGrid {
        width: 2,
        height: 2,
        bounds: GeoBounds {
            south: 50.0,
            west: 10.0,
            north: 50.01,
            east: 10.01,
        },
        grid_space: GridSpace::Geographic,
        values: vec![Some(10.0), Some(30.0), Some(50.0), Some(70.0)],
        vertical_datum: VerticalDatum::Egm96,
        source: source(),
    }
}

#[test]
fn node_centers_orientation_and_bilinear_interpolation_are_explicit() {
    let grid = grid();
    let sample = grid.sampler().unwrap();
    assert_eq!(sample.sample(50.01, 10.0).unwrap(), 10.0);
    assert_eq!(sample.sample(50.0, 10.01).unwrap(), 70.0);
    assert!((sample.sample(50.005, 10.005).unwrap() - 40.0).abs() < 1e-8);
    assert!(sample.sample(49.99, 10.0).is_err());
    let json = serde_json::to_string(&grid).unwrap();
    let restored: ElevationGrid = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.vertical_datum, VerticalDatum::Egm96);
    assert_eq!(restored.source.timestamp.as_deref(), Some("2026-01-01"));
}

#[test]
fn a_void_sample_is_only_ignored_when_its_interpolation_weight_is_zero() {
    let mut grid = grid();
    grid.values[3] = None;
    let sample = grid.sampler().unwrap();
    assert_eq!(sample.sample(50.01, 10.0).unwrap(), 10.0);
    assert!(matches!(
        sample.sample(50.005, 10.005),
        Err(TerrainError::MissingElevation(_))
    ));
    grid.values[3] = Some(f32::NAN);
    assert!(grid.validate().is_err());
    grid.values.pop();
    assert!(grid.validate().is_err());
}

#[test]
fn mercator_rows_are_interpolated_in_the_declared_grid_space() {
    let mut grid = grid();
    grid.grid_space = GridSpace::WebMercator;
    let north = grid.bounds.north.to_radians().tan().asinh();
    let south = grid.bounds.south.to_radians().tan().asinh();
    let midpoint = ((north + south) * 0.5).sinh().atan().to_degrees();
    assert!((grid.sampler().unwrap().sample(midpoint, 10.005).unwrap() - 40.0).abs() < 1e-7);
}

#[test]
fn signed_big_endian_height_rasters_preserve_negative_heights_and_voids() {
    let bytes: Vec<_> = [-10i16, 0, 120, i16::MIN]
        .into_iter()
        .flat_map(i16::to_be_bytes)
        .collect();
    let raster =
        ElevationGrid::from_hgt(&bytes, 2, grid().bounds, source(), VerticalDatum::Egm96).unwrap();
    assert_eq!(
        raster.values,
        vec![Some(-10.0), Some(0.0), Some(120.0), None]
    );
    assert!(ElevationGrid::from_hgt(
        &bytes[..6],
        2,
        grid().bounds,
        source(),
        VerticalDatum::Egm96
    )
    .is_err());
}

fn tile(x: u32, y: u32, values: [u8; 4]) -> TerrariumTile {
    let rgb: Vec<_> = values.into_iter().flat_map(|v| [128, v, 0]).collect();
    TerrariumTile::from_rgb(1, x, y, 2, 2, &rgb).unwrap()
}
#[test]
fn terrarium_decoding_preserves_fractional_meters_and_interpolates_across_seams() {
    assert_eq!(decode_terrarium([128, 0, 128]), 0.5);
    assert_eq!(decode_terrarium([127, 255, 0]), -1.0);
    let tiles = vec![
        tile(0, 0, [0, 10, 20, 30]),
        tile(1, 0, [40, 50, 60, 70]),
        tile(0, 1, [80, 90, 100, 110]),
        tile(1, 1, [120, 130, 140, 150]),
    ];
    let mosaic = TerrariumMosaic::new(tiles).unwrap();
    // At lat/lon 0, both axes lie halfway between the four tile-border pixels.
    assert_eq!(mosaic.sample(0.0, 0.0).unwrap(), 75.0);
    let sampled = mosaic
        .sample_grid(
            GeoBounds::around(0.0, 0.0, 100.0).unwrap(),
            3,
            3,
            source(),
            VerticalDatum::Unknown,
        )
        .unwrap();
    assert_eq!(sampled.values[4], Some(75.0));
    let missing = TerrariumMosaic::new(vec![tile(0, 0, [0, 10, 20, 30])]).unwrap();
    assert!(matches!(
        missing.sample(0.0, 0.0),
        Err(TerrainError::MissingElevation(_))
    ));
    assert!(TerrariumMosaic::new(vec![tile(0, 0, [0; 4]), tile(0, 0, [0; 4])]).is_err());
}

#[test]
fn projection_roundtrip_and_vertical_fitting_preserve_reported_scales() {
    let bounds = GeoBounds::around(52.5, 13.4, 1500.0).unwrap();
    let p = GeoReference::new(bounds, 4.0, [48, 18, 48]).unwrap();
    let [x, z] = p.project(52.501, 13.399).unwrap();
    let [lat, lon] = p.unproject(x, z).unwrap();
    assert!((lat - 52.501).abs() < 1e-12 && (lon - 13.399).abs() < 1e-12);
    let scale = VerticalScale::fit(-20.0, 580.0, 100.0, 4.0, 1.0, 54, true).unwrap();
    assert!(scale.effective_meters_per_plate > scale.requested_meters_per_plate);
    assert!(scale.surface_top(580.0, 18) + scale.height_plates(100.0) <= 72);
    assert!(VerticalScale::fit(-20.0, 580.0, 100.0, 4.0, 1.0, 54, false).is_err());
}
