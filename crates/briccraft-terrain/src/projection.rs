use crate::{error::invalid, TerrainResult};
use serde::{Deserialize, Serialize};

const EARTH_RADIUS_M: f64 = 6_378_137.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeoBounds {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}
impl GeoBounds {
    pub fn around(latitude: f64, longitude: f64, radius_m: f64) -> TerrainResult<Self> {
        if !latitude.is_finite()
            || latitude.abs() > 85.0
            || !longitude.is_finite()
            || longitude.abs() > 180.0
            || !radius_m.is_finite()
            || !(1.0..=10_000.0).contains(&radius_m)
        {
            return Err(invalid(
                "latitude/longitude or area radius is outside the supported local-map range",
            ));
        }
        let dy = (radius_m / EARTH_RADIUS_M).to_degrees();
        let dx = dy / latitude.to_radians().cos();
        let value = Self {
            south: latitude - dy,
            west: longitude - dx,
            north: latitude + dy,
            east: longitude + dx,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> TerrainResult<()> {
        if [self.south, self.west, self.north, self.east]
            .iter()
            .any(|v| !v.is_finite())
            || self.south >= self.north
            || self.west >= self.east
            || self.south < -85.0
            || self.north > 85.0
            || self.west < -180.0
            || self.east > 180.0
        {
            return Err(invalid("map bounds must be a finite, ordered box within ±85° latitude; split dateline-crossing areas"));
        }
        let width = (self.east - self.west).to_radians()
            * EARTH_RADIUS_M
            * ((self.south + self.north) * 0.5).to_radians().cos();
        let height = (self.north - self.south).to_radians() * EARTH_RADIUS_M;
        if width > 20_000.0 || height > 20_000.0 {
            return Err(invalid("use local map areas no larger than 20 km per side"));
        }
        Ok(())
    }
}

/// Local equirectangular projection for a bounded area: +X east, +Z south.
/// One vertical plate is 0.4 stud, preserving metric proportions in the brick grid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoReference {
    pub latitude: f64,
    pub longitude: f64,
    pub meters_per_stud: f64,
    pub origin: [i32; 3],
    pub bounds: GeoBounds,
    pub projection: String,
}
impl GeoReference {
    pub fn new(bounds: GeoBounds, meters_per_stud: f64, origin: [i32; 3]) -> TerrainResult<Self> {
        bounds.validate()?;
        if !meters_per_stud.is_finite() || !(0.05..=1000.0).contains(&meters_per_stud) {
            return Err(invalid("meters per stud must be between 0.05 and 1000"));
        }
        if origin.iter().any(|v| v.unsigned_abs() > 1_000_000_000) {
            return Err(invalid("grid origin is too large"));
        }
        Ok(Self {
            latitude: (bounds.south + bounds.north) * 0.5,
            longitude: (bounds.west + bounds.east) * 0.5,
            meters_per_stud,
            origin,
            bounds,
            projection: "local equirectangular; X east, Z south; Y plates".into(),
        })
    }
    pub fn project(&self, latitude: f64, longitude: f64) -> TerrainResult<[f64; 2]> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || latitude.abs() > 90.0
            || longitude.abs() > 180.0
        {
            return Err(invalid(
                "map coordinate is not finite WGS84 latitude/longitude",
            ));
        }
        Ok([
            self.origin[0] as f64
                + (longitude - self.longitude).to_radians()
                    * EARTH_RADIUS_M
                    * self.latitude.to_radians().cos()
                    / self.meters_per_stud,
            self.origin[2] as f64
                - (latitude - self.latitude).to_radians() * EARTH_RADIUS_M / self.meters_per_stud,
        ])
    }
    /// Inverse projection at a stud coordinate, returning [latitude, longitude].
    pub fn unproject(&self, x: f64, z: f64) -> TerrainResult<[f64; 2]> {
        if !x.is_finite() || !z.is_finite() {
            return Err(invalid("stud coordinate must be finite"));
        }
        Ok([
            self.latitude
                - ((z - self.origin[2] as f64) * self.meters_per_stud / EARTH_RADIUS_M)
                    .to_degrees(),
            self.longitude
                + ((x - self.origin[0] as f64) * self.meters_per_stud
                    / (EARTH_RADIUS_M * self.latitude.to_radians().cos()))
                .to_degrees(),
        ])
    }
    /// [minimum X, minimum Z, exclusive maximum X, exclusive maximum Z].
    pub fn grid_bounds(&self) -> TerrainResult<[i32; 4]> {
        let nw = self.project(self.bounds.north, self.bounds.west)?;
        let se = self.project(self.bounds.south, self.bounds.east)?;
        Ok([
            nw[0].floor() as i32,
            nw[1].floor() as i32,
            se[0].ceil() as i32,
            se[1].ceil() as i32,
        ])
    }
    pub fn height_plates(&self, meters: f64) -> i32 {
        (meters / (self.meters_per_stud * 0.4)).round() as i32
    }
}
