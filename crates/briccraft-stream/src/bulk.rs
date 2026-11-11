use crate::{
    CHUNK_CELLS, ChunkCoord, GenerationBudget, GlobalChange, GlobalPlacement, PartKey, StreamError,
    StreamWorld,
};
use std::collections::BTreeSet;

/// A cancelable, unpublished transaction. Dropping it makes no world changes and consumes no IDs.
#[derive(Debug)]
pub struct ImportJob {
    pub(crate) change: GlobalChange,
    chunks: Vec<(ChunkCoord, Vec<usize>)>,
    changing: BTreeSet<PartKey>,
    next_chunk: usize,
    source_revision: u64,
    first_user_id: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportProgress {
    pub validated_chunks: usize,
    pub total_chunks: usize,
    pub total_parts: usize,
    pub processed_cells: usize,
    pub complete: bool,
}

impl ImportJob {
    pub fn progress(&self) -> ImportProgress {
        ImportProgress {
            validated_chunks: self.next_chunk,
            total_chunks: self.chunks.len(),
            total_parts: self.change.changes.len(),
            processed_cells: 0,
            complete: self.next_chunk == self.chunks.len(),
        }
    }
}

impl StreamWorld {
    /// Metadata and vertical bounds are checked before any generation or world mutation. Collision
    /// checks run incrementally in `advance_import`, and the complete transaction publishes once.
    pub fn begin_import(&self, placements: Vec<GlobalPlacement>) -> Result<ImportJob, StreamError> {
        let change = self.new_parts_change(&placements)?;
        let chunks = self.prepare_change(&change)?.into_iter().collect();
        let changing = change.changes.iter().map(|entry| entry.key).collect();
        Ok(ImportJob {
            change,
            chunks,
            changing,
            next_chunk: 0,
            source_revision: self.revision,
            first_user_id: self.next_user_id,
        })
    }

    pub fn advance_import(
        &self,
        job: &mut ImportJob,
        budget: GenerationBudget,
    ) -> Result<ImportProgress, StreamError> {
        if job.source_revision != self.revision || job.first_user_id != self.next_user_id {
            return Err(StreamError::StaleSnapshot);
        }
        let count = budget.max_chunks.min(budget.max_cells / CHUNK_CELLS);
        let start = job.next_chunk;
        while job.next_chunk < job.chunks.len() && job.next_chunk - start < count {
            let (coord, indices) = &job.chunks[job.next_chunk];
            self.validate_chunk_change(*coord, &job.change, indices, &job.changing)?;
            job.next_chunk += 1;
        }
        let mut progress = job.progress();
        progress.processed_cells = (job.next_chunk - start) * CHUNK_CELLS;
        Ok(progress)
    }

    pub fn commit_import(&mut self, job: ImportJob) -> Result<GlobalChange, StreamError> {
        if job.source_revision != self.revision || job.first_user_id != self.next_user_id {
            return Err(StreamError::StaleSnapshot);
        }
        if !job.progress().complete {
            return Err(StreamError::NotReady {
                remaining: job.chunks.len() - job.next_chunk,
            });
        }
        self.commit_change(&job.change)?;
        self.next_user_id = self
            .next_user_id
            .checked_add(job.change.changes.len() as u64)
            .ok_or(StreamError::LimitExceeded("part identity"))?;
        self.record_history(job.change.clone());
        Ok(job.change)
    }
}
