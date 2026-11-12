//! Immutable geographic source regions and independently editable constructions.
//!
//! A source miss stays a miss. Network hosts decide how to fetch it; the stream
//! never substitutes procedural terrain for an unknown real-world location.
mod model;
mod source;
mod world;

pub use model::{MapAnchor, MapRegion};
pub(crate) use source::{MapSource, SavedMap};

/// Bounds source storage independently of the resident renderer's chunk cache.
pub const MAX_MAP_REGIONS: usize = 32;
pub const MAX_MAP_SOURCE_PARTS: usize = 250_000;

#[cfg(test)]
mod tests;
