use crate::{GlobalChange, PartChange, StreamError, StreamWorld};

impl StreamWorld {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    /// Inspect the next complete logical edit without mutating global state.
    pub fn undo_change(&self) -> Option<&GlobalChange> {
        self.undo.back()
    }
    pub fn redo_change(&self) -> Option<&GlobalChange> {
        self.redo.last()
    }

    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.history_parts = 0;
    }

    /// Undo uses global identities, so eviction and moving to a different render window are safe.
    pub fn undo(&mut self) -> Result<Option<GlobalChange>, StreamError> {
        let Some(original) = self.undo.back().cloned() else {
            return Ok(None);
        };
        let inverse = GlobalChange {
            changes: original
                .changes
                .iter()
                .map(|c| PartChange {
                    key: c.key,
                    before: c.after,
                    after: c.before,
                })
                .collect(),
        };
        self.apply_validated(&inverse)?;
        self.undo.pop_back();
        self.history_parts -= original.changes.len();
        self.redo.push(original);
        Ok(Some(inverse))
    }

    pub fn redo(&mut self) -> Result<Option<GlobalChange>, StreamError> {
        let Some(change) = self.redo.last().cloned() else {
            return Ok(None);
        };
        self.apply_validated(&change)?;
        self.redo.pop();
        self.history_parts += change.changes.len();
        self.undo.push_back(change.clone());
        Ok(Some(change))
    }
}
