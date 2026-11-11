//! Platform-independent input, camera and construction sessions.
#![forbid(unsafe_code)]
mod blueprints;
pub mod camera;
mod construction;
mod importing;
pub mod input;
mod persistence;
pub mod session;
mod streaming;
pub mod vehicle;
mod world_info;
pub use briccraft_stream::{
    BiomePreset, GlobalPlacement, GlobalPos, ImportProgress, StreamError, StreamStats, WindowOrigin,
};
pub use camera::{CameraMode, CameraRig};
pub use input::Input;
pub use session::Session;
pub use streaming::StreamSession;
pub use world_info::{GameAttribution, WorldInfo};

#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod streaming_tests;
#[cfg(test)]
mod vehicle_tests;
