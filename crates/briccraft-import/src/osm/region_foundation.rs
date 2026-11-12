//! Whole-footprint foundation sampling, independent from the publishing region.
use super::{geometry, raster, OsmOptions};
use crate::{error::invalid, voxels::Voxels, GeoReference, ImportResult};
use briccraft_terrain::HeightField;

pub(super) fn surface(
    outer: &[Vec<[f64; 2]>],
    inner: &[Vec<[f64; 2]>],
    projection: &GeoReference,
    options: &OsmOptions,
    field: &HeightField,
    grid: &mut Voxels<'_>,
) -> ImportResult<i32> {
    let terrain = options.terrain.as_ref().expect("terrain path selected");
    let sampler = terrain.elevation.sampler()?;
    let bounds = raster::bounds(
        outer.iter().flatten(),
        [-100_000_000, -100_000_000, 100_000_000, 100_000_000],
        0.0,
    );
    let mut surface = None;
    for x in bounds[0]..bounds[2] {
        for z in bounds[1]..bounds[3] {
            grid.tick()?;
            if !geometry::in_polygon(outer, inner, [x as f64 + 0.5, z as f64 + 0.5]) {
                continue;
            }
            let [latitude, longitude] = projection.unproject(x as f64 + 0.5, z as f64 + 0.5)?;
            let elevation = sampler.sample(latitude, longitude)?;
            if elevation < field.scale.reference_elevation_m - 1e-6 {
                return Err(invalid(
                    "a complete mapped foundation is below the journey's fixed height range",
                ));
            }
            let top = field.scale.surface_top(elevation, field.base_y);
            surface = Some(surface.map_or(top, |value: i32| value.max(top)));
        }
    }
    surface.ok_or_else(|| {
        invalid("a mapped building has no complete foundation at this stud resolution")
    })
}
