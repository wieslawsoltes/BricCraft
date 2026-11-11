//! Explicit limits and counters for the bounded GPU working set.

pub const ATLAS_WIDTH: usize = 1024;
pub const MAX_ATLAS_HEIGHT: usize = 4096;
pub const MAX_ATLAS_TEXELS: usize = ATLAS_WIDTH * MAX_ATLAS_HEIGHT;
/// Four texels describe a part; record zero is reserved for empty cells.
pub const MAX_PACKED_PARTS: usize = MAX_ATLAS_TEXELS / 4 - 1;
/// Matches the simulation's limit for one independently moving construction.
pub const MAX_VEHICLE_PARTS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasStats {
    pub instances: usize,
    pub static_instances: usize,
    pub vehicle_instances: usize,
    pub occupied_cells: usize,
    /// Placement cells in the current window, independent of the global world.
    pub active_cells: usize,
    /// Additional cells reserved for the detached moving body.
    pub vehicle_cells: usize,
    pub coarse_tiles: usize,
    pub occupied_coarse_tiles: usize,
    /// Bytes in both current RGBA8 atlas images, including row padding.
    pub texture_bytes: usize,
    /// CPU storage for the compact source-to-GPU ID lookup.
    pub lookup_bytes: usize,
    /// Owned pixel-vector capacities plus lookup storage. Excludes World,
    /// temporary build inputs, and FerroUI/driver copies of the images.
    pub staging_bytes: usize,
    pub part_budget: usize,
    pub cell_texel_budget: usize,
    pub texture_budget_bytes: usize,
    /// New immutable atlas images handed to FerroUI. GPU transfer is deferred
    /// until the image is drawn; this is not a driver timing measurement.
    pub uploads: u64,
    pub cell_uploads: u64,
    pub part_uploads: u64,
    pub uploaded_bytes: u64,
    /// Atlas bytes published by the most recent call to `Renderer::effect`.
    pub last_uploaded_bytes: usize,
    pub rebuilds: u64,
    /// Effect snapshots prepared, not presented frames or a frame-rate estimate.
    pub effect_frames: u64,
    pub uniform_only_frames: u64,
}
