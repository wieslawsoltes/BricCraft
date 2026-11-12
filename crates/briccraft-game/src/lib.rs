//! Platform-independent input, camera and construction sessions.
#![forbid(unsafe_code)]
mod blueprints;
pub mod camera;
mod construction;
mod designs;
mod gameplay;
mod importing;
pub mod input;
mod mapped;
mod persistence;
pub mod session;
mod streaming;
pub mod vehicle;
mod world_info;
pub use briccraft_design::{CaptureKind, ConstructionDesign, DesignMetadata};
pub use briccraft_play::{BuildQuote, GameMode, Gameplay, ObjectiveView};
pub use briccraft_stream::{
    BiomePreset, GlobalPlacement, GlobalPos, ImportProgress, MapAnchor, MapRegion, StreamError,
    StreamStats, WindowOrigin,
};
pub use camera::{CameraMode, CameraRig};
pub use designs::DesignCaptureStatus;
pub use input::Input;
pub use session::Session;
pub use streaming::StreamSession;
pub use vehicle::WorkshopStatus;
pub use world_info::{GameAttribution, WorldInfo};

#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod streaming_tests;
#[cfg(test)]
mod vehicle_tests;
