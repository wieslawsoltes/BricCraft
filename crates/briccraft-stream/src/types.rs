use crate::{ChunkCoord, GlobalPos, Rebase, WindowOrigin};
use briccraft_world::{BuildError, Instance, Placement, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomePreset {
    #[default]
    Meadow,
    Alpine,
    Desert,
    Coast,
    Flat,
}

impl BiomePreset {
    pub const ALL: [Self; 5] = [
        Self::Meadow,
        Self::Alpine,
        Self::Desert,
        Self::Coast,
        Self::Flat,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Meadow => "Wildflower meadows",
            Self::Alpine => "Alpine highlands",
            Self::Desert => "Golden dunes",
            Self::Coast => "Coral coast",
            Self::Flat => "Open building canvas",
        }
    }
}

/// Stable identity independent of render-window origins, cache eviction, and compact local IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PartKey {
    Generated { chunk: ChunkCoord, index: u16 },
    User(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPlacement {
    pub part_id: u16,
    pub color: u8,
    pub position: GlobalPos,
    pub rotation: u8,
}

impl GlobalPlacement {
    pub const fn new(part_id: u16, color: u8, position: GlobalPos, rotation: u8) -> Self {
        Self {
            part_id,
            color,
            position,
            rotation,
        }
    }
    pub fn from_local(origin: WindowOrigin, placement: Placement) -> Result<Self, StreamError> {
        Ok(Self::new(
            placement.part_id,
            placement.color,
            origin.global_cell(placement.position)?,
            placement.rotation,
        ))
    }
    pub fn size(self) -> Result<[u8; 3], StreamError> {
        Ok(briccraft_parts::part(self.part_id)
            .ok_or(BuildError::UnknownPart(self.part_id))?
            .rotated_size(self.rotation))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPart {
    pub key: PartKey,
    pub placement: GlobalPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartChange {
    pub key: PartKey,
    pub before: Option<GlobalPlacement>,
    pub after: Option<GlobalPlacement>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalChange {
    pub changes: Vec<PartChange>,
}

impl GlobalChange {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
    pub fn added_keys(&self) -> Vec<PartKey> {
        self.changes
            .iter()
            .filter(|c| c.before.is_none() && c.after.is_some())
            .map(|c| c.key)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainSample {
    pub height_plates: i32,
    pub color: u8,
    pub biome: BiomePreset,
}

#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub world: World,
    pub origin: WindowOrigin,
    /// Whole globally retained parts intersecting the outer boundary. They are intentionally not
    /// voxelized or destructively clipped; the 32-stud interaction guard keeps them out of reach.
    pub boundary_parts: Vec<GlobalPart>,
    pub(crate) seed: u64,
    pub(crate) preset: BiomePreset,
    pub(crate) revision: u64,
    pub(crate) keys: BTreeMap<u32, PartKey>,
    pub(crate) baseline: BTreeMap<u32, Instance>,
}

impl WorldSnapshot {
    pub fn part_key(&self, local_id: u32) -> Option<PartKey> {
        self.keys.get(&local_id).copied()
    }
    pub fn local_id(&self, key: PartKey) -> Option<u32> {
        self.keys.iter().find_map(|(&id, &value)| {
            (value == key && self.baseline.contains_key(&id)).then_some(id)
        })
    }
    pub fn global_position(&self, local: [f32; 3]) -> [f64; 3] {
        self.origin.global_position(local)
    }
    pub fn global_cell(&self, local: [i32; 3]) -> Result<GlobalPos, StreamError> {
        self.origin.global_cell(local)
    }
    pub fn local_position(&self, global: GlobalPos) -> Result<[f32; 3], StreamError> {
        self.origin.local_position(global)
    }
    pub fn rebase_around(&self, camera_local: [f32; 3]) -> Result<Option<Rebase>, StreamError> {
        self.origin.rebase_around(camera_local)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    Build(BuildError),
    CoordinateOverflow,
    InvalidPosition,
    UnalignedOrigin(WindowOrigin),
    InvalidCacheCapacity(usize),
    NotReady { remaining: usize },
    NoWindowRequested,
    MissingPart(PartKey),
    DuplicatePart(PartKey),
    Occupied(GlobalPos),
    StaleSnapshot,
    HistoryConflict,
    LimitExceeded(&'static str),
    InvalidSave(String),
}

impl From<BuildError> for StreamError {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}
impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(e) => e.fmt(f),
            Self::CoordinateOverflow => write!(
                f,
                "The global coordinate exceeds the signed 64-bit world range"
            ),
            Self::InvalidPosition => write!(
                f,
                "A position is non-finite or exceeds the supported local range"
            ),
            Self::UnalignedOrigin(o) => {
                write!(f, "Window origin {}, {} must align to 16 studs", o.x, o.z)
            }
            Self::InvalidCacheCapacity(c) => {
                write!(f, "Chunk cache capacity {c} must be between 36 and 256")
            }
            Self::NotReady { remaining } => {
                write!(f, "The next window still needs {remaining} chunks")
            }
            Self::NoWindowRequested => write!(f, "No streaming window has been requested"),
            Self::MissingPart(k) => write!(f, "Global part {k:?} no longer exists"),
            Self::DuplicatePart(k) => {
                write!(f, "Global part {k:?} occurs twice in one transaction")
            }
            Self::Occupied(p) => write!(
                f,
                "A part already occupies global cell {}, {}, {}",
                p.x, p.y, p.z
            ),
            Self::StaleSnapshot => write!(
                f,
                "Refresh this window before capturing further local edits"
            ),
            Self::HistoryConflict => {
                write!(f, "Global parts changed outside this construction history")
            }
            Self::LimitExceeded(name) => write!(f, "The bounded {name} limit was exceeded"),
            Self::InvalidSave(reason) => write!(f, "Invalid streamed world: {reason}"),
        }
    }
}
impl std::error::Error for StreamError {}
