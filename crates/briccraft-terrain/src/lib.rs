//! Provider-independent, georeferenced elevation and bounded editable brick terrain.
//! Coordinates retain WGS84 horizontal positions and explicit vertical provenance;
//! no datum conversion or filling of missing elevations is performed implicitly.
mod error;
mod field;
mod grid;
mod packing;
mod projection;
mod terrarium;

pub use error::{TerrainError, TerrainResult};
pub use field::{HeightField, TerrainCell, TerrainOptions, TerrainReport, VerticalScale};
pub use grid::{ElevationGrid, ElevationSampler, ElevationSource, GridSpace, VerticalDatum};
pub use packing::{pack_heightfield, TerrainPackLimits};
pub use projection::{GeoBounds, GeoReference};
pub use terrarium::{decode_terrarium, TerrariumMosaic, TerrariumTile};

pub const MAX_ELEVATION_SAMPLES: usize = 1_048_576;
pub const MAX_TERRAIN_COLUMNS: usize = 262_144;
