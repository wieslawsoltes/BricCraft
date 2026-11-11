use crate::{
    error::{invalid, limit},
    ElevationGrid, ElevationSource, GeoBounds, GeoReference, TerrainError, TerrainResult,
    VerticalDatum, MAX_TERRAIN_COLUMNS,
};
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}
fn default_columns() -> usize {
    MAX_TERRAIN_COLUMNS
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainOptions {
    pub elevation: ElevationGrid,
    #[serde(default = "one")]
    pub vertical_exaggeration: f64,
    #[serde(default = "yes")]
    pub fit_height: bool,
    #[serde(default = "yes")]
    pub auto_coarsen: bool,
    #[serde(default = "default_columns")]
    pub max_columns: usize,
}
impl TerrainOptions {
    pub fn validate(&self) -> TerrainResult<()> {
        self.elevation.validate()?;
        if !self.vertical_exaggeration.is_finite()
            || !(0.05..=20.0).contains(&self.vertical_exaggeration)
        {
            return Err(invalid("vertical exaggeration must be between 0.05 and 20"));
        }
        if self.max_columns == 0 || self.max_columns > MAX_TERRAIN_COLUMNS {
            return Err(invalid(format!(
                "terrain column budget must be between 1 and {MAX_TERRAIN_COLUMNS}"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VerticalScale {
    pub reference_elevation_m: f64,
    pub requested_meters_per_plate: f64,
    pub effective_meters_per_plate: f64,
}
impl VerticalScale {
    pub fn fit(
        minimum: f64,
        maximum: f64,
        tallest_structure_m: f64,
        meters_per_stud: f64,
        exaggeration: f64,
        height_budget: u32,
        fit_height: bool,
    ) -> TerrainResult<Self> {
        if [
            minimum,
            maximum,
            tallest_structure_m,
            meters_per_stud,
            exaggeration,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || maximum < minimum
            || tallest_structure_m < 0.0
            || meters_per_stud <= 0.0
            || exaggeration <= 0.0
            || height_budget < 3
        {
            return Err(invalid(
                "terrain vertical scale or height budget is invalid",
            ));
        }
        let requested = meters_per_stud * 0.4 / exaggeration;
        // One ground plate plus conservative rounding space for a terrain step
        // and a building. No geometry is clipped to satisfy the resident height.
        let required = (maximum - minimum + tallest_structure_m) / (height_budget - 2) as f64;
        if !fit_height && required > requested + 1e-10 {
            return Err(invalid(format!("true-height terrain plus buildings exceeds {height_budget} plates; use at least {:.3} meters per stud or enable fit_height", required * exaggeration / 0.4)));
        }
        Ok(Self {
            reference_elevation_m: minimum,
            requested_meters_per_plate: requested,
            effective_meters_per_plate: requested.max(required),
        })
    }
    pub fn height_plates(&self, meters: f64) -> i32 {
        (meters / self.effective_meters_per_plate).round() as i32
    }
    pub fn surface_top(&self, elevation_m: f64, origin_y: i32) -> i32 {
        origin_y + 1 + self.height_plates((elevation_m - self.reference_elevation_m).max(0.0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainReport {
    pub source: ElevationSource,
    pub vertical_datum: VerticalDatum,
    pub coverage_bounds: GeoBounds,
    pub elevation_min_m: f64,
    pub elevation_max_m: f64,
    pub reference_elevation_m: f64,
    pub requested_meters_per_stud: f64,
    pub effective_meters_per_stud: f64,
    pub requested_meters_per_plate: f64,
    pub effective_meters_per_plate: f64,
    pub requested_vertical_exaggeration: f64,
    pub effective_vertical_exaggeration: f64,
    pub vertical_compression: f64,
    pub horizontal_coarsening: f64,
    pub columns: usize,
    pub filled_cells: usize,
    pub terrain_parts: usize,
    pub foundation_columns: usize,
    pub inferred_water_levels: usize,
    pub sampled_water_levels: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainCell {
    /// Exclusive top. The complete solid column occupies [field.base_y, top).
    pub top: i32,
    pub color: u8,
}

/// A dense 2D surface, not a dense 3D voxel allocation. It stores only one top
/// and material per column and packs its full solid volume directly into bricks.
pub struct HeightField {
    bounds: [i32; 4],
    pub base_y: i32,
    pub cells: Vec<TerrainCell>,
    pub scale: VerticalScale,
    pub minimum_m: f64,
    pub maximum_m: f64,
}
impl HeightField {
    pub fn sample(
        projection: &GeoReference,
        options: &TerrainOptions,
        tallest_structure_m: f64,
        height_budget: u32,
        callback: &mut dyn FnMut(usize, usize) -> bool,
    ) -> TerrainResult<Self> {
        options.validate()?;
        if !options.elevation.covers(projection.bounds) {
            return Err(TerrainError::MissingElevation(
                "the elevation grid does not cover the complete requested map bounds".into(),
            ));
        }
        let bounds = projection.grid_bounds()?;
        let width = (bounds[2] - bounds[0]) as usize;
        let depth = (bounds[3] - bounds[1]) as usize;
        let count = width
            .checked_mul(depth)
            .ok_or_else(|| invalid("terrain dimensions overflow"))?;
        limit("terrain columns", count, options.max_columns)?;
        let sampler = options.elevation.sampler()?;
        let mut elevations = Vec::with_capacity(count);
        let mut minimum_m = f64::INFINITY;
        let mut maximum_m = f64::NEG_INFINITY;
        for z in bounds[1]..bounds[3] {
            if !callback(elevations.len(), count) {
                return Err(TerrainError::Cancelled);
            }
            for x in bounds[0]..bounds[2] {
                let [lat, lon] = projection.unproject(x as f64 + 0.5, z as f64 + 0.5)?;
                // Boundary cells may straddle the map box by less than a stud.
                // Sample the nearest point INSIDE that box, never extrapolate DEM.
                let elevation = sampler.sample(
                    lat.clamp(projection.bounds.south, projection.bounds.north),
                    lon.clamp(projection.bounds.west, projection.bounds.east),
                )?;
                minimum_m = minimum_m.min(elevation);
                maximum_m = maximum_m.max(elevation);
                elevations.push(elevation);
            }
        }
        let scale = VerticalScale::fit(
            minimum_m,
            maximum_m,
            tallest_structure_m,
            projection.meters_per_stud,
            options.vertical_exaggeration,
            height_budget,
            options.fit_height,
        )?;
        let cells = elevations
            .into_iter()
            .map(|v| TerrainCell {
                top: scale.surface_top(v, projection.origin[1]),
                color: 29,
            })
            .collect();
        Ok(Self {
            bounds,
            base_y: projection.origin[1],
            cells,
            scale,
            minimum_m,
            maximum_m,
        })
    }
    pub fn bounds(&self) -> [i32; 4] {
        self.bounds
    }
    pub fn width(&self) -> usize {
        (self.bounds[2] - self.bounds[0]) as usize
    }
    pub fn depth(&self) -> usize {
        (self.bounds[3] - self.bounds[1]) as usize
    }
    pub fn index(&self, x: i32, z: i32) -> Option<usize> {
        (x >= self.bounds[0] && x < self.bounds[2] && z >= self.bounds[1] && z < self.bounds[3])
            .then(|| (z - self.bounds[1]) as usize * self.width() + (x - self.bounds[0]) as usize)
    }
    pub fn cell(&self, x: i32, z: i32) -> Option<&TerrainCell> {
        self.index(x, z).map(|i| &self.cells[i])
    }
    pub fn cell_mut(&mut self, x: i32, z: i32) -> Option<&mut TerrainCell> {
        let i = self.index(x, z)?;
        Some(&mut self.cells[i])
    }
    pub fn filled_cells(&self) -> usize {
        self.cells
            .iter()
            .map(|c| (c.top - self.base_y).max(0) as usize)
            .sum()
    }
    pub fn report(
        &self,
        options: &TerrainOptions,
        requested_mps: f64,
        effective_mps: f64,
    ) -> TerrainReport {
        TerrainReport {
            source: options.elevation.source.clone(),
            vertical_datum: options.elevation.vertical_datum,
            coverage_bounds: options.elevation.bounds,
            elevation_min_m: self.minimum_m,
            elevation_max_m: self.maximum_m,
            reference_elevation_m: self.scale.reference_elevation_m,
            requested_meters_per_stud: requested_mps,
            effective_meters_per_stud: effective_mps,
            requested_meters_per_plate: requested_mps * 0.4 / options.vertical_exaggeration,
            effective_meters_per_plate: self.scale.effective_meters_per_plate,
            requested_vertical_exaggeration: options.vertical_exaggeration,
            effective_vertical_exaggeration: effective_mps * 0.4
                / self.scale.effective_meters_per_plate,
            vertical_compression: self.scale.requested_meters_per_plate
                / self.scale.effective_meters_per_plate,
            horizontal_coarsening: effective_mps / requested_mps,
            columns: self.cells.len(),
            filled_cells: self.filled_cells(),
            terrain_parts: 0,
            foundation_columns: 0,
            inferred_water_levels: 0,
            sampled_water_levels: 0,
        }
    }
}
