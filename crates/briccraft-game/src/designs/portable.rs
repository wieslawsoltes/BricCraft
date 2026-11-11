use crate::Session;
use briccraft_design::{DesignDocument, MAX_DESIGN_FILE_BYTES};

impl Session {
    /// Export one recipe; live geometry and the current resource wallet are not included.
    pub fn export_design(&self, id: u32) -> Result<String, String> {
        let document = self.design_library.export(id).map_err(|e| e.to_string())?;
        let source = serde_json::to_string(&document).map_err(|e| e.to_string())?;
        if source.len() > MAX_DESIGN_FILE_BYTES {
            return Err("The design document exceeds 256 KiB.".into());
        }
        Ok(source)
    }

    /// Imports only a validated recipe. Copies are purchased when they are actually placed.
    pub fn import_design(&mut self, source: &str) -> Result<u32, String> {
        self.design_edit_allowed()?;
        if source.len() > MAX_DESIGN_FILE_BYTES {
            return Err("The design document exceeds 256 KiB.".into());
        }
        let document: DesignDocument =
            serde_json::from_str(source).map_err(|e| format!("Invalid design document: {e}"))?;
        let id = self
            .design_library
            .import(document)
            .map_err(|e| e.to_string())?;
        let design = self.design_library.get(id).expect("imported design");
        self.status = format!("Imported {} · {} parts. The recipe is ready; placing a copy uses the current construction budget.", design.name, design.part_count());
        Ok(id)
    }
}
