use super::{
    geometry,
    protobuf::{self, Reader, Value},
    TileRange,
};
use crate::{
    error::invalid,
    limits::limit,
    osm::data::{Feature, Kind},
    ImportLimits, ImportResult,
};
use std::collections::BTreeMap;

pub fn tile(
    bytes: &[u8],
    tile: &TileRange,
    limits: &ImportLimits,
    result: &mut Vec<Feature>,
    vertices: &mut usize,
) -> ImportResult<()> {
    let mut reader = Reader::new(bytes);
    let mut layers = 0;
    while let Some((number, value)) = reader.field()? {
        if let (3, Value::Bytes(bytes)) = (number, value) {
            layers += 1;
            limit("MVT layers", layers, 128)?;
            layer(bytes, tile, limits, result, vertices)?;
        }
    }
    Ok(())
}

fn layer(
    bytes: &[u8],
    tile: &TileRange,
    limits: &ImportLimits,
    result: &mut Vec<Feature>,
    vertices: &mut usize,
) -> ImportResult<()> {
    let mut reader = Reader::new(bytes);
    let mut name = String::new();
    let mut extent = 4096u32;
    let mut version = 0;
    while let Some((number, value)) = reader.field()? {
        match (number, value) {
            (1, Value::Bytes(v)) => name = protobuf::string(v)?,
            (5, Value::Varint(v)) => {
                extent = u32::try_from(v).map_err(|_| invalid("MVT extent overflows"))?
            }
            (15, Value::Varint(v)) => version = v,
            _ => {}
        }
    }
    if !matches!(version, 1 | 2) || !(256..=65536).contains(&extent) {
        return Err(invalid(
            "MVT layer needs version 1 or 2 and a 256–65536 extent",
        ));
    }
    if !matches!(
        name.as_str(),
        "buildings"
            | "land"
            | "sites"
            | "ocean"
            | "water_polygons"
            | "water_lines"
            | "streets"
            | "street_polygons"
    ) {
        return Ok(());
    }
    let mut keys = Vec::new();
    let mut values = Vec::new();
    let mut features = Vec::new();
    let mut reader = Reader::new(bytes);
    while let Some((number, value)) = reader.field()? {
        match (number, value) {
            (2, Value::Bytes(v)) => {
                limit(
                    "MVT source features",
                    features.len() + 1,
                    limits.max_features,
                )?;
                features.push(v);
            }
            (3, Value::Bytes(v)) => {
                limit("MVT tag keys", keys.len() + 1, 4096)?;
                keys.push(protobuf::string(v)?);
            }
            (4, Value::Bytes(v)) => {
                limit("MVT tag values", values.len() + 1, 16384)?;
                values.push(protobuf::scalar(v)?);
            }
            _ => {}
        }
    }
    for bytes in features {
        let mut reader = Reader::new(bytes);
        let mut indices = Vec::new();
        let mut geometry = None;
        let mut geometry_kind = 0;
        while let Some((number, value)) = reader.field()? {
            match (number, value) {
                (2, Value::Bytes(v)) => {
                    let mut packed = Reader::new(v);
                    while !packed.empty() {
                        limit("MVT feature tags", indices.len() + 1, 256)?;
                        indices.push(packed.varint()?);
                    }
                }
                (3, Value::Varint(v)) => geometry_kind = v,
                (4, Value::Bytes(v)) => {
                    if geometry.replace(v).is_some() {
                        return Err(invalid("duplicate MVT geometry field"));
                    }
                }
                _ => {}
            }
        }
        if indices.len() % 2 != 0 {
            return Err(invalid("MVT tags need key/value pairs"));
        }
        let mut properties = BTreeMap::new();
        let mut property_bytes = 0usize;
        for pair in indices.chunks_exact(2) {
            let key = keys
                .get(usize::try_from(pair[0]).map_err(|_| invalid("MVT key index overflows"))?)
                .ok_or_else(|| invalid("MVT key index is outside the layer dictionary"))?;
            let value = values
                .get(usize::try_from(pair[1]).map_err(|_| invalid("MVT value index overflows"))?)
                .ok_or_else(|| invalid("MVT value index is outside the layer dictionary"))?;
            property_bytes = property_bytes.saturating_add(key.len() + value.len());
            limit("MVT feature metadata bytes", property_bytes, 2048)?;
            properties.insert(key.clone(), value.clone());
        }
        let Some((kind, tags)) = tags(&name, &properties, geometry_kind) else {
            continue;
        };
        let bytes = geometry.ok_or_else(|| invalid("supported MVT feature is missing geometry"))?;
        let paths = geometry::decode(
            bytes,
            geometry_kind,
            extent,
            tile,
            vertices,
            limits.max_vertices,
        )?;
        if geometry_kind == 3 {
            let mut outer = Vec::new();
            let mut inner = Vec::new();
            for path in paths {
                if path.exterior {
                    // Providers commonly aggregate all same-tag buildings into
                    // one MultiPolygon. Each exterior and its following holes
                    // is an independent footprint with its own foundation.
                    if !outer.is_empty() {
                        push(
                            result,
                            limits,
                            kind,
                            tags.clone(),
                            std::mem::take(&mut outer),
                            std::mem::take(&mut inner),
                            Vec::new(),
                        )?;
                    }
                    outer.push(path.coordinates);
                } else {
                    if outer.is_empty() {
                        return Err(invalid("MVT polygon starts with an interior ring"));
                    }
                    inner.push(path.coordinates);
                }
            }
            if !outer.is_empty() {
                push(result, limits, kind, tags, outer, inner, Vec::new())?;
            }
        } else {
            for path in paths {
                push(
                    result,
                    limits,
                    kind,
                    tags.clone(),
                    Vec::new(),
                    Vec::new(),
                    path.coordinates,
                )?;
            }
        }
    }
    Ok(())
}

fn push(
    result: &mut Vec<Feature>,
    limits: &ImportLimits,
    kind: Kind,
    tags: BTreeMap<String, String>,
    outer: Vec<Vec<[f64; 2]>>,
    inner: Vec<Vec<[f64; 2]>>,
    line: Vec<[f64; 2]>,
) -> ImportResult<()> {
    limit("map elements", result.len() + 1, limits.max_features)?;
    // Vector feature identifiers are not necessarily source OSM identifiers.
    // A negative bundle-local ID and explicit kind avoid claiming otherwise.
    result.push(Feature {
        id: -(result.len() as i64) - 1,
        osm_type: "vector_tile".into(),
        kind,
        tags,
        outer,
        inner,
        line,
    });
    Ok(())
}

fn tags(
    layer: &str,
    values: &BTreeMap<String, String>,
    geometry: u64,
) -> Option<(Kind, BTreeMap<String, String>)> {
    if !matches!(geometry, 2 | 3) {
        return None;
    }
    let value = values.get("kind").map(String::as_str).unwrap_or("");
    if values
        .get("tunnel")
        .is_some_and(|v| matches!(v.as_str(), "true" | "1" | "yes"))
    {
        return None; // The surface world does not fabricate underground routes.
    }
    let mut tags = BTreeMap::new();
    let kind = match layer {
        "buildings" if geometry == 3 => {
            tags.insert("building".into(), "yes".into());
            Kind::Building
        }
        "ocean" | "water_polygons" if geometry == 3 && value != "glacier" => {
            tags.insert("natural".into(), "water".into());
            tags.insert(
                "water".into(),
                if layer == "ocean" { "ocean" } else { value }.into(),
            );
            Kind::Water
        }
        "water_lines" if geometry == 2 => {
            tags.insert("waterway".into(), value.into());
            Kind::Water
        }
        "streets" | "street_polygons" => {
            if values
                .get("rail")
                .is_some_and(|v| matches!(v.as_str(), "true" | "1" | "yes"))
                || matches!(
                    value,
                    "rail" | "subway" | "tram" | "light_rail" | "monorail"
                )
            {
                return None;
            }
            tags.insert("highway".into(), value.into());
            if geometry == 3 {
                tags.insert("area".into(), "yes".into());
            }
            Kind::Road
        }
        "land" | "sites" | "water_polygons" if geometry == 3 => {
            let key = match value {
                "wood" | "scrub" | "grassland" | "heath" | "sand" | "beach" | "bare_rock"
                | "wetland" | "glacier" => "natural",
                "park" | "garden" | "golf_course" | "pitch" | "recreation_ground" => "leisure",
                _ => "landuse",
            };
            tags.insert(key.into(), value.into());
            Kind::Landcover
        }
        _ => return None,
    };
    for key in [
        "height",
        "building:height",
        "building:levels",
        "min_height",
        "width",
        "lanes",
        "building:colour",
        "building:color",
        "roof:colour",
        "roof:color",
        "name",
    ] {
        if let Some(value) = values.get(key) {
            tags.insert(key.into(), value.clone());
        }
    }
    Some((kind, tags))
}
