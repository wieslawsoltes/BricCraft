//! Offline, deterministic conversion of map data and static meshes into bricks.
//! Coordinates are global signed stud/plate positions, never clipped to a local
//! world's resident window. The caller owns network access and chunk streaming.
#![forbid(unsafe_code)]

mod color;
mod error;
mod limits;
pub mod mesh;
mod model;
pub mod osm;
mod packing;
mod projection;
mod voxels;

pub use error::{ImportError, ImportResult};
pub use limits::{ImportLimits, ImportProgress, ImportStage};
pub use mesh::{import_glb, import_gltf_json, import_obj, MeshOptions};
pub use model::{Attribution, GridBounds, HeightEvidence, ImportPlan, ImportReport, SourceKind};
pub use osm::{import_overpass_json, overpass_query, OsmOptions};
pub use projection::{GeoBounds, GeoReference};
