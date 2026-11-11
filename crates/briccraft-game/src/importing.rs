use crate::{GameAttribution, Session, StreamSession};
use briccraft_stream::{
    GenerationBudget, GlobalPlacement, GlobalPos, ImportJob, ImportProgress, StreamError,
    WindowOrigin,
};

pub(crate) struct PendingImport {
    pub job: ImportJob,
    pub label: String,
    pub attributions: Vec<GameAttribution>,
    pub center: GlobalPos,
    pub origin: WindowOrigin,
    pub previous_info: crate::WorldInfo,
}

impl Session {
    pub fn import_progress(&self) -> Option<ImportProgress> {
        self.streaming
            .as_ref()?
            .pending_import
            .as_ref()
            .map(|pending| pending.job.progress())
    }
    pub fn take_import_result(&mut self) -> Option<Result<String, String>> {
        self.streaming.as_mut()?.import_result.take()
    }

    pub fn begin_import(
        &mut self,
        placements: Vec<GlobalPlacement>,
        label: String,
        attributions: Vec<GameAttribution>,
    ) -> Result<(), StreamError> {
        if self.gameplay.is_expedition() {
            return Err(StreamError::InvalidSave("Switch to Creative before importing maps or models. Imported geography uses unlimited construction; expedition resources are not charged.".into()));
        }
        if self.active_vehicle.is_some() {
            return Err(StreamError::InvalidSave(
                "Park the active vehicle before importing a construction".into(),
            ));
        }
        if placements.is_empty() {
            return Err(StreamError::InvalidSave(
                "The import contains no parts".into(),
            ));
        }
        if label.len() > 1_024
            || attributions.len() > 64
            || attributions.iter().any(|a| {
                a.label.len() > 4_096
                    || a.url.len() > 8_192
                    || a.license.len() > 4_096
                    || (!a.url.is_empty()
                        && !a.url.starts_with("https://")
                        && !a.url.starts_with("http://"))
            })
        {
            return Err(StreamError::LimitExceeded("source metadata"));
        }
        let mut combined_sources = self.world_info.attributions.clone();
        for source in &attributions {
            if !combined_sources.contains(source) {
                combined_sources.push(source.clone());
            }
        }
        if combined_sources.len() > 64 {
            return Err(StreamError::LimitExceeded("source metadata"));
        }
        self.capture_streamed_edits()?;
        let state = self.streaming.as_mut().ok_or_else(|| {
            StreamError::InvalidSave(
                "Open a streamed building canvas before importing models or maps".into(),
            )
        })?;
        if state.pending_import.is_some() {
            return Err(StreamError::InvalidSave(
                "An import is already being prepared; cancel it before starting another".into(),
            ));
        }
        let mut min = [i64::MAX; 3];
        let mut max = [i64::MIN; 3];
        for placement in &placements {
            let size = placement.size()?;
            let position = [
                placement.position.x,
                i64::from(placement.position.y),
                placement.position.z,
            ];
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(
                    position[axis]
                        .checked_add(i64::from(size[axis]))
                        .ok_or(StreamError::CoordinateOverflow)?,
                );
            }
        }
        let center = GlobalPos::new(
            ((min[0] as i128 + max[0] as i128) / 2) as i64,
            ((min[1] + max[1]) / 2) as i32,
            ((min[2] as i128 + max[2] as i128) / 2) as i64,
        );
        let origin = WindowOrigin::around(center)?;
        let job = state.stream.begin_import(placements)?;
        let count = job.progress().total_parts;
        state.import_result = None;
        state.import_success = None;
        state.pending_import = Some(PendingImport {
            job,
            label,
            attributions,
            center,
            origin,
            previous_info: self.world_info.clone(),
        });
        self.status = format!("Preparing {count} imported parts · checking terrain and connections in bounded batches.");
        Ok(())
    }

    /// Cancel unpublished staging, or permit a host worker to stop. Once the transaction has
    /// committed, its complete window must finish publication before another command can replace it.
    pub fn cancel_import(&mut self) -> bool {
        if let Some(state) = &mut self.streaming {
            if state.import_focus.is_some() {
                self.status = "The import has already committed. Finishing its world view before editing can continue.".into();
                return false;
            }
            if let Some(pending) = state.pending_import.take() {
                self.world_info = pending.previous_info;
                state.import_result =
                    Some(Err("Import canceled. No imported parts were added.".into()));
                self.status = "Import canceled. No imported parts were added.".into();
            }
        }
        true
    }

    pub(crate) fn advance_import_state(
        &mut self,
        state: &mut StreamSession,
        chunk_budget: usize,
    ) -> Result<(), StreamError> {
        let Some(pending) = &mut state.pending_import else {
            return Ok(());
        };
        let progress = state
            .stream
            .advance_import(&mut pending.job, GenerationBudget::chunks(chunk_budget))?;
        if !progress.complete {
            self.status = format!(
                "Preparing {} · {}/{} chunks checked · {} parts.",
                pending.label,
                progress.validated_chunks,
                progress.total_chunks,
                progress.total_parts
            );
            return Ok(());
        }
        let pending = state.pending_import.take().expect("completed import");
        let count = pending.job.progress().total_parts;
        if let Err(error) = state.stream.commit_import(pending.job) {
            self.world_info = pending.previous_info;
            return Err(error);
        }
        self.world_info.name = pending.label;
        for source in pending.attributions {
            if !self.world_info.attributions.contains(&source) {
                self.world_info.attributions.push(source);
            }
        }
        self.last_blueprint.clear();
        state.last_blueprint_keys.clear();
        let origin = pending.origin;
        state.stream.request_window(origin)?;
        state.pending_origin = Some(origin);
        state.import_focus = Some(pending.center);
        let message = format!("Imported {count} editable parts. Explore to see the complete construction; Undo removes the import as one action.");
        state.import_success = Some(message.clone());
        self.status = message;
        Ok(())
    }
}
