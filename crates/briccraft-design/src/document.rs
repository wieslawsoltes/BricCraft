use crate::{ConstructionDesign, DesignError};
use serde::{Deserialize, Serialize};

pub const DESIGN_FORMAT: &str = "briccraft.design";
pub const MAX_DESIGN_FILE_BYTES: usize = 256 * 1024;

/// Portable interchange envelope. A source library ID is never a destination identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignDocument {
    format: String,
    version: u8,
    design: ConstructionDesign,
}

impl DesignDocument {
    pub fn from_design(design: &ConstructionDesign) -> Result<Self, DesignError> {
        design.validate()?;
        Ok(Self {
            format: DESIGN_FORMAT.into(),
            version: 1,
            design: design.clone(),
        })
    }

    pub(crate) fn into_design(mut self) -> Result<ConstructionDesign, DesignError> {
        if self.format != DESIGN_FORMAT || self.version != 1 {
            return Err(DesignError::InvalidDocument);
        }
        // Validate geometry independently of the unrelated source world's local library ID.
        self.design.id = 1;
        self.design.validate()?;
        Ok(self.design)
    }
}
