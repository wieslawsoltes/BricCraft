use crate::{geometry, DesignError, MAX_NAME_CHARS};
use briccraft_world::Placement;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    ConnectedAssembly,
    /// An explicitly remembered construction group may include separate decorative pieces.
    ConstructionGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignMetadata {
    pub source_world_id: String,
    pub source_world_name: String,
    /// Exact source minimum in global cells; informational, never a placement offset.
    pub source_origin: [i64; 3],
    pub capture: CaptureKind,
}

impl DesignMetadata {
    pub(crate) fn validate(&self) -> Result<(), DesignError> {
        if self.source_world_id.len() > 256 || self.source_world_name.len() > 1_024 {
            return Err(DesignError::InvalidMetadata);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionDesign {
    pub id: u32,
    pub name: String,
    pub(crate) parts: Vec<Placement>,
    pub(crate) dimensions: [u8; 3],
    pub(crate) components: u16,
    pub(crate) metadata: DesignMetadata,
}

impl ConstructionDesign {
    pub fn part_count(&self) -> usize {
        self.parts.len()
    }
    pub const fn dimensions(&self) -> [u8; 3] {
        self.dimensions
    }
    pub const fn component_count(&self) -> u16 {
        self.components
    }
    pub fn metadata(&self) -> &DesignMetadata {
        &self.metadata
    }
    pub fn relative_parts(&self) -> &[Placement] {
        &self.parts
    }
    pub fn validate(&self) -> Result<(), DesignError> {
        if self.id == 0 || normalize_name(&self.name)? != self.name {
            return Err(DesignError::InvalidLibrary);
        }
        self.metadata.validate()?;
        let (dimensions, components) = geometry::validate(&self.parts)?;
        if self.dimensions != dimensions || self.components != components {
            return Err(DesignError::InvalidLibrary);
        }
        if self.metadata.capture == CaptureKind::ConnectedAssembly && components != 1 {
            return Err(DesignError::Disconnected);
        }
        Ok(())
    }
}

pub(crate) fn normalize_name(name: &str) -> Result<String, DesignError> {
    let name = name.trim();
    if name.is_empty()
        || name.len() > MAX_NAME_CHARS * 4
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err(DesignError::InvalidName);
    }
    Ok(name.to_owned())
}
