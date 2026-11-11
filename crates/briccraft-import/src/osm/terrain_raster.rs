use super::{
    data::{Feature, Kind},
    geometry, raster, OsmOptions,
};
use crate::{
    color, error::invalid, limits::progress, voxels::Voxels, GeoReference, ImportProgress,
    ImportReport, ImportResult, ImportStage,
};
use briccraft_terrain::HeightField;

pub(super) fn render(
    features: &[Feature],
    projection: &GeoReference,
    options: &OsmOptions,
    field: &mut HeightField,
    grid: &mut Voxels<'_>,
    report: &mut ImportReport,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<()> {
    let clip = field.bounds();
    for (index, feature) in features.iter().enumerate() {
        progress(callback, ImportStage::Voxelize, index, features.len())?;
        let outer = raster::projected(&feature.outer, projection)?;
        let inner = raster::projected(&feature.inner, projection)?;
        let line: Vec<_> = feature
            .line
            .iter()
            .map(|p| projection.project(p[0], p[1]).map_err(Into::into))
            .collect::<ImportResult<_>>()?;
        let mut drawn = false;
        if !outer.is_empty() {
            let b = raster::bounds(outer.iter().flatten(), clip, 0.0);
            let mut footprint = Vec::new();
            for x in b[0]..b[2] {
                if x.rem_euclid(32) == 0 {
                    progress(callback, ImportStage::Voxelize, index, features.len())?;
                }
                for z in b[1]..b[3] {
                    grid.tick()?;
                    if geometry::in_polygon(&outer, &inner, [x as f64 + 0.5, z as f64 + 0.5]) {
                        footprint.push([x, z]);
                    }
                }
            }
            if footprint.is_empty() {
                continue;
            }
            drawn = true;
            match feature.kind {
                Kind::Building => building(
                    feature, &footprint, &outer, &inner, options, field, grid, report,
                )?,
                Kind::Water => water(feature, &footprint, field, report),
                Kind::Landcover | Kind::Road => {
                    let color = if feature.kind == Kind::Road {
                        7
                    } else {
                        raster::landcover_color(feature)
                    };
                    for [x, z] in footprint {
                        field.cell_mut(x, z).unwrap().color = color;
                    }
                }
            }
        } else if line.len() >= 2 && matches!(feature.kind, Kind::Water | Kind::Road) {
            let (width, inferred) = line_width(feature);
            let radius = (width / projection.meters_per_stud * 0.5).max(0.5);
            let color = if feature.kind == Kind::Water { 14 } else { 7 };
            for edge in line.windows(2) {
                let b = raster::bounds(edge.iter(), clip, radius);
                for x in b[0]..b[2] {
                    for z in b[1]..b[3] {
                        grid.tick()?;
                        if geometry::segment_distance_squared(
                            [x as f64 + 0.5, z as f64 + 0.5],
                            edge[0],
                            edge[1],
                        ) <= radius * radius
                        {
                            field.cell_mut(x, z).unwrap().color = color;
                            drawn = true;
                        }
                    }
                }
            }
            if inferred {
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
    // OSM commonly supplies overlapping whole buildings and building:part
    // footprints. A later foundation can bury earlier feature voxels; removing
    // precisely that intersection retains one solid, non-overlapping result.
    let before = grid.cells.len();
    grid.cells
        .retain(|p, _| field.cell(p[0], p[2]).is_none_or(|c| p[1] >= c.top));
    if before != grid.cells.len() {
        report.warn("Overlapping mapped building foundations were merged with the terrain; buried feature cells were removed to retain one editable solid per cell.");
    }
    Ok(())
}

fn building(
    feature: &Feature,
    footprint: &[[i32; 2]],
    outer: &[Vec<[f64; 2]>],
    inner: &[Vec<[f64; 2]>],
    options: &OsmOptions,
    field: &mut HeightField,
    grid: &mut Voxels<'_>,
    report: &mut ImportReport,
) -> ImportResult<()> {
    let height = raster::building_height(feature, options);
    let minimum = feature
        .tags
        .get("min_height")
        .and_then(|s| raster::meters(s))
        .unwrap_or(0.0);
    let surface = footprint
        .iter()
        .map(|[x, z]| field.cell(*x, *z).unwrap().top)
        .max()
        .unwrap();
    let bottom = surface + field.scale.height_plates(minimum);
    let top = surface + field.scale.height_plates(height.height_m).max(1);
    if top <= bottom {
        report.warn("A building with min_height at or above height was skipped.");
        return Ok(());
    }
    if top > 72
        || options
            .max_height_plates
            .is_some_and(|limit| top - options.origin[1] > limit as i32)
    {
        return Err(invalid("elevation plus mapped building exceeds the validated vertical budget; use a coarser scale"));
    }
    let base_color = feature
        .tags
        .get("building:colour")
        .or_else(|| feature.tags.get("building:color"))
        .and_then(|v| color::tag_color(v))
        .unwrap_or(10);
    let roof_color = feature
        .tags
        .get("roof:colour")
        .or_else(|| feature.tags.get("roof:color"))
        .and_then(|v| color::tag_color(v))
        .unwrap_or(base_color);
    for &[x, z] in footprint {
        let c = field.cell_mut(x, z).unwrap();
        if c.top < surface {
            c.top = surface;
            c.color = 6;
            report.terrain.as_mut().unwrap().foundation_columns += 1;
        }
        let p = [x as f64 + 0.5, z as f64 + 0.5];
        let wall = [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]]
            .iter()
            .any(|d| !geometry::in_polygon(outer, inner, [p[0] + d[0], p[1] + d[1]]));
        if wall || options.solid_buildings {
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
    report.inferred_heights += usize::from(height.inferred);
    report.height_evidence.push(height);
    Ok(())
}

fn water(
    feature: &Feature,
    footprint: &[[i32; 2]],
    field: &mut HeightField,
    report: &mut ImportReport,
) {
    let flowing = feature.tags.get("water").is_some_and(|v| v == "river")
        || feature
            .tags
            .get("waterway")
            .is_some_and(|v| matches!(v.as_str(), "riverbank" | "river" | "stream" | "canal"));
    let level = if flowing {
        None
    } else {
        let mut tops: Vec<_> = footprint
            .iter()
            .map(|[x, z]| field.cell(*x, *z).unwrap().top)
            .collect();
        let mid = tops.len() / 2;
        tops.select_nth_unstable(mid);
        Some(tops[mid])
    };
    for &[x, z] in footprint {
        let c = field.cell_mut(x, z).unwrap();
        if let Some(top) = level {
            c.top = top;
        }
        c.color = 14;
    }
    if level.is_some() {
        report.terrain.as_mut().unwrap().inferred_water_levels += 1;
        report.warn("Enclosed water surfaces use the median DEM ground level inside each mapped polygon, an inferred level rather than bathymetry or a measured water stage. Flowing waterways follow DEM terrain.");
    }
    if feature.tags.contains_key("ele") {
        report.warn("Water ele tags are retained in the source extract but are not mixed with DEM heights because no common vertical-datum conversion is configured.");
    }
}

fn line_width(feature: &Feature) -> (f64, bool) {
    if let Some(width) = feature
        .tags
        .get("width")
        .and_then(|v| raster::meters(v))
        .filter(|v| *v > 0.0)
    {
        return (width, false);
    }
    let width = if feature.kind == Kind::Water {
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
    };
    (width, true)
}
