//! The native-testable boundary for one atomic, geographically stable map region.
use briccraft_import::{
    osm::MapRegionOptions,
    vector::{import_shortbread, TileRange},
    GeoBounds, ImportPlan, ImportProgress, OsmOptions, TerrainOptions,
};
use briccraft_terrain::VerticalScale;
use serde::Deserialize;

#[derive(Deserialize)]
struct Request {
    latitude: f64,
    longitude: f64,
    meters_per_stud: f64,
    origin: [i32; 3],
    clip: [i32; 4],
    bounds: GeoBounds,
    tiles: Vec<TileRange>,
    #[serde(default)]
    vertical_scale: Option<VerticalScale>,
    #[serde(default = "enabled")]
    terrain: bool,
}
fn enabled() -> bool {
    true
}

pub fn convert_map_region(
    metadata: &str,
    vector_bytes: &[u8],
    elevation_metadata: &str,
    elevation_bytes: &[u8],
    progress: &mut dyn FnMut(ImportProgress) -> bool,
) -> Result<ImportPlan, String> {
    if metadata.len() > 32 * 1024 {
        return Err("Mapped region metadata exceeds 32 KiB".into());
    }
    let request: Request = serde_json::from_str(metadata).map_err(|e| e.to_string())?;
    if !(0..=45).contains(&request.origin[1]) {
        return Err("Mapped ground origin must leave room for terrain and height headroom".into());
    }
    if !request.terrain && (!elevation_metadata.is_empty() || !elevation_bytes.is_empty()) {
        return Err("Flat mapped regions must not include unexpected elevation data".into());
    }
    let terrain = if request.terrain {
        let source = super::elevation_grid_json(elevation_metadata, elevation_bytes)?;
        Some(TerrainOptions {
            elevation: serde_json::from_str(&source).map_err(|e| e.to_string())?,
            vertical_exaggeration: 1.0,
            fit_height: true,
            auto_coarsen: false,
            max_columns: 65536,
        })
    } else {
        None
    };
    let options = OsmOptions {
        region: Some(MapRegionOptions {
            latitude: request.latitude,
            longitude: request.longitude,
            clip: request.clip,
            vertical_scale: request.vertical_scale,
        }),
        terrain,
        bounds: request.bounds,
        meters_per_stud: request.meters_per_stud,
        origin: request.origin,
        max_height_plates: Some((72 - request.origin[1]) as u32),
        ..OsmOptions::default()
    };
    let mut plan = import_shortbread(vector_bytes, &request.tiles, &options, progress)
        .map_err(|e| e.to_string())?;
    plan.report.source_name = format!(
        "Mapped world · {:.4}, {:.4}",
        request.latitude, request.longitude
    );
    Ok(plan)
}
