use briccraft_parts::part;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: u32,
    pub part_id: u16,
    pub color: u8,
    pub position: [i32; 3],
    pub rotation: u8,
}

impl Instance {
    pub fn size(&self) -> [u8; 3] {
        part(self.part_id).map_or([1, 1, 1], |p| p.rotated_size(self.rotation))
    }
    pub fn placement(&self) -> Placement {
        Placement {
            part_id: self.part_id,
            color: self.color,
            position: self.position,
            rotation: self.rotation,
        }
    }
    pub fn world_position(&self) -> [f32; 3] {
        [
            self.position[0] as f32,
            self.position[1] as f32 * 0.4,
            self.position[2] as f32,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub part_id: u16,
    pub color: u8,
    pub position: [i32; 3],
    pub rotation: u8,
}

impl Placement {
    pub const fn new(part_id: u16, color: u8, position: [i32; 3], rotation: u8) -> Self {
        Self {
            part_id,
            color,
            position,
            rotation,
        }
    }
    pub fn size(&self) -> [u8; 3] {
        part(self.part_id).map_or([1, 1, 1], |p| p.rotated_size(self.rotation))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    UnknownPart(u16),
    InvalidColor(u8),
    InvalidRotation(u8),
    OutOfBounds([i32; 3]),
    Occupied([i32; 3]),
    MissingInstance(u32),
    DuplicateInstance(u32),
    InvalidSave(String),
    HistoryConflict,
    TooManyParts,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPart(id) => write!(f, "Unknown part {id}"),
            Self::InvalidColor(id) => write!(f, "Unknown color {id}"),
            Self::InvalidRotation(r) => write!(f, "Rotation {r} must be in 0..4"),
            Self::OutOfBounds(p) => write!(f, "Outside the world at {p:?}"),
            Self::Occupied(p) => write!(f, "A part already occupies {p:?}"),
            Self::MissingInstance(id) => write!(f, "Part instance {id} no longer exists"),
            Self::DuplicateInstance(id) => write!(f, "Duplicate part instance {id}"),
            Self::InvalidSave(reason) => write!(f, "Invalid world: {reason}"),
            Self::HistoryConflict => {
                write!(f, "The world changed outside this construction history")
            }
            Self::TooManyParts => write!(f, "The part identity limit was reached"),
        }
    }
}
impl std::error::Error for BuildError {}

#[derive(Debug, Clone)]
pub enum BuildOp {
    Place(Placement),
    Remove(u32),
}

#[derive(Debug, Clone)]
pub struct ChangeSet {
    pub removed: Vec<Instance>,
    pub added: Vec<Instance>,
}
