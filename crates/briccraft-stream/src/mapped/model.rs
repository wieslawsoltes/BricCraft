use crate::{GlobalPlacement, StreamError};
use serde::{Deserialize, Serialize};

/// One fixed projection and vertical reference for an entire mapped session.
/// X points east, Z south, and Y is in plates. Subsequent regions must use these
/// exact scales; independently fitting their heights would create seams.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAnchor {
    pub latitude: f64,
    pub longitude: f64,
    pub meters_per_stud: f64,
    pub origin: [i32; 3],
    pub reference_elevation_m: f64,
    pub meters_per_plate: f64,
    pub region_size: u16,
    #[serde(default = "terrain_enabled")]
    pub terrain: bool,
}

fn terrain_enabled() -> bool {
    true
}

impl MapAnchor {
    pub fn validate(&self) -> Result<(), StreamError> {
        if !self.latitude.is_finite()
            || self.latitude.abs() > 85.0
            || !self.longitude.is_finite()
            || self.longitude.abs() > 180.0
            || !self.meters_per_stud.is_finite()
            || !(0.05..=1_000.0).contains(&self.meters_per_stud)
            || !self.meters_per_plate.is_finite()
            || !(0.001..=10_000.0).contains(&self.meters_per_plate)
            || !self.reference_elevation_m.is_finite()
            || !(-15_000.0..=20_000.0).contains(&self.reference_elevation_m)
            || !(96..=512).contains(&self.region_size)
            || self.region_size % 16 != 0
            || self.origin.iter().any(|v| v.unsigned_abs() > 1_000_000_000)
            || !(0..=69).contains(&self.origin[1])
        {
            return Err(StreamError::InvalidSave(
                "Invalid fixed map projection, height scale or region extent".into(),
            ));
        }
        Ok(())
    }

    pub fn region_at(&self, x: i64, z: i64) -> Result<[i64; 4], StreamError> {
        let size = i64::from(self.region_size);
        let x = x
            .div_euclid(size)
            .checked_mul(size)
            .ok_or(StreamError::CoordinateOverflow)?;
        let z = z
            .div_euclid(size)
            .checked_mul(size)
            .ok_or(StreamError::CoordinateOverflow)?;
        Ok([
            x,
            z,
            x.checked_add(size).ok_or(StreamError::CoordinateOverflow)?,
            z.checked_add(size).ok_or(StreamError::CoordinateOverflow)?,
        ])
    }

    /// Geographic coordinates for status/maps, retaining the session projection.
    pub fn unproject(&self, x: f64, z: f64) -> Result<[f64; 2], StreamError> {
        self.validate()?;
        if !x.is_finite() || !z.is_finite() {
            return Err(StreamError::InvalidPosition);
        }
        let radius = 6_378_137.0;
        let latitude = self.latitude
            - ((z - self.origin[2] as f64) * self.meters_per_stud / radius).to_degrees();
        let longitude = self.longitude
            + ((x - self.origin[0] as f64) * self.meters_per_stud
                / (radius * self.latitude.to_radians().cos()))
            .to_degrees();
        if latitude.abs() > 85.0 || longitude.abs() > 180.0 {
            return Err(StreamError::InvalidPosition);
        }
        Ok([latitude, longitude])
    }
}

/// A complete, disjoint geographic source region. Geometry is never cropped by
/// the stream; converters must pack every part entirely inside these bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapRegion {
    /// Minimum X/Z and exclusive maximum X/Z in global studs.
    pub bounds: [i64; 4],
    pub placements: Vec<GlobalPlacement>,
}

pub(crate) fn intersects(a: [i64; 4], b: [i64; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

pub(crate) fn contains(bounds: [i64; 4], x: i64, z: i64) -> bool {
    x >= bounds[0] && x < bounds[2] && z >= bounds[1] && z < bounds[3]
}
