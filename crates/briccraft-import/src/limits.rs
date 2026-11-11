use crate::{ImportError, ImportResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportLimits {
    pub max_input_bytes: usize,
    pub max_features: usize,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_voxels: usize,
    pub max_parts: usize,
    /// Includes candidate raster cells and triangle/voxel intersection tests.
    pub max_work: usize,
}
impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 32 * 1024 * 1024,
            max_features: 20_000,
            max_vertices: 250_000,
            max_triangles: 250_000,
            max_voxels: 1_000_000,
            max_parts: 100_000,
            max_work: 20_000_000,
        }
    }
}
impl ImportLimits {
    pub(crate) fn input(&self, bytes: usize) -> ImportResult<()> {
        limit("input bytes", bytes, self.max_input_bytes)
    }
}
pub(crate) fn limit(resource: &'static str, actual: usize, maximum: usize) -> ImportResult<()> {
    if actual > maximum {
        Err(ImportError::BudgetExceeded {
            resource,
            limit: maximum,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStage {
    Parse,
    Project,
    Voxelize,
    Pack,
    Complete,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ImportProgress {
    pub stage: ImportStage,
    pub completed: usize,
    pub total: usize,
}
pub(crate) fn progress(
    callback: &mut dyn FnMut(ImportProgress) -> bool,
    stage: ImportStage,
    completed: usize,
    total: usize,
) -> ImportResult<()> {
    if callback(ImportProgress {
        stage,
        completed,
        total,
    }) {
        Ok(())
    } else {
        Err(ImportError::Cancelled)
    }
}
