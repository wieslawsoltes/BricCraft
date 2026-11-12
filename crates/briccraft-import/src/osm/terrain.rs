use super::{
    data::{Feature, Kind},
    raster, terrain_raster, OsmOptions,
};
use crate::{
    error::invalid, limits::progress, packing, voxels::Voxels, Attribution, GeoReference,
    GridBounds, ImportError, ImportPlan, ImportProgress, ImportReport, ImportResult, ImportStage,
};
use briccraft_terrain::{pack_heightfield, HeightField, TerrainPackLimits};

/// Preflight and whole-area retries are deterministic. A failed attempt is never
/// returned as a partial plan; the host publishes only the final atomic result.
pub(super) fn import(
    features: &[Feature],
    options: &OsmOptions,
    report: ImportReport,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    let terrain = options.terrain.as_ref().expect("terrain path selected");
    terrain.validate()?;
    if !options.include_ground {
        return Err(invalid("elevation terrain requires include_ground=true"));
    }
    if !(0..=69).contains(&options.origin[1]) {
        return Err(invalid(
            "elevation terrain origin must leave at least three plates in Y=0..72",
        ));
    }
    let height_budget = options
        .max_height_plates
        .unwrap_or((72 - options.origin[1]) as u32);
    if height_budget > (72 - options.origin[1]) as u32 {
        return Err(invalid(
            "elevation height budget exceeds the editable world's 72-plate vertical range",
        ));
    }
    let tallest = features
        .iter()
        .filter(|f| f.kind == Kind::Building)
        .map(|f| raster::building_height(f, options).height_m)
        .fold(0.0, f64::max);
    let mut scale = options.meters_per_stud;
    let mut last_budget = None;
    for attempt in 0..9 {
        let projection = options.projection(scale)?;
        let b = options.grid_bounds(&projection)?;
        let count = (b[2] - b[0]) as usize * (b[3] - b[1]) as usize;
        if count > terrain.max_columns {
            if !terrain.auto_coarsen || options.region.is_some() {
                return Err(ImportError::BudgetExceeded {
                    resource: "terrain columns",
                    limit: terrain.max_columns,
                });
            }
            scale = coarser(scale * ((count as f64 / terrain.max_columns as f64).sqrt() * 1.01))?;
            continue;
        }
        match attempt_import(
            features,
            options,
            &projection,
            height_budget,
            tallest,
            report.clone(),
            callback,
        ) {
            Ok(mut plan) => {
                if scale > options.meters_per_stud + 1e-9 {
                    plan.report.warn(format!("The complete mapped area was coarsened from {:.3} to {:.3} meters per stud to satisfy bounded column, feature, work and part budgets; no map extent was truncated.",options.meters_per_stud,scale));
                }
                return Ok(plan);
            }
            Err(e @ ImportError::BudgetExceeded { .. })
                if terrain.auto_coarsen && options.region.is_none() && attempt < 8 =>
            {
                last_budget = Some(e);
                scale = coarser(scale * 1.5)?;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_budget.unwrap_or(ImportError::BudgetExceeded {
        resource: "terrain columns",
        limit: terrain.max_columns,
    }))
}

fn coarser(value: f64) -> ImportResult<f64> {
    let value = (value * 1000.0).ceil() / 1000.0;
    if value > 1000.0 {
        return Err(invalid("the complete terrain cannot fit import budgets within the supported maximum 1000 meters per stud"));
    }
    Ok(value)
}

fn attempt_import(
    features: &[Feature],
    options: &OsmOptions,
    projection: &GeoReference,
    height_budget: u32,
    tallest: f64,
    mut report: ImportReport,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    let terrain = options.terrain.as_ref().unwrap();
    let mut sample_progress = |done, total| {
        callback(ImportProgress {
            stage: ImportStage::Project,
            completed: done,
            total,
        })
    };
    let mut field = if let Some(region) = &options.region {
        HeightField::sample_region(
            projection,
            terrain,
            tallest,
            height_budget,
            region.clip,
            region.vertical_scale,
            &mut sample_progress,
        )?
    } else {
        HeightField::sample(
            projection,
            terrain,
            tallest,
            height_budget,
            &mut sample_progress,
        )?
    };
    report.projection = Some(projection.clone());
    report.terrain =
        Some(field.report(terrain, options.meters_per_stud, projection.meters_per_stud));
    let mut grid = Voxels::new(&options.limits);
    terrain_raster::render(
        features,
        projection,
        options,
        &mut field,
        &mut grid,
        &mut report,
        callback,
    )?;
    let feature_voxels = grid.cells.len();
    let feature_bounds = grid.bounds();
    // Foundations are finalized before packing either layer, so terrain and
    // structures occupy disjoint cells and cannot collide during atomic import.
    let terrain_cells = field.filled_cells();
    let mut placements = packing::pack(grid, &mut |mut p| {
        if p.stage == ImportStage::Complete {
            p.stage = ImportStage::Pack;
        }
        callback(p)
    })?;
    let terrain_parts = pack_heightfield(
        &field,
        TerrainPackLimits {
            max_parts: options.limits.max_parts.saturating_sub(placements.len()),
            max_work: options.limits.max_work,
        },
        &mut |done, total| {
            callback(ImportProgress {
                stage: ImportStage::Pack,
                completed: done,
                total,
            })
        },
    )?;
    let terrain_part_count = terrain_parts.len();
    placements.extend(terrain_parts);
    let b = field.bounds();
    let highest = field
        .cells
        .iter()
        .map(|c| c.top)
        .max()
        .unwrap_or(field.base_y);
    let mut bounds = GridBounds {
        min: [b[0], field.base_y, b[1]],
        max: [b[2], highest, b[3]],
    };
    if feature_voxels > 0 {
        for a in 0..3 {
            bounds.min[a] = bounds.min[a].min(feature_bounds.min[a]);
            bounds.max[a] = bounds.max[a].max(feature_bounds.max[a]);
        }
    }
    report.bounds = bounds;
    report.voxels = feature_voxels + terrain_cells;
    report.parts = placements.len();
    let terrain_report = report.terrain.as_mut().unwrap();
    terrain_report.filled_cells = terrain_cells;
    terrain_report.terrain_parts = terrain_part_count;
    let compressed = terrain_report.vertical_compression < 0.999999;
    let effective_mpp = terrain_report.effective_meters_per_plate;
    let datum = terrain_report.vertical_datum;
    if options.region.is_some() {
        report.warn("Live map regions share one fixed geographic projection and vertical reference. Their integer cell edges are exact; elevation and height scales never refit independently between regions. The initial region reserves twelve plates of lower and upper terrain headroom.");
    } else {
        report.warn("The complete area is sampled from the attributed elevation grid. Heights are relative to its sampled minimum; source vertical datum is retained without an implicit datum conversion. Boundary cells sample the nearest point within the requested box.");
    }
    report.warn("Terrain is quantized to editable plates. Roads and land-cover surfaces follow the sampled terrain; building foundations rise to the highest terrain cell beneath each mapped footprint. Facades and unmapped roof geometry are not inferred from the elevation model.");
    if compressed {
        report.warn(format!("Vertical relief and building heights share an explicit compressed scale of {effective_mpp:.4} meters per plate to fit the resident height. Disable fit_height for strict metric proportions."));
    }
    if datum == briccraft_terrain::VerticalDatum::Unknown {
        report.warn("The elevation provider's vertical datum is mixed or unspecified; absolute heights are not asserted to be EGM96, EGM2008 or ellipsoidal heights.");
    }
    let source = &terrain.elevation.source;
    let attribution = vec![
        Attribution::openstreetmap(),
        Attribution {
            text: source.attribution.clone(),
            url: source.url.clone(),
            license: source.license.clone(),
            license_url: source.license_url.clone(),
        },
    ];
    progress(callback, ImportStage::Complete, report.parts, report.parts)?;
    Ok(ImportPlan {
        placements,
        report,
        attribution,
    })
}
