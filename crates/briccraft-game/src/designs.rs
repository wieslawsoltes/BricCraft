//! Saved construction recipes are separate from live part identities and construction history.
mod capture;
mod placement;
mod portable;
pub(crate) use capture::CaptureCache;

use crate::{BuildQuote, Session};
use briccraft_design::{ConstructionDesign, DesignMetadata, MAX_DESIGNS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignCaptureStatus {
    pub ready: bool,
    pub detail: String,
    pub part_count: usize,
}

impl Session {
    pub fn designs(&self) -> &[ConstructionDesign] {
        self.design_library.designs()
    }

    pub fn design_quote(&self, id: u32) -> Option<BuildQuote> {
        let design = self.design_library.get(id)?;
        briccraft_play::parts_cost(design.relative_parts().iter().map(|part| part.part_id))
            .ok()
            .map(|cost| self.gameplay.quote(cost))
    }

    /// This is cached by scene, world revision and selection; HUD polling does not reflood terrain.
    pub fn design_capture_status(&self) -> DesignCaptureStatus {
        let candidate = self.design_edit_allowed().and_then(|()| {
            if self.design_library.len() >= MAX_DESIGNS {
                return Err(format!(
                    "This world has {MAX_DESIGNS} saved designs. Delete one to make room."
                ));
            }
            self.prepare_design_capture()
        });
        match candidate {
            Ok(candidate) => DesignCaptureStatus {
                ready: true,
                detail: format!(
                    "Save {} parts from {}. Colors and orientations are retained.",
                    candidate.ids.len(),
                    candidate.label
                ),
                part_count: candidate.ids.len(),
            },
            Err(detail) => DesignCaptureStatus {
                ready: false,
                detail,
                part_count: 0,
            },
        }
    }

    pub fn capture_design(&mut self, name: &str) -> Result<u32, String> {
        self.design_edit_allowed()?;
        let candidate = self.prepare_design_capture()?;
        let parts: Vec<_> = candidate
            .ids
            .iter()
            .map(|id| {
                self.world
                    .instance(*id)
                    .map(|part| part.placement())
                    .ok_or_else(|| "The selected construction changed. Select it again.".to_owned())
            })
            .collect::<Result<_, _>>()?;
        let min: [i32; 3] = std::array::from_fn(|axis| {
            parts
                .iter()
                .map(|part| part.position[axis])
                .min()
                .expect("nonempty candidate")
        });
        let source_origin = self
            .world_origin()
            .global_cell(min)
            .map_err(|e| e.to_string())?;
        let metadata = DesignMetadata {
            source_world_id: self.world_info.id.clone(),
            source_world_name: self.world_info.name.clone(),
            source_origin: [source_origin.x, i64::from(source_origin.y), source_origin.z],
            capture: candidate.kind,
        };
        let id = self
            .design_library
            .capture(name, &parts, metadata)
            .map_err(|e| e.to_string())?;
        let design = self.design_library.get(id).expect("captured design");
        self.status = format!("Saved {} · {} parts. The recipe is stored with this world; copies use the current play-mode budget.", design.name, design.part_count());
        Ok(id)
    }

    pub fn rename_design(&mut self, id: u32, name: &str) -> Result<(), String> {
        self.design_edit_allowed()?;
        self.design_library
            .rename(id, name)
            .map_err(|e| e.to_string())?;
        self.status = format!(
            "Design renamed to {}.",
            self.design_library.get(id).expect("renamed design").name
        );
        Ok(())
    }

    pub fn delete_design(&mut self, id: u32) -> Result<(), String> {
        self.design_edit_allowed()?;
        let name = self
            .design_library
            .get(id)
            .ok_or_else(|| format!("Saved design {id} no longer exists."))?
            .name
            .clone();
        self.design_library.delete(id).map_err(|e| e.to_string())?;
        self.status =
            format!("Deleted the {name} recipe. Placed constructions remain in the world.");
        Ok(())
    }

    fn design_edit_allowed(&self) -> Result<(), String> {
        if self.active_vehicle.is_some() {
            return Err("Park your vehicle before capturing or placing designs.".into());
        }
        if self.import_progress().is_some()
            || self
                .streaming
                .as_ref()
                .is_some_and(|state| state.import_focus.is_some())
        {
            return Err(
                "Finish or cancel the current import before changing saved designs.".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
