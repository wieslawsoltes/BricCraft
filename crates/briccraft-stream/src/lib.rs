//! A bounded working set over a deterministic, horizontally open brick world.
//!
//! Global coordinates and construction identities never rebase. The renderer receives an ordinary
//! 96 × 72 × 96 [`briccraft_world::World`] and floating-point camera coordinates remain nearby.
//! Procedural saves contain sparse differences; mapped saves retain bounded source regions plus edits.
//! Generation and window publication are
//! separate: callers keep drawing the previous complete window while the next one is prepared.

mod bulk;
mod cache;
mod coords;
mod edits;
mod generation;
mod history;
mod mapped;
mod save;
mod snapshot;
mod types;

pub use bulk::{ImportJob, ImportProgress};
pub use cache::{GenerationBudget, GenerationProgress, StreamStats, StreamWorld};
pub use coords::{ChunkCoord, GlobalPos, Rebase, WindowOrigin};
pub use mapped::{MAX_MAP_REGIONS, MAX_MAP_SOURCE_PARTS, MapAnchor, MapRegion};
pub use types::{
    BiomePreset, GlobalChange, GlobalPart, GlobalPlacement, PartChange, PartKey, StreamError,
    TerrainSample, WorldSnapshot,
};

/// Horizontal chunk extent, in studs. Vertical extent remains the World's 72 plates.
pub const CHUNK_SIZE: i64 = 16;
pub const WINDOW_CHUNKS: usize = 6;
pub const WINDOW_CHUNK_COUNT: usize = WINDOW_CHUNKS * WINDOW_CHUNKS;
pub const CHUNK_CELLS: usize = 16 * 72 * 16;
pub const DEFAULT_CACHE_CAPACITY: usize = 64;
pub const GENERATOR_VERSION: u32 = 1;
pub const STREAM_SAVE_VERSION: u32 = 1;
pub const MAX_SPARSE_EDITS: usize = 250_000;
pub const MAX_TRANSACTION_PARTS: usize = 150_000;
pub const MAX_TRANSACTION_CHUNKS: usize = 4_096;
pub const MAX_HISTORY_PARTS: usize = 250_000;

#[cfg(test)]
mod capture_tests;
#[cfg(test)]
mod tests;
