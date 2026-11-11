use crate::{BuildError, BuildOp, ChangeSet, Instance, World};

/// History records whole user actions, including multi-part blueprints, as a single reversible edit.
#[derive(Debug)]
pub struct History {
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
    capacity: usize,
}
impl Default for History {
    fn default() -> Self {
        Self::new(256)
    }
}
impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            capacity: capacity.max(1),
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    /// Read-only transaction inspection lets callers authorize external state
    /// (for example a construction wallet) before applying an undo or redo.
    pub fn undo_change(&self) -> Option<&ChangeSet> {
        self.undo.last()
    }
    pub fn redo_change(&self) -> Option<&ChangeSet> {
        self.redo.last()
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
    pub fn execute(
        &mut self,
        world: &mut World,
        operations: &[BuildOp],
    ) -> Result<Vec<u32>, BuildError> {
        let change = world.apply(operations)?;
        let ids = change.added.iter().map(|instance| instance.id).collect();
        if !change.added.is_empty() || !change.removed.is_empty() {
            if self.undo.len() == self.capacity {
                self.undo.remove(0);
            }
            self.undo.push(change);
            self.redo.clear();
        }
        Ok(ids)
    }
    pub fn undo(&mut self, world: &mut World) -> Result<bool, BuildError> {
        let Some(change) = self.undo.last() else {
            return Ok(false);
        };
        world.apply_exact(&change.added, &change.removed)?;
        self.redo.push(self.undo.pop().expect("checked history"));
        Ok(true)
    }
    /// Atomically move/recolor/rotate existing identities. Parking and assembly transforms preserve
    /// references held by selection, mechanical connections, and later undo/redo operations.
    pub fn transform(
        &mut self,
        world: &mut World,
        after: &[Instance],
    ) -> Result<Vec<u32>, BuildError> {
        let before: Vec<_> = after
            .iter()
            .map(|instance| {
                world
                    .instance(instance.id)
                    .cloned()
                    .ok_or(BuildError::MissingInstance(instance.id))
            })
            .collect::<Result<_, _>>()?;
        world.apply_exact(&before, after)?;
        let ids = after.iter().map(|instance| instance.id).collect();
        if !after.is_empty() {
            if self.undo.len() == self.capacity {
                self.undo.remove(0);
            }
            self.undo.push(ChangeSet {
                removed: before,
                added: after.to_vec(),
            });
            self.redo.clear();
        }
        Ok(ids)
    }
    pub fn redo(&mut self, world: &mut World) -> Result<bool, BuildError> {
        let Some(change) = self.redo.last() else {
            return Ok(false);
        };
        world.apply_exact(&change.removed, &change.added)?;
        self.undo.push(self.redo.pop().expect("checked history"));
        Ok(true)
    }
}
