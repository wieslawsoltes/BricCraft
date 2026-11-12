//! Fixed geographic coordinates for adjacent live-map regions.
use super::OsmOptions;
use crate::{error::invalid, GeoReference, ImportResult};
use briccraft_terrain::VerticalScale;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapRegionOptions {
    /// One shared WGS84 anchor for the complete roaming session.
    pub latitude: f64,
    pub longitude: f64,
    /// Exact integer cell edges [west, north, east, south], maximum excluded.
    pub clip: [i32; 4],
    /// None fits the initial region with headroom. Later regions reuse its scale.
    #[serde(default)]
    pub vertical_scale: Option<VerticalScale>,
}

impl MapRegionOptions {
    pub fn validate(&self) -> ImportResult<()> {
        if !self.latitude.is_finite()
            || self.latitude.abs() > 85.0
            || !self.longitude.is_finite()
            || self.longitude.abs() > 180.0
            || self.clip.iter().any(|v| v.unsigned_abs() > 10_000_000)
            || self.clip[0] >= self.clip[2]
            || self.clip[1] >= self.clip[3]
            || i64::from(self.clip[2]) - i64::from(self.clip[0]) > 256
            || i64::from(self.clip[3]) - i64::from(self.clip[1]) > 256
        {
            return Err(invalid("live map regions need a finite geographic anchor and ordered, bounded integer cell edges"));
        }
        if self.clip.iter().any(|v| v.rem_euclid(16) != 0) {
            return Err(invalid(
                "live map regions must align with 16-stud world chunks",
            ));
        }
        Ok(())
    }
}

impl OsmOptions {
    pub(super) fn projection(&self, meters_per_stud: f64) -> ImportResult<GeoReference> {
        let mut projection = GeoReference::new(self.bounds, meters_per_stud, self.origin)?;
        if let Some(region) = &self.region {
            region.validate()?;
            if (meters_per_stud - self.meters_per_stud).abs() > 1e-12 {
                return Err(invalid(
                    "live map regions cannot change their shared horizontal scale",
                ));
            }
            projection.latitude = region.latitude;
            projection.longitude = region.longitude;
            let northwest = projection.unproject(region.clip[0] as f64, region.clip[1] as f64)?;
            let southeast = projection.unproject(region.clip[2] as f64, region.clip[3] as f64)?;
            let wanted = [southeast[0], northwest[1], northwest[0], southeast[1]];
            let given = [
                self.bounds.south,
                self.bounds.west,
                self.bounds.north,
                self.bounds.east,
            ];
            if wanted.iter().zip(given).any(|(a, b)| (a - b).abs() > 1e-8) {
                return Err(invalid(
                    "live map bounds do not match their fixed projection and integer region",
                ));
            }
        }
        Ok(projection)
    }

    pub(super) fn grid_bounds(&self, projection: &GeoReference) -> ImportResult<[i32; 4]> {
        Ok(match &self.region {
            Some(region) => region.clip,
            None => projection.grid_bounds()?,
        })
    }
}
