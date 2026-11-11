use crate::{BuildError, Instance, SAVE_VERSION, WORLD_SIZE, World};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedWorld {
    format: String,
    version: u32,
    seed: u64,
    dimensions: [u16; 3],
    parts: Vec<Instance>,
}

impl World {
    pub fn to_json(&self) -> Result<String, BuildError> {
        serde_json::to_string(&SavedWorld {
            format: "briccraft-world".into(),
            version: SAVE_VERSION,
            seed: self.seed,
            dimensions: self.size,
            parts: self.instances().cloned().collect(),
        })
        .map_err(|error| BuildError::InvalidSave(error.to_string()))
    }

    /// Import into a new world only after all dimensions, identifiers, parts, and occupied cells validate.
    pub fn from_json(source: &str) -> Result<Self, BuildError> {
        if source.len() > 32 * 1024 * 1024 {
            return Err(BuildError::InvalidSave("file exceeds 32 MiB".into()));
        }
        let saved: SavedWorld = serde_json::from_str(source)
            .map_err(|error| BuildError::InvalidSave(error.to_string()))?;
        if saved.format != "briccraft-world" || saved.version != SAVE_VERSION {
            return Err(BuildError::InvalidSave(
                "unsupported format or version".into(),
            ));
        }
        if saved.dimensions != WORLD_SIZE {
            return Err(BuildError::InvalidSave(
                "unsupported world dimensions".into(),
            ));
        }
        if saved.parts.len() > 250_000 {
            return Err(BuildError::InvalidSave("too many parts".into()));
        }
        let mut world = Self::empty(saved.seed);
        world.restore(&saved.parts)?;
        Ok(world)
    }
}
