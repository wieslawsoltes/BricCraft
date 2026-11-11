use crate::{
    error::{invalid, limit},
    grid::mercator,
    ElevationGrid, ElevationSource, GeoBounds, GridSpace, TerrainError, TerrainResult,
    VerticalDatum, MAX_ELEVATION_SAMPLES,
};
use std::collections::BTreeMap;

/// Mapzen Terrarium encoding, in meters: R × 256 + G + B / 256 - 32768.
pub fn decode_terrarium(rgb: [u8; 3]) -> f32 {
    rgb[0] as f32 * 256.0 + rgb[1] as f32 + rgb[2] as f32 / 256.0 - 32768.0
}

#[derive(Clone, Debug)]
pub struct TerrariumTile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    heights: Vec<f32>,
}
impl TerrariumTile {
    /// RGB bytes must be raw decoded channel values, without color-space conversion.
    pub fn from_rgb(
        z: u8,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgb: &[u8],
    ) -> TerrainResult<Self> {
        if z > 22
            || x >= (1u32 << z)
            || y >= (1u32 << z)
            || width != height
            || !(2..=1024).contains(&width)
            || !width.is_power_of_two()
        {
            return Err(invalid(
                "Terrarium tile coordinates or square pixel dimensions are invalid",
            ));
        }
        let count = width as usize * height as usize;
        if rgb.len() != count * 3 {
            return Err(invalid(
                "Terrarium RGB byte count does not match dimensions",
            ));
        }
        let heights = rgb
            .chunks_exact(3)
            .map(|v| decode_terrarium([v[0], v[1], v[2]]))
            .collect();
        Ok(Self {
            z,
            x,
            y,
            width,
            height,
            heights,
        })
    }
}

/// Bounded, same-zoom tile mosaic. Bilinear interpolation uses pixel centers and
/// reads across tile seams. Missing neighbors are errors, never edge replication.
pub struct TerrariumMosaic {
    tiles: BTreeMap<(u32, u32), TerrariumTile>,
    z: u8,
    side: u32,
}
impl TerrariumMosaic {
    pub fn new(tiles: Vec<TerrariumTile>) -> TerrainResult<Self> {
        limit("Terrarium tiles", tiles.len(), 64)?;
        let first = tiles
            .first()
            .ok_or_else(|| invalid("Terrarium mosaic is empty"))?;
        let z = first.z;
        let side = first.width;
        if z > 22 || !(2..=1024).contains(&side) || !side.is_power_of_two() {
            return Err(invalid("Terrarium mosaic has invalid zoom or dimensions"));
        }
        limit(
            "Terrarium decoded pixels",
            tiles.len() * side as usize * side as usize,
            4_194_304,
        )?;
        let mut map = BTreeMap::new();
        for tile in tiles {
            if tile.z != z || tile.width != side || tile.height != side {
                return Err(invalid(
                    "Terrarium mosaic tiles must have the same zoom and dimensions",
                ));
            }
            if tile.x >= (1u32 << z)
                || tile.y >= (1u32 << z)
                || tile.heights.len() != side as usize * side as usize
            {
                return Err(invalid(
                    "Terrarium tile metadata was changed after decoding",
                ));
            }
            if map.insert((tile.x, tile.y), tile).is_some() {
                return Err(invalid("duplicate Terrarium mosaic tile"));
            }
        }
        Ok(Self {
            tiles: map,
            z,
            side,
        })
    }
    pub fn sample(&self, latitude: f64, longitude: f64) -> TerrainResult<f64> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || latitude.abs() > 85.0
            || longitude.abs() > 180.0
        {
            return Err(invalid(
                "Terrarium sample coordinate is outside supported WGS84 bounds",
            ));
        }
        let extent = (1u64 << self.z) * self.side as u64;
        // Slippy-map coordinates denote pixel edges; subtract half a pixel to
        // interpolate the samples recorded at each pixel center.
        let x = (longitude + 180.0) / 360.0 * extent as f64 - 0.5;
        let y = (1.0 - mercator(latitude) / std::f64::consts::PI) * 0.5 * extent as f64 - 0.5;
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let dx = x - ix as f64;
        let dy = y - iy as f64;
        let mut result = 0.0;
        for (sx, sy, w) in [
            (ix, iy, (1.0 - dx) * (1.0 - dy)),
            (ix + 1, iy, dx * (1.0 - dy)),
            (ix, iy + 1, (1.0 - dx) * dy),
            (ix + 1, iy + 1, dx * dy),
        ] {
            if w <= 1e-12 {
                continue;
            }
            let sx = sx.rem_euclid(extent as i64) as u32;
            if sy < 0 || sy >= extent as i64 {
                return Err(TerrainError::MissingElevation(
                    "Terrarium sample exceeds the Mercator tile grid".into(),
                ));
            }
            let sy = sy as u32;
            let tile = self
                .tiles
                .get(&(sx / self.side, sy / self.side))
                .ok_or_else(|| {
                    TerrainError::MissingElevation(format!(
                        "Terrarium tile {}/{}/{} required for seam interpolation",
                        self.z,
                        sx / self.side,
                        sy / self.side
                    ))
                })?;
            result +=
                tile.heights[(sy % self.side * self.side + sx % self.side) as usize] as f64 * w;
        }
        Ok(result)
    }
    pub fn sample_grid(
        &self,
        bounds: GeoBounds,
        width: u32,
        height: u32,
        source: ElevationSource,
        vertical_datum: VerticalDatum,
    ) -> TerrainResult<ElevationGrid> {
        bounds.validate()?;
        source.validate()?;
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| invalid("grid dimensions overflow"))?;
        limit("elevation samples", count, MAX_ELEVATION_SAMPLES)?;
        if width < 2 || height < 2 {
            return Err(invalid("sample grid needs at least two rows and columns"));
        }
        let mut values = Vec::with_capacity(count);
        for y in 0..height {
            let lat = bounds.north - (bounds.north - bounds.south) * y as f64 / (height - 1) as f64;
            for x in 0..width {
                let lon = bounds.west + (bounds.east - bounds.west) * x as f64 / (width - 1) as f64;
                values.push(Some(self.sample(lat, lon)? as f32));
            }
        }
        Ok(ElevationGrid {
            width,
            height,
            bounds,
            grid_space: GridSpace::Geographic,
            values,
            vertical_datum,
            source,
        })
    }
}
