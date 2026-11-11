//! GPU brick-world rendering through FerroUI's Skia custom-effect API.
//!
//! Geometry lives in two immutable data textures. Frames change uniforms only;
//! edits repack a bounded active window and publish only changed atlas images.
//! An active assembly has a separate local volume so driving never revoxelizes
//! the world, including when the static window changes underneath it.

mod camera;
mod effect;
mod frame;
mod renderer;

pub use briccraft_render_data::{
    AtlasError, AtlasStats, MAX_ATLAS_TEXELS, MAX_PACKED_PARTS, MAX_VEHICLE_PARTS,
};
pub use camera::Camera;
pub use effect::{BrickWorldEffect, WORLD_SKSL};
pub use frame::{PlacementPreview, RenderFrame, VehicleRender};
pub use renderer::Renderer;
