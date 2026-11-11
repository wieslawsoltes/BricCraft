//! GPU brick-world rendering through FerroUI's Skia custom-effect API.
//!
//! Geometry lives in two immutable data textures. Frames change uniforms only;
//! editing the world or activating an assembly rebuilds the textures. An active
//! assembly has a separate local volume so driving never revoxelizes the world.

mod atlas;
mod camera;
mod effect;
mod renderer;

pub use atlas::{AtlasError, AtlasStats};
pub use camera::Camera;
pub use effect::{BrickWorldEffect, WORLD_SKSL};
pub use renderer::{PlacementPreview, RenderFrame, Renderer, VehicleRender};
