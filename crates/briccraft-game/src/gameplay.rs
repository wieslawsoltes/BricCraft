//! Session transactions keep geometry, the construction wallet and progression in order.
use crate::{BuildQuote, Session};
use briccraft_play::{parts_cost, Activity, ChangeValue, ResourcePermit, Reward};
use briccraft_stream::GlobalChange;
use briccraft_world::{BuildOp, ChangeSet};

impl Session {
    pub fn set_expedition(&mut self, enabled: bool) {
        if enabled == self.gameplay.is_expedition() {
            return;
        }
        if self.active_vehicle.is_some()
            || self.import_progress().is_some()
            || self
                .streaming
                .as_ref()
                .is_some_and(|s| s.import_focus.is_some())
        {
            self.status =
                "Park your vehicle and finish any import before changing play mode.".into();
            return;
        }
        if let Err(error) = self.capture_streamed_edits() {
            self.status = error.to_string();
            return;
        }
        self.gameplay.set_expedition(enabled);
        self.history.clear();
        if let Some(stream) = &mut self.streaming {
            stream.stream.clear_history();
        }
        self.status = if enabled {
            format!("Expedition started · {} studs. Recycle parts, build a base and bring a working vehicle to life. Your progress is saved with this world.",self.gameplay.studs())
        } else {
            "Creative mode · unlimited building. Expedition wallet and objectives are preserved; construction history starts fresh when modes change.".into()
        };
    }
    pub fn part_quote(&self, id: u16) -> Option<BuildQuote> {
        briccraft_play::part_cost(id).map(|cost| self.gameplay.quote(cost))
    }
    pub fn selected_part_quote(&self) -> BuildQuote {
        self.part_quote(self.selected_part).unwrap_or_default()
    }
    pub fn blueprint_quote(&self, name: &str) -> Option<BuildQuote> {
        let blueprint = crate::blueprints::named(name)?;
        let parts = blueprint.placements([0; 3], 0);
        parts_cost(parts.iter().map(|p| p.part_id))
            .ok()
            .map(|cost| self.gameplay.quote(cost))
    }

    /// Resource authorization is nonmutating. Both World validation and streamed
    /// capture must succeed before consuming the permit or advancing objectives.
    pub(crate) fn execute_construction(
        &mut self,
        operations: &[BuildOp],
    ) -> Result<(Vec<u32>, String), String> {
        let added = parts_cost(operations.iter().filter_map(|op| match op {
            BuildOp::Place(p) => Some(p.part_id),
            _ => None,
        }))
        .map_err(|e| e.to_string())?;
        let removed = parts_cost(operations.iter().filter_map(|op| match op {
            BuildOp::Remove(id) => self.world.instance(*id).map(|p| p.part_id),
            _ => None,
        }))
        .map_err(|e| e.to_string())?;
        let permit = self
            .gameplay
            .approve(ChangeValue { added, removed })
            .map_err(|e| e.to_string())?;
        let ids = self
            .history
            .execute(&mut self.world, operations)
            .map_err(|e| e.to_string())?;
        self.capture_streamed_edits()
            .map_err(|e| format!("Construction was not saved: {e}"))?;
        // No callback or gameplay mutation occurs between approval and commit.
        self.gameplay
            .commit(permit)
            .expect("synchronous approved construction wallet");
        let placed = operations
            .iter()
            .filter(|op| matches!(op, BuildOp::Place(_)))
            .count() as u64;
        let recycled = operations
            .iter()
            .filter(|op| matches!(op, BuildOp::Remove(_)))
            .count() as u64;
        let mut message = String::new();
        if placed > 0 {
            message.push_str(&reward_text(self.gameplay.record(Activity::Placed(placed))));
        }
        if recycled > 0 {
            message.push_str(&reward_text(
                self.gameplay.record(Activity::Recycled(recycled)),
            ));
        }
        Ok((ids, message))
    }
    pub(crate) fn approve_local_history(
        &self,
        redo: bool,
    ) -> Result<Option<ResourcePermit>, String> {
        let change = if redo {
            self.history.redo_change()
        } else {
            self.history.undo_change()
        };
        change
            .map(|c| {
                local_value(c).and_then(|v| {
                    self.gameplay
                        .approve(if redo { v } else { v.inverse() })
                        .map_err(|e| e.to_string())
                })
            })
            .transpose()
    }
    pub(crate) fn approve_global_history(
        &self,
        change: &GlobalChange,
        redo: bool,
    ) -> Result<ResourcePermit, String> {
        let value = ChangeValue {
            added: parts_cost(
                change
                    .changes
                    .iter()
                    .filter_map(|c| c.after.map(|p| p.part_id)),
            )
            .map_err(|e| e.to_string())?,
            removed: parts_cost(
                change
                    .changes
                    .iter()
                    .filter_map(|c| c.before.map(|p| p.part_id)),
            )
            .map_err(|e| e.to_string())?,
        };
        self.gameplay
            .approve(if redo { value } else { value.inverse() })
            .map_err(|e| e.to_string())
    }
}
fn local_value(change: &ChangeSet) -> Result<ChangeValue, String> {
    Ok(ChangeValue {
        added: parts_cost(change.added.iter().map(|p| p.part_id)).map_err(|e| e.to_string())?,
        removed: parts_cost(change.removed.iter().map(|p| p.part_id)).map_err(|e| e.to_string())?,
    })
}
pub(crate) fn reward_text(reward: Reward) -> String {
    if reward.objectives.is_empty() {
        String::new()
    } else {
        format!(
            " Objective complete: {} · +{} studs.",
            reward.objectives.join(" · "),
            reward.studs
        )
    }
}

#[cfg(test)]
mod tests;
