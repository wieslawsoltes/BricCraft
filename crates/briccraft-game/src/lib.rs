//! Platform-independent input, camera and construction sessions.
#![forbid(unsafe_code)]
mod blueprints;
pub mod camera;
mod construction;
pub mod input;
mod persistence;
pub mod session;
pub mod vehicle;
pub use camera::{CameraMode, CameraRig};
pub use input::Input;
pub use session::Session;

#[cfg(test)]
mod vehicle_tests;
