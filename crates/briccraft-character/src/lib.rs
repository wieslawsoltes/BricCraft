//! Kinematic walking through a bounded voxel volume.
//!
//! The collider is an upright cylinder with a circular footprint and flat feet/head.
//! Horizontal movement sweeps that circle against occupied cell rectangles, slides
//! along contact normals, and optionally steps over a low obstacle. Vertical sweeps
//! stop at floors and ceilings. This is an occupancy controller, not a rigid body
//! solver: holes and slopes inside an occupied voxel remain conservatively solid.
//!
//! Positions are at the centre of the feet, in world units. The caller owns position,
//! so rebasing a streamed world does not reset velocity or require hidden state edits.
#![forbid(unsafe_code)]

mod config;
mod controller;
mod grid;
mod recovery;
mod sweep;

pub use config::{CharacterConfig, ConfigError};
pub use controller::{CharacterController, CharacterInput, CharacterMotion};
pub use grid::CollisionGrid;

#[cfg(test)]
mod tests;
