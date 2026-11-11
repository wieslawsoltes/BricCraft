use crate::{BuildError, BuildOp, ChangeSet, Instance, World};
use std::collections::HashSet;

impl World {
    /// Validate every operation before changing any cell. A failed transaction leaves the world intact.
    pub fn apply(&mut self, operations: &[BuildOp]) -> Result<ChangeSet, BuildError> {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        let mut removing = HashSet::new();
        for operation in operations {
            if let BuildOp::Remove(id) = operation {
                if !removing.insert(*id) {
                    return Err(BuildError::DuplicateInstance(*id));
                }
                removed.push(
                    self.instance(*id)
                        .ok_or(BuildError::MissingInstance(*id))?
                        .clone(),
                );
            }
        }
        let mut reserved = HashSet::new();
        for operation in operations {
            if let BuildOp::Place(placement) = operation {
                for index in self.footprint(placement)? {
                    let occupied = self.cells[index];
                    if (occupied != 0 && !removing.contains(&(occupied - 1)))
                        || !reserved.insert(index)
                    {
                        return Err(BuildError::Occupied(self.coordinate(index)));
                    }
                }
                let id = self
                    .slots
                    .len()
                    .checked_add(added.len())
                    .and_then(|id| u32::try_from(id).ok())
                    .filter(|id| *id < u32::MAX)
                    .ok_or(BuildError::TooManyParts)?;
                added.push(Instance {
                    id,
                    part_id: placement.part_id,
                    color: placement.color,
                    position: placement.position,
                    rotation: placement.rotation,
                });
            }
        }
        self.commit(&removed, &added);
        Ok(ChangeSet { removed, added })
    }

    pub(crate) fn apply_exact(
        &mut self,
        removed: &[Instance],
        added: &[Instance],
    ) -> Result<(), BuildError> {
        let mut removing = HashSet::new();
        for instance in removed {
            if !removing.insert(instance.id) {
                return Err(BuildError::DuplicateInstance(instance.id));
            }
            if self.instance(instance.id) != Some(instance) {
                return Err(BuildError::HistoryConflict);
            }
        }
        let mut ids = HashSet::new();
        let mut reserved = HashSet::new();
        for instance in added {
            // Avoid allocating gigabytes for a malicious sparse identity in an imported construction.
            if instance.id as usize > self.slots.len().saturating_add(1_000_000)
                || instance.id == u32::MAX
            {
                return Err(BuildError::TooManyParts);
            }
            if !ids.insert(instance.id)
                || (self.instance(instance.id).is_some() && !removing.contains(&instance.id))
            {
                return Err(BuildError::DuplicateInstance(instance.id));
            }
            for index in self.footprint(&instance.placement())? {
                let occupied = self.cells[index];
                if (occupied != 0 && !removing.contains(&(occupied - 1))) || !reserved.insert(index)
                {
                    return Err(BuildError::Occupied(self.coordinate(index)));
                }
            }
        }
        self.commit(removed, added);
        Ok(())
    }

    fn commit(&mut self, removed: &[Instance], added: &[Instance]) {
        if removed.is_empty() && added.is_empty() {
            return;
        }
        for instance in removed {
            for index in self
                .footprint(&instance.placement())
                .expect("validated removal")
            {
                self.cells[index] = 0;
            }
            self.slots[instance.id as usize] = None;
            self.active -= 1;
        }
        for instance in added {
            for index in self
                .footprint(&instance.placement())
                .expect("validated addition")
            {
                self.cells[index] = instance.id + 1;
            }
            if instance.id as usize >= self.slots.len() {
                self.slots.resize(instance.id as usize + 1, None);
            }
            self.slots[instance.id as usize] = Some(instance.clone());
            self.active += 1;
        }
        self.revision = self.revision.wrapping_add(1);
    }
}
