//! A platform-independent construction world with stable part identities and atomic edits.
#![forbid(unsafe_code)]

mod blueprint;
mod construct;
mod generation;
mod history;
mod picking;
mod railway;
mod save;
mod transaction;
mod types;
mod world;

pub use blueprint::Blueprint;
pub use history::History;
pub use picking::RayHit;
pub use railway::{
    COASTAL_RAIL_GAUGE, COASTAL_RAIL_HEIGHT, COASTAL_RAIL_LENGTH, COASTAL_RAIL_RADIUS,
    CoastalRailSample, coastal_rail_closest_distance, coastal_rail_sample,
};
pub use types::{BuildError, BuildOp, ChangeSet, Instance, Placement};
pub use world::World;

pub const WORLD_SIZE: [u16; 3] = [96, 72, 96];
pub const SEA_LEVEL_PLATES: i32 = 6;
pub const SEA_LEVEL: f32 = SEA_LEVEL_PLATES as f32 * 0.4;
pub const DEFAULT_SEED: u64 = 0xB71C_C4AF;
pub const SAVE_VERSION: u32 = 1;

#[cfg(test)]
mod tests;
