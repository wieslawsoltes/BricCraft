//! OSM geometry is a source of mapped footprints and tags, not missing 3D scans.
mod data;
mod geometry;
mod raster;
mod terrain;
mod terrain_raster;

use crate::{
    error::invalid, limits::progress, packing, voxels::Voxels, Attribution, GeoBounds,
    GeoReference, ImportLimits, ImportPlan, ImportProgress, ImportReport, ImportResult,
    ImportStage, SourceKind,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsmOptions {
    /// Optional measured elevation. None preserves the legacy flat import path.
    #[serde(default)]
    pub terrain: Option<briccraft_terrain::TerrainOptions>,
    pub bounds: GeoBounds,
    pub meters_per_stud: f64,
    pub origin: [i32; 3],
    pub default_height_m: f64,
    pub solid_buildings: bool,
    pub include_ground: bool,
    /// Optional resident-height budget, measured above origin. Heights are never clamped.
    pub max_height_plates: Option<u32>,
    pub limits: ImportLimits,
}
impl Default for OsmOptions {
    fn default() -> Self {
        Self {
            terrain: None,
            bounds: GeoBounds {
                south: 52.515,
                west: 13.376,
                north: 52.517,
                east: 13.380,
            },
            meters_per_stud: 4.0,
            origin: [0, 0, 0],
            default_height_m: 9.0,
            solid_buildings: false,
            include_ground: true,
            max_height_plates: None,
            limits: ImportLimits::default(),
        }
    }
}

/// Construct one bounded read-only query. The host applies endpoint policy,
/// attribution, caching, user initiation, request pacing and retry-after handling.
pub fn overpass_query(bounds: GeoBounds) -> ImportResult<String> {
    bounds.validate()?;
    let b = format!(
        "{:.7},{:.7},{:.7},{:.7}",
        bounds.south, bounds.west, bounds.north, bounds.east
    );
    Ok(format!("[out:json][timeout:25][maxsize:16777216];(way[building]({b});relation[building]({b});way[\"building:part\"]({b});relation[\"building:part\"]({b});way[highway]({b});way[natural=water]({b});relation[natural=water]({b});way[waterway]({b});relation[waterway=riverbank]({b});way[landuse=reservoir]({b});relation[landuse=reservoir]({b});way[landuse]({b});relation[landuse]({b});way[natural~\"^(wood|scrub|grassland|heath|sand|beach|bare_rock|wetland|glacier)$\"]({b});relation[natural~\"^(wood|scrub|grassland|heath|sand|beach|bare_rock|wetland|glacier)$\"]({b});way[leisure~\"^(park|garden|golf_course|pitch|recreation_ground)$\"]({b});relation[leisure~\"^(park|garden|golf_course|pitch|recreation_ground)$\"]({b}););out tags geom;"))
}

pub fn import_overpass_json(source: &str, options: &OsmOptions) -> ImportResult<ImportPlan> {
    import_overpass_json_with_progress(source, options, &mut |_| true)
}

pub fn import_overpass_json_with_progress(
    source: &str,
    options: &OsmOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    options.limits.input(source.len())?;
    if options.max_height_plates == Some(0) {
        return Err(invalid("map height budget must be positive"));
    }
    if !options.default_height_m.is_finite() || !(0.5..=1000.0).contains(&options.default_height_m)
    {
        return Err(invalid(
            "default building height must be between 0.5 and 1000 meters",
        ));
    }
    let projection = GeoReference::new(options.bounds, options.meters_per_stud, options.origin)?;
    progress(callback, ImportStage::Parse, 0, source.len())?;
    let mut report = ImportReport::new(SourceKind::OpenStreetMap);
    report.source_name = "OpenStreetMap extract".into();
    report.projection = Some(projection.clone());
    let features = data::parse(source, &options.limits, &mut report)?;
    progress(
        callback,
        ImportStage::Project,
        features.len(),
        features.len(),
    )?;
    if options.terrain.is_some() {
        return terrain::import(&features, options, report, callback);
    }
    let mut grid = Voxels::new(&options.limits);
    raster::render(
        &features,
        &projection,
        options,
        &mut grid,
        &mut report,
        callback,
    )?;
    if grid.cells.is_empty() {
        return Err(crate::ImportError::EmptyGeometry);
    }
    if report.imported_features == 0 {
        report.warn("No supported mapped feature intersects this extract at the selected resolution; only the ground baseline was created.");
    }
    report.voxels = grid.cells.len();
    report.bounds = grid.bounds();
    let placements = packing::pack(grid, callback)?;
    report.parts = placements.len();
    report.warn("Mapped footprints and tagged heights are retained at the selected stud resolution. Missing heights, road widths and colors are inferred and identified in this report.");
    report.warn("No terrain elevation or facade mesh is provided by this extract. Ground is a flat baseline; roofs use the footprint outline without inventing unmapped roof details.");
    Ok(ImportPlan {
        placements,
        report,
        attribution: vec![Attribution::openstreetmap()],
    })
}
