//! Backend-independent atlas packing for BricCraft's bounded GPU working set.
//!
//! The global world's size does not affect the static GPU volume: callers
//! supply one validated active window. Moving bodies have a separate local
//! volume and ID namespace so a window rebase never detaches their geometry.

mod atlas;
mod data_texture;
mod metrics;
mod part_lookup;

pub use atlas::{AtlasError, SceneAtlas};
pub use data_texture::PixelAtlas;
pub use metrics::{AtlasStats, MAX_ATLAS_TEXELS, MAX_PACKED_PARTS, MAX_VEHICLE_PARTS};
pub use part_lookup::PartLookup;
