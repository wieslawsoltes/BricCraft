use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignError {
    Empty,
    TooManyParts,
    TooLarge,
    InvalidName,
    DuplicateName,
    InvalidMetadata,
    UnknownPart(u16),
    InvalidColor(u8),
    InvalidRotation(u8),
    Overlap,
    Disconnected,
    InvalidLibrary,
    InvalidDocument,
    MissingDesign(u32),
    LibraryFull,
    CoordinateOverflow,
}

impl fmt::Display for DesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "A design must contain at least one part."),
            Self::TooManyParts => write!(
                f,
                "A design can contain at most {} parts.",
                crate::MAX_PARTS
            ),
            Self::TooLarge => write!(
                f,
                "Keep a design within 32 × 72 × 32 cells (X studs, Y plates, Z studs)."
            ),
            Self::InvalidName => write!(
                f,
                "Use a name with 1–64 characters and no control characters."
            ),
            Self::DuplicateName => write!(
                f,
                "A saved design already uses that name. Choose a different name."
            ),
            Self::InvalidMetadata => {
                write!(f, "The design source metadata is invalid or oversized.")
            }
            Self::UnknownPart(id) => write!(f, "Unknown design part {id}."),
            Self::InvalidColor(color) => write!(f, "Unknown design color {color}."),
            Self::InvalidRotation(rotation) => {
                write!(f, "Design rotation {rotation} must be in 0..4.")
            }
            Self::Overlap => write!(f, "Parts in a design cannot occupy the same cells."),
            Self::Disconnected => write!(
                f,
                "A connected-assembly capture must form one face-connected construction."
            ),
            Self::InvalidLibrary => write!(
                f,
                "The saved design library has an invalid version, identity, or geometry."
            ),
            Self::InvalidDocument => {
                write!(f, "This file is not a supported briccraft.design document.")
            }
            Self::MissingDesign(id) => write!(f, "Saved design {id} no longer exists."),
            Self::LibraryFull => write!(
                f,
                "This world already has {} saved designs. Delete one before capturing another.",
                crate::MAX_DESIGNS
            ),
            Self::CoordinateOverflow => write!(
                f,
                "The design destination exceeds the supported coordinate range."
            ),
        }
    }
}
impl std::error::Error for DesignError {}
