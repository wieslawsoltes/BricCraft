use crate::{BuildError, BuildOp, ChangeSet, World};

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
    pub fn redo(&mut self, world: &mut World) -> Result<bool, BuildError> {
        let Some(change) = self.redo.last() else {
            return Ok(false);
        };
        world.apply_exact(&change.removed, &change.added)?;
        self.undo.push(self.redo.pop().expect("checked history"));
        Ok(true)
    }
}
