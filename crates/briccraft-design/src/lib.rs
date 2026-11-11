//! Reusable, bounded construction templates independent of any UI or live-world identity.
//! A design stores relative cells, original part colors and quarter-turn orientations.
//! Validate deserialized libraries before exposing their data or replacing a live session.
#![forbid(unsafe_code)]

mod document;
mod error;
mod geometry;
mod library;
mod model;
mod transform;

pub use document::{DesignDocument, DESIGN_FORMAT, MAX_DESIGN_FILE_BYTES};
pub use error::DesignError;
pub use library::DesignLibrary;
pub use model::{CaptureKind, ConstructionDesign, DesignMetadata};

pub const MAX_DESIGNS: usize = 32;
pub const MAX_PARTS: usize = 256;
/// X/Z studs and Y plates. One plate occupies 0.4 world units.
pub const MAX_DIMENSIONS: [u8; 3] = [32, 72, 32];
pub const MAX_NAME_CHARS: usize = 64;
