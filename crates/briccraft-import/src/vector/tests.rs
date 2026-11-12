use super::*;
use crate::{osm::data::Kind, ImportError, ImportLimits};

fn varint(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    while value >= 128 {
        bytes.push(value as u8 | 128);
        value >>= 7;
    }
    bytes.push(value as u8);
    bytes
}
fn field(number: u8, wire: u8, value: &[u8]) -> Vec<u8> {
    let mut bytes = varint(u64::from(number) * 8 + u64::from(wire));
    if wire == 2 {
        bytes.extend(varint(value.len() as u64));
    }
    bytes.extend(value);
    bytes
}
fn delta(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}
fn path(points: &[[i64; 2]], polygon: bool, cursor: &mut [i64; 2]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (i, point) in points.iter().enumerate() {
        if i == 0 {
            bytes.extend(varint(9));
        } else if i == 1 {
            bytes.extend(varint(((points.len() - 1) as u64) << 3 | 2));
        }
        for axis in 0..2 {
            bytes.extend(varint(delta(point[axis] - cursor[axis])));
        }
        *cursor = *point;
    }
    if polygon {
        bytes.extend(varint(15));
    }
    bytes
}
fn layer(name: &str, kind: u8, geometry: &[u8], properties: &[(&str, &str)]) -> Vec<u8> {
    let tags = (0..properties.len())
        .flat_map(|i| [i, i].into_iter().flat_map(|v| varint(v as u64)))
        .collect::<Vec<_>>();
    let mut feature = field(2, 2, &tags);
    feature.extend(field(3, 0, &[kind]));
    feature.extend(field(4, 2, geometry));
    let mut layer = field(1, 2, name.as_bytes());
    layer.extend(field(2, 2, &feature));
    for (key, _) in properties {
        layer.extend(field(3, 2, key.as_bytes()));
    }
    for (_, value) in properties {
        layer.extend(field(4, 2, &field(1, 2, value.as_bytes())));
    }
    layer.extend(field(5, 0, &varint(4096)));
    layer.extend(field(15, 0, &[2]));
    field(3, 2, &layer)
}
fn descriptor(length: usize) -> TileRange {
    TileRange {
        z: 14,
        x: 8192,
        y: 8192,
        offset: 0,
        length,
    }
}
fn square() -> Vec<u8> {
    path(
        &[[1980, 1980], [2116, 1980], [2116, 2116], [1980, 2116]],
        true,
        &mut [0; 2],
    )
}

#[test]
fn shortbread_preserves_polygon_holes_surface_roads_and_explicit_heights() {
    let mut cursor = [0; 2];
    let mut geometry = path(
        &[[1980, 1980], [2116, 1980], [2116, 2116], [1980, 2116]],
        true,
        &mut cursor,
    );
    geometry.extend(path(
        &[[2020, 2020], [2020, 2076], [2076, 2076], [2076, 2020]],
        true,
        &mut cursor,
    ));
    let mut bytes = layer("buildings", 3, &geometry, &[("height", "12")]);
    bytes.extend(layer(
        "streets",
        2,
        &path(&[[0, 1900], [4096, 1900]], false, &mut [0; 2]),
        &[("kind", "footway")],
    ));
    let mut features = Vec::new();
    let mut vertices = 0;
    shortbread::tile(
        &bytes,
        &descriptor(bytes.len()),
        &ImportLimits::default(),
        &mut features,
        &mut vertices,
    )
    .unwrap();
    assert_eq!(features.len(), 2);
    assert!(matches!(features[0].kind, Kind::Building));
    assert_eq!(features[0].outer.len(), 1);
    assert_eq!(features[0].inner.len(), 1);
    assert_eq!(features[0].tags["height"], "12");
    assert_eq!(features[0].osm_type, "vector_tile");
    assert_eq!(features[1].tags["highway"], "footway");
    assert_eq!(vertices, 10);
    let northwest = features[0].outer[0][0];
    let southeast = features[0].outer[0][2];
    assert!(northwest[0] > southeast[0] && northwest[1] < southeast[1]);
}

#[test]
fn shortbread_conversion_records_missing_heights_and_never_claims_source_osm_ids() {
    let bytes = layer("buildings", 3, &square(), &[]);
    let tile = descriptor(bytes.len());
    let latitude = (std::f64::consts::PI * (1.0 - 2.0 * 8192.5 / 16384.0))
        .sinh()
        .atan()
        .to_degrees();
    let longitude = 8192.5 / 16384.0 * 360.0 - 180.0;
    let options = OsmOptions {
        bounds: crate::GeoBounds::around(latitude, longitude, 100.0).unwrap(),
        meters_per_stud: 4.0,
        origin: [32, 0, 32],
        max_height_plates: Some(72),
        ..OsmOptions::default()
    };
    let plan = import_shortbread(&bytes, &[tile], &options, &mut |_| true).unwrap();
    assert_eq!(plan.report.buildings, 1);
    assert_eq!(plan.report.inferred_heights, 1);
    assert_eq!(plan.report.height_evidence[0].osm_type, "vector_tile");
    assert!(plan.report.height_evidence[0].osm_id < 0);
    assert!(!plan.placements.is_empty());
    assert!(plan.attribution[0].text.contains("OpenStreetMap"));
}

#[test]
fn malformed_geometry_varints_ranges_and_budgets_fail_before_any_plan() {
    for bytes in [vec![0x1a, 0xff], vec![0], vec![0xff; 11]] {
        let mut reader = protobuf::Reader::new(&bytes);
        assert!(reader.field().is_err());
    }
    let truncated = layer("buildings", 3, &[9, 0], &[]);
    assert!(import_shortbread(
        &truncated,
        &[descriptor(truncated.len())],
        &OsmOptions::default(),
        &mut |_| true
    )
    .is_err());
    let bytes = layer("buildings", 3, &square(), &[]);
    let mut options = OsmOptions::default();
    options.limits.max_vertices = 3;
    assert!(matches!(
        import_shortbread(&bytes, &[descriptor(bytes.len())], &options, &mut |_| true),
        Err(ImportError::BudgetExceeded { .. })
    ));
    let mut tile = descriptor(bytes.len());
    tile.offset = 1;
    assert!(import_shortbread(&bytes, &[tile], &OsmOptions::default(), &mut |_| true).is_err());
    assert!(matches!(
        import_shortbread(
            &bytes,
            &[descriptor(bytes.len())],
            &OsmOptions::default(),
            &mut |_| false
        ),
        Err(ImportError::Cancelled)
    ));
}

#[test]
fn buffered_tile_geometry_uses_exact_mercator_coordinates_at_shared_edges() {
    let mut vertices = 0;
    let left = descriptor(0);
    let right = TileRange {
        x: left.x + 1,
        ..left.clone()
    };
    let a = geometry::decode(
        &path(&[[4096, 100], [4096, 200]], false, &mut [0; 2]),
        2,
        4096,
        &left,
        &mut vertices,
        100,
    )
    .unwrap();
    let b = geometry::decode(
        &path(&[[0, 100], [0, 200]], false, &mut [0; 2]),
        2,
        4096,
        &right,
        &mut vertices,
        100,
    )
    .unwrap();
    assert_eq!(a[0].coordinates, b[0].coordinates);
}

#[test]
fn unsupported_layers_and_tunnels_do_not_invent_surface_geometry() {
    let mut bytes = layer("pois", 1, &[9, 0, 0], &[("kind", "cafe")]);
    bytes.extend(layer(
        "streets",
        2,
        &path(&[[0, 0], [100, 100]], false, &mut [0; 2]),
        &[("kind", "primary"), ("tunnel", "true")],
    ));
    let mut result = Vec::new();
    let mut vertices = 0;
    shortbread::tile(
        &bytes,
        &descriptor(bytes.len()),
        &ImportLimits::default(),
        &mut result,
        &mut vertices,
    )
    .unwrap();
    assert!(result.is_empty());
}

#[test]
fn aggregated_building_multipolygons_have_independent_foundations() {
    let mut cursor = [0; 2];
    let mut geometry = path(
        &[[100, 100], [200, 100], [200, 200], [100, 200]],
        true,
        &mut cursor,
    );
    geometry.extend(path(
        &[[400, 100], [500, 100], [500, 200], [400, 200]],
        true,
        &mut cursor,
    ));
    let bytes = layer("buildings", 3, &geometry, &[]);
    let mut features = Vec::new();
    let mut vertices = 0;
    shortbread::tile(
        &bytes,
        &descriptor(bytes.len()),
        &ImportLimits::default(),
        &mut features,
        &mut vertices,
    )
    .unwrap();
    assert_eq!(features.len(), 2);
    assert!(features
        .iter()
        .all(|f| f.outer.len() == 1 && f.inner.is_empty()));
}
