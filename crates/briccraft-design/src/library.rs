use crate::{
    geometry, model::normalize_name, CaptureKind, ConstructionDesign, DesignDocument, DesignError,
    DesignMetadata, MAX_DESIGNS, MAX_NAME_CHARS,
};
use briccraft_world::Placement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignLibrary {
    version: u8,
    next_id: u32,
    designs: Vec<ConstructionDesign>,
}

impl Default for DesignLibrary {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            designs: Vec::new(),
        }
    }
}

impl DesignLibrary {
    pub fn designs(&self) -> &[ConstructionDesign] {
        &self.designs
    }
    pub fn get(&self, id: u32) -> Option<&ConstructionDesign> {
        self.designs.iter().find(|design| design.id == id)
    }
    pub fn len(&self) -> usize {
        self.designs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.designs.is_empty()
    }

    /// Metadata creation never edits world geometry or owns a resource wallet.
    pub fn capture(
        &mut self,
        name: &str,
        source: &[Placement],
        metadata: DesignMetadata,
    ) -> Result<u32, DesignError> {
        if self.designs.len() >= MAX_DESIGNS {
            return Err(DesignError::LibraryFull);
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or(DesignError::InvalidLibrary)?;
        let name = normalize_name(name)?;
        self.check_name(&name, None)?;
        metadata.validate()?;
        let parts = geometry::normalize(source)?;
        let (dimensions, components) = geometry::validate(&parts)?;
        if metadata.capture == CaptureKind::ConnectedAssembly && components != 1 {
            return Err(DesignError::Disconnected);
        }
        let design = ConstructionDesign {
            id: self.next_id,
            name,
            parts,
            dimensions,
            components,
            metadata,
        };
        design.validate()?;
        let id = design.id;
        self.designs.push(design);
        self.next_id = next;
        Ok(id)
    }

    pub fn export(&self, id: u32) -> Result<DesignDocument, DesignError> {
        let design = self.get(id).ok_or(DesignError::MissingDesign(id))?;
        DesignDocument::from_design(design)
    }

    /// Import creates a local identity and never edits geometry or a gameplay wallet.
    pub fn import(&mut self, document: DesignDocument) -> Result<u32, DesignError> {
        if self.designs.len() >= MAX_DESIGNS {
            return Err(DesignError::LibraryFull);
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or(DesignError::InvalidLibrary)?;
        let mut design = document.into_design()?;
        design.name = self.available_import_name(&design.name)?;
        design.id = self.next_id;
        design.validate()?;
        let id = design.id;
        self.designs.push(design);
        self.next_id = next;
        Ok(id)
    }

    pub fn rename(&mut self, id: u32, name: &str) -> Result<(), DesignError> {
        let name = normalize_name(name)?;
        self.check_name(&name, Some(id))?;
        let design = self
            .designs
            .iter_mut()
            .find(|design| design.id == id)
            .ok_or(DesignError::MissingDesign(id))?;
        design.name = name;
        Ok(())
    }

    pub fn delete(&mut self, id: u32) -> Result<(), DesignError> {
        let index = self
            .designs
            .iter()
            .position(|design| design.id == id)
            .ok_or(DesignError::MissingDesign(id))?;
        self.designs.remove(index);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        if self.version != 1 || self.next_id == 0 || self.designs.len() > MAX_DESIGNS {
            return Err(DesignError::InvalidLibrary);
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for design in &self.designs {
            if design.id >= self.next_id
                || !ids.insert(design.id)
                || !names.insert(design.name.to_lowercase())
            {
                return Err(DesignError::InvalidLibrary);
            }
            design.validate()?;
        }
        Ok(())
    }

    fn available_import_name(&self, name: &str) -> Result<String, DesignError> {
        if self.check_name(name, None).is_ok() {
            return Ok(name.to_owned());
        }
        for ordinal in 2..=MAX_DESIGNS + 1 {
            let suffix = format!(" ({ordinal})");
            let prefix: String = name
                .chars()
                .take(MAX_NAME_CHARS - suffix.chars().count())
                .collect();
            let candidate = format!("{}{suffix}", prefix.trim_end());
            if self.check_name(&candidate, None).is_ok() {
                return Ok(candidate);
            }
        }
        Err(DesignError::DuplicateName)
    }

    fn check_name(&self, name: &str, except: Option<u32>) -> Result<(), DesignError> {
        let key = name.to_lowercase();
        if self
            .designs
            .iter()
            .any(|d| Some(d.id) != except && d.name.to_lowercase() == key)
        {
            Err(DesignError::DuplicateName)
        } else {
            Ok(())
        }
    }
}
