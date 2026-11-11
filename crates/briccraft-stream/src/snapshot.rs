use crate::{GlobalPart, PartKey, StreamError, StreamWorld, WindowOrigin, WorldSnapshot};
use briccraft_world::{Instance, World};
use std::collections::{BTreeMap, BTreeSet};

impl StreamWorld {
    /// Publish only a complete window. This never generates chunks or exposes a partial terrain slab.
    pub fn snapshot(&mut self, origin: WindowOrigin) -> Result<WorldSnapshot, StreamError> {
        let required = origin.chunks()?;
        let remaining = required
            .iter()
            .filter(|coord| !self.chunks.contains_key(coord))
            .count();
        if remaining != 0 {
            return Err(StreamError::NotReady { remaining });
        }
        let mut parts = BTreeMap::new();
        let mut edited_keys = BTreeSet::new();
        for coord in &required {
            let entry = self.chunks.get_mut(coord).expect("complete snapshot cache");
            self.tick = self.tick.wrapping_add(1);
            entry.touched = self.tick;
            for part in &entry.chunk.parts {
                if !self.edits.contains_key(&part.key) {
                    parts.insert(part.key, part.placement);
                }
            }
            if let Some(keys) = self.edit_chunks.get(coord) {
                edited_keys.extend(keys);
            }
        }
        for key in edited_keys {
            if let Some(Some(placement)) = self.edits.get(key) {
                parts.insert(*key, *placement);
            }
        }
        let mut instances = Vec::with_capacity(parts.len());
        let mut keys = BTreeMap::new();
        let mut boundary_parts = Vec::new();
        for (key, placement) in parts {
            let size = placement.size()?;
            // Both values are already near the window due to its chunk index, but checked subtraction
            // also makes malformed saves at the signed-coordinate endpoints fail explicitly.
            let x = placement
                .position
                .x
                .checked_sub(origin.x)
                .ok_or(StreamError::CoordinateOverflow)?;
            let z = placement
                .position
                .z
                .checked_sub(origin.z)
                .ok_or(StreamError::CoordinateOverflow)?;
            if x >= 96 || z >= 96 || x + i64::from(size[0]) <= 0 || z + i64::from(size[2]) <= 0 {
                continue;
            }
            if x < 0 || z < 0 || x + i64::from(size[0]) > 96 || z + i64::from(size[2]) > 96 {
                boundary_parts.push(GlobalPart { key, placement });
                continue;
            }
            let id = instances.len() as u32;
            keys.insert(id, key);
            instances.push(Instance {
                id,
                part_id: placement.part_id,
                color: placement.color,
                position: [x as i32, placement.position.y, z as i32],
                rotation: placement.rotation,
            });
        }
        let world = World::from_instances(self.seed, &instances)?;
        let baseline = instances
            .into_iter()
            .map(|instance| (instance.id, instance))
            .collect();
        self.origin = origin;
        Ok(WorldSnapshot {
            world,
            origin,
            boundary_parts,
            seed: self.seed,
            preset: self.preset,
            revision: self.revision,
            keys,
            baseline,
        })
    }

    /// Synchronous setup helper; animation loops should request, budget generation, then snapshot.
    pub fn prepare_window(&mut self, origin: WindowOrigin) -> Result<WorldSnapshot, StreamError> {
        self.request_window(origin)?;
        self.advance_generation(crate::WINDOW_CHUNK_COUNT)?;
        self.snapshot(origin)
    }
}

impl WorldSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn visible_keys(&self) -> impl Iterator<Item = (u32, PartKey)> + '_ {
        self.keys
            .iter()
            .filter(|(id, _)| self.baseline.contains_key(id))
            .map(|(&id, &key)| (id, key))
    }
}
