use super::{
    data::{Feature, Kind},
    geometry, OsmOptions,
};
use crate::{
    color,
    error::invalid,
    limits::{limit, progress},
    voxels::Voxels,
    GeoReference, HeightEvidence, ImportProgress, ImportReport, ImportResult, ImportStage,
};

/// OSM numeric lengths are meters unless an explicit feet suffix is present.
pub(super) fn meters(value: &str) -> Option<f64> {
    let value = value.trim();
    let numeric: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        .collect();
    let mut number = numeric.parse::<f64>().ok()?;
    let suffix = value[numeric.len()..].trim();
    if suffix == "ft" || suffix == "feet" || suffix.starts_with('\'') {
        number *= 0.3048;
    } else if !suffix.is_empty() && suffix != "m" && suffix != "meters" && suffix != "metres" {
        return None;
    }
    (number.is_finite() && number >= 0.0 && number <= 10_000.0).then_some(number)
}
pub(super) fn building_height(feature: &Feature, options: &OsmOptions) -> HeightEvidence {
    if let Some(height) = feature
        .tags
        .get("height")
        .or_else(|| feature.tags.get("building:height"))
        .and_then(|s| meters(s))
        .filter(|v| *v > 0.0)
    {
        HeightEvidence {
            osm_id: feature.id,
            osm_type: feature.osm_type.clone(),
            height_m: height,
            source: "height tag (meters or explicit feet)".into(),
            inferred: false,
        }
    } else if let Some(levels) = feature
        .tags
        .get("building:levels")
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 300.0)
    {
        HeightEvidence {
            osm_id: feature.id,
            osm_type: feature.osm_type.clone(),
            height_m: levels * 3.0,
            source: "building:levels × assumed 3 m floor height".into(),
            inferred: true,
        }
    } else {
        HeightEvidence {
            osm_id: feature.id,
            osm_type: feature.osm_type.clone(),
            height_m: options.default_height_m,
            source: "configured default; no valid mapped height or levels".into(),
            inferred: true,
        }
    }
}

pub(super) fn projected(
    rings: &[Vec<[f64; 2]>],
    projection: &GeoReference,
) -> ImportResult<Vec<Vec<[f64; 2]>>> {
    rings
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| projection.project(p[0], p[1]).map_err(Into::into))
                .collect()
        })
        .collect()
}
pub(super) fn bounds<'a>(
    points: impl Iterator<Item = &'a [f64; 2]>,
    clip: [i32; 4],
    padding: f64,
) -> [i32; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in points {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    }
    [
        ((b[0] - padding).floor() as i32).max(clip[0]),
        ((b[1] - padding).floor() as i32).max(clip[1]),
        ((b[2] + padding).ceil() as i32).min(clip[2]),
        ((b[3] + padding).ceil() as i32).min(clip[3]),
    ]
}

pub(super) fn render(
    features: &[Feature],
    projection: &GeoReference,
    options: &OsmOptions,
    grid: &mut Voxels<'_>,
    report: &mut ImportReport,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<()> {
    let clip = options.grid_bounds(projection)?;
    if options.include_ground {
        let count = ((clip[2] - clip[0]) as usize)
            .checked_mul((clip[3] - clip[1]) as usize)
            .ok_or_else(|| invalid("ground area exceeds the addressable grid"))?;
        limit("ground voxels", count, options.limits.max_voxels)?;
        for x in clip[0]..clip[2] {
            for z in clip[1]..clip[3] {
                grid.insert([x, options.origin[1], z], 29)?;
            }
            if x.rem_euclid(32) == 0 {
                progress(
                    callback,
                    ImportStage::Voxelize,
                    grid.work,
                    options.limits.max_work,
                )?;
            }
        }
    }
    for (index, feature) in features.iter().enumerate() {
        progress(callback, ImportStage::Voxelize, index, features.len())?;
        let outer = projected(&feature.outer, projection)?;
        let inner = projected(&feature.inner, projection)?;
        let line: Vec<_> = feature
            .line
            .iter()
            .map(|p| projection.project(p[0], p[1]).map_err(Into::into))
            .collect::<ImportResult<_>>()?;
        let surface = options.origin[1] + 1;
        let mut drawn = false;
        if !outer.is_empty() {
            let b = bounds(outer.iter().flatten(), clip, 0.0);
            if b[0] >= b[2] || b[1] >= b[3] {
                continue;
            }
            let height = if feature.kind == Kind::Building {
                Some(building_height(feature, options))
            } else {
                None
            };
            let minimum = feature
                .tags
                .get("min_height")
                .and_then(|s| meters(s))
                .unwrap_or(0.0);
            let bottom = surface + projection.height_plates(minimum);
            let top = height.as_ref().map_or(surface + 1, |h| {
                surface + projection.height_plates(h.height_m).max(1)
            });
            if top <= bottom {
                report.warn("A building with min_height at or above height was skipped.");
                continue;
            }
            if let Some(limit) = options.max_height_plates {
                if (top - options.origin[1]) as u32 > limit {
                    let suggested = height.as_ref().map_or(projection.meters_per_stud, |h| {
                        h.height_m / (limit.saturating_sub(1).max(1) as f64 * 0.4)
                    });
                    return Err(invalid(format!("mapped height needs {} plates above the origin, available {limit}; use at least {suggested:.2} meters per stud to preserve proportions",top-options.origin[1])));
                }
            }
            let base_color = match feature.kind {
                Kind::Building => feature
                    .tags
                    .get("building:colour")
                    .or_else(|| feature.tags.get("building:color"))
                    .and_then(|v| color::tag_color(v))
                    .unwrap_or(10),
                Kind::Road => 7,
                Kind::Water => 14,
                Kind::Landcover => landcover_color(feature),
            };
            let roof_color = feature
                .tags
                .get("roof:colour")
                .or_else(|| feature.tags.get("roof:color"))
                .and_then(|v| color::tag_color(v))
                .unwrap_or(base_color);
            for x in b[0]..b[2] {
                if x.rem_euclid(32) == 0 {
                    progress(callback, ImportStage::Voxelize, index, features.len())?;
                }
                for z in b[1]..b[3] {
                    grid.tick()?;
                    let p = [x as f64 + 0.5, z as f64 + 0.5];
                    if !geometry::in_polygon(&outer, &inner, p) {
                        continue;
                    }
                    drawn = true;
                    let wall = feature.kind == Kind::Building
                        && [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]]
                            .iter()
                            .any(|d| {
                                !geometry::in_polygon(&outer, &inner, [p[0] + d[0], p[1] + d[1]])
                            });
                    if feature.kind != Kind::Building || wall || options.solid_buildings {
                        for y in bottom..top {
                            grid.insert(
                                [x, y, z],
                                if y == top - 1 { roof_color } else { base_color },
                            )?;
                        }
                    } else {
                        grid.insert([x, bottom, z], base_color)?;
                        grid.insert([x, top - 1, z], roof_color)?;
                    }
                }
            }
            if let Some(height) = height.filter(|_| drawn) {
                report.inferred_heights += usize::from(height.inferred);
                report.height_evidence.push(height);
            }
        } else if line.len() >= 2 {
            let explicit = feature
                .tags
                .get("width")
                .and_then(|v| meters(v))
                .filter(|v| *v > 0.0);
            let width = explicit.unwrap_or_else(|| {
                if feature.kind == Kind::Water {
                    if feature.tags.get("waterway").is_some_and(|v| v == "river") {
                        8.0
                    } else {
                        2.0
                    }
                } else if let Some(lanes) = feature
                    .tags
                    .get("lanes")
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| v.is_finite() && *v > 0.0 && *v < 20.0)
                {
                    lanes * 3.0
                } else {
                    match feature.tags.get("highway").map(String::as_str) {
                        Some("footway" | "path" | "steps" | "cycleway") => 1.5,
                        Some("motorway" | "trunk" | "primary") => 9.0,
                        Some("secondary" | "tertiary") => 7.0,
                        _ => 5.0,
                    }
                }
            });
            let radius = (width / projection.meters_per_stud * 0.5).max(0.5);
            let color = if feature.kind == Kind::Water { 14 } else { 7 };
            for edge in line.windows(2) {
                let b = bounds(edge.iter(), clip, radius);
                for x in b[0]..b[2] {
                    for z in b[1]..b[3] {
                        grid.tick()?;
                        if geometry::segment_distance_squared(
                            [x as f64 + 0.5, z as f64 + 0.5],
                            edge[0],
                            edge[1],
                        ) <= radius * radius
                        {
                            grid.insert([x, surface, z], color)?;
                            drawn = true;
                        }
                    }
                }
            }
            if explicit.is_none() {
                report.inferred_widths += 1;
            }
        }
        if drawn {
            report.imported_features += 1;
            match feature.kind {
                Kind::Building => report.buildings += 1,
                Kind::Road => report.roads += 1,
                Kind::Water => report.water += 1,
                Kind::Landcover => report.landcover += 1,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn height_units_are_not_silently_misread() {
        assert_eq!(meters("10m"), Some(10.0));
        assert_eq!(meters("10 ft"), Some(3.048));
        assert_eq!(meters("10;20"), None);
        assert_eq!(meters("NaN"), None);
    }
}

/// Stylized material selection; source polygon boundaries remain unchanged.
pub(super) fn landcover_color(feature: &Feature) -> u8 {
    let tag = feature
        .tags
        .get("natural")
        .or_else(|| feature.tags.get("landuse"))
        .or_else(|| feature.tags.get("leisure"))
        .map(String::as_str)
        .unwrap_or("");
    match tag {
        "wood" | "forest" | "orchard" => 5,
        "scrub" | "heath" | "cemetery" => 25,
        "grassland" | "grass" | "meadow" | "park" | "garden" | "golf_course" | "pitch"
        | "recreation_ground" => 12,
        "sand" | "beach" | "construction" | "quarry" => 10,
        "bare_rock" => 6,
        "wetland" => 13,
        "glacier" => 3,
        "farmland" | "farmyard" | "allotments" | "vineyard" => 26,
        "industrial" | "commercial" | "retail" | "railway" | "garages" => 6,
        _ => 29,
    }
}
