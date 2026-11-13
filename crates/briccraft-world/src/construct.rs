use crate::{BuildError, Instance, World};
use std::sync::Arc;

impl World {
    /// Construct an unpublished world in one pass. The new occupancy grid itself detects overlaps;
    /// no second full-volume hash set is needed when streaming a complete, already compacted window.
    /// An error discards the candidate, leaving any caller's current World untouched.
    pub fn from_instances(seed: u64, instances: &[Instance]) -> Result<Self, BuildError> {
        let mut world = Self::empty(seed);
        for instance in instances {
            if instance.id > 1_000_000 {
                return Err(BuildError::TooManyParts);
            }
            if world.instance(instance.id).is_some() {
                return Err(BuildError::DuplicateInstance(instance.id));
            }
            let cells = world.footprint(&instance.placement())?;
            for &index in &cells {
                if world.cells[index] != 0 {
                    return Err(BuildError::Occupied(world.coordinate(index)));
                }
            }
            let occupancy = Arc::make_mut(&mut world.cells);
            for index in cells {
                occupancy[index] = instance.id + 1;
            }
            let slots = Arc::make_mut(&mut world.slots);
            if instance.id as usize >= slots.len() {
                slots.resize(instance.id as usize + 1, None);
            }
            slots[instance.id as usize] = Some(instance.clone());
            world.active += 1;
        }
        world.revision = u64::from(!instances.is_empty());
        Ok(world)
    }
}
