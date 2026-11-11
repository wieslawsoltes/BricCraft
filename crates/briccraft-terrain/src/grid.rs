use crate::{
    error::{invalid, limit},
    GeoBounds, TerrainError, TerrainResult, MAX_ELEVATION_SAMPLES,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GridSpace {
    #[default]
    Geographic,
    WebMercator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VerticalDatum {
    Egm96,
    Egm2008,
    Wgs84Ellipsoid,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElevationSource {
    pub name: String,
    pub url: String,
    pub attribution: String,
    pub license: String,
    pub license_url: String,
    #[serde(default)]
    pub resolution_m: Option<f64>,
    #[serde(default)]
    pub timestamp: Option<String>,
}
impl ElevationSource {
    pub fn validate(&self) -> TerrainResult<()> {
        if self.name.trim().is_empty() || self.attribution.trim().is_empty() {
            return Err(invalid(
                "elevation source name and attribution are required",
            ));
        }
        for text in [
            &self.name,
            &self.url,
            &self.attribution,
            &self.license,
            &self.license_url,
        ] {
            limit("source metadata bytes per field", text.len(), 4096)?;
        }
        if let Some(timestamp) = &self.timestamp {
            limit("source timestamp bytes", timestamp.len(), 256)?;
        }
        if self
            .resolution_m
            .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 10_000_000.0)
        {
            return Err(invalid(
                "source resolution must be a finite positive number of meters",
            ));
        }
        Ok(())
    }
}

/// Samples are meters in row-major north-to-south, west-to-east order.
/// Bounds describe sample NODE CENTERS, not the outside edges of raster pixels.
/// A null sample is missing data; it is never interpreted as zero or sea level.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElevationGrid {
    pub width: u32,
    pub height: u32,
    pub bounds: GeoBounds,
    #[serde(default)]
    pub grid_space: GridSpace,
    pub values: Vec<Option<f32>>,
    #[serde(default)]
    pub vertical_datum: VerticalDatum,
    pub source: ElevationSource,
}
impl ElevationGrid {
    pub fn validate(&self) -> TerrainResult<()> {
        self.bounds.validate()?;
        self.source.validate()?;
        if self.width < 2 || self.height < 2 {
            return Err(invalid(
                "an elevation grid needs at least two rows and columns",
            ));
        }
        let count = (self.width as usize)
            .checked_mul(self.height as usize)
            .ok_or_else(|| invalid("elevation dimensions overflow"))?;
        limit("elevation samples", count, MAX_ELEVATION_SAMPLES)?;
        if self.values.len() != count {
            return Err(invalid(
                "elevation sample count does not match its dimensions",
            ));
        }
        if self
            .values
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 100_000.0)
        {
            return Err(invalid(
                "elevation samples must be finite meters between -100000 and 100000",
            ));
        }
        Ok(())
    }
    /// Validates once, then offers constant-time bilinear sampling.
    pub fn sampler(&self) -> TerrainResult<ElevationSampler<'_>> {
        self.validate()?;
        Ok(ElevationSampler { grid: self })
    }
    pub fn covers(&self, bounds: GeoBounds) -> bool {
        self.bounds.south <= bounds.south + 1e-10
            && self.bounds.north >= bounds.north - 1e-10
            && self.bounds.west <= bounds.west + 1e-10
            && self.bounds.east >= bounds.east - 1e-10
    }
    /// Decode a signed big-endian SRTM HGT raster. The standard -32768 void
    /// sentinel becomes None. Bounds and vertical datum must be supplied by the host.
    pub fn from_hgt(
        bytes: &[u8],
        side: u32,
        bounds: GeoBounds,
        source: ElevationSource,
        vertical_datum: VerticalDatum,
    ) -> TerrainResult<Self> {
        let count = (side as usize)
            .checked_mul(side as usize)
            .ok_or_else(|| invalid("HGT dimensions overflow"))?;
        limit("elevation samples", count, MAX_ELEVATION_SAMPLES)?;
        if side < 2 || bytes.len() != count * 2 {
            return Err(invalid("HGT byte length must equal side × side × 2"));
        }
        let values = bytes
            .chunks_exact(2)
            .map(|b| {
                let value = i16::from_be_bytes([b[0], b[1]]);
                (value != i16::MIN).then_some(value as f32)
            })
            .collect();
        let grid = Self {
            width: side,
            height: side,
            bounds,
            grid_space: GridSpace::Geographic,
            values,
            vertical_datum,
            source,
        };
        grid.validate()?;
        Ok(grid)
    }
}

pub struct ElevationSampler<'a> {
    grid: &'a ElevationGrid,
}
impl ElevationSampler<'_> {
    pub fn sample(&self, latitude: f64, longitude: f64) -> TerrainResult<f64> {
        let g = self.grid;
        let u = (longitude - g.bounds.west) / (g.bounds.east - g.bounds.west);
        let v = match g.grid_space {
            GridSpace::Geographic => {
                (g.bounds.north - latitude) / (g.bounds.north - g.bounds.south)
            }
            GridSpace::WebMercator => {
                (mercator(g.bounds.north) - mercator(latitude))
                    / (mercator(g.bounds.north) - mercator(g.bounds.south))
            }
        };
        if !u.is_finite()
            || !v.is_finite()
            || !(-1e-8..=1.00000001).contains(&u)
            || !(-1e-8..=1.00000001).contains(&v)
        {
            return Err(TerrainError::MissingElevation(format!(
                "coordinate {latitude:.7},{longitude:.7} is outside elevation coverage"
            )));
        }
        let x = u.clamp(0.0, 1.0) * (g.width - 1) as f64;
        let y = v.clamp(0.0, 1.0) * (g.height - 1) as f64;
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(g.width as usize - 1);
        let y1 = (y0 + 1).min(g.height as usize - 1);
        let dx = x - x0 as f64;
        let dy = y - y0 as f64;
        let mut result = 0.0;
        for (sx, sy, weight) in [
            (x0, y0, (1.0 - dx) * (1.0 - dy)),
            (x1, y0, dx * (1.0 - dy)),
            (x0, y1, (1.0 - dx) * dy),
            (x1, y1, dx * dy),
        ] {
            if weight > 1e-12 {
                let value = g.values[sy * g.width as usize + sx].ok_or_else(|| {
                    TerrainError::MissingElevation(format!(
                        "sample {sx},{sy} required at {latitude:.7},{longitude:.7} is void"
                    ))
                })?;
                result += value as f64 * weight;
            }
        }
        Ok(result)
    }
}
pub(crate) fn mercator(latitude: f64) -> f64 {
    latitude.to_radians().tan().asinh()
}
