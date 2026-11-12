use crate::{
    BiomePreset, CHUNK_CELLS, ChunkCoord, GlobalChange, GlobalPlacement, GlobalPos,
    MAX_HISTORY_PARTS, PartKey, StreamError, TerrainSample, WINDOW_CHUNK_COUNT, WindowOrigin,
    generation,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

/// Whole chunks are indivisible units. A cell budget below `CHUNK_CELLS` makes no progress.
#[derive(Debug, Clone, Copy)]
pub struct GenerationBudget {
    pub max_chunks: usize,
    pub max_cells: usize,
}

impl GenerationBudget {
    pub const fn chunks(max_chunks: usize) -> Self {
        Self {
            max_chunks,
            max_cells: max_chunks.saturating_mul(CHUNK_CELLS),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GenerationProgress {
    pub generated: usize,
    pub generated_cells: usize,
    pub ready: usize,
    pub total: usize,
    pub pending: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamStats {
    pub cached_chunks: usize,
    pub cache_capacity: usize,
    pub cached_parts: usize,
    pub cached_cells: usize,
    pub generated_chunks: u64,
    pub cache_hits: u64,
    pub cache_evictions: u64,
    pub sparse_edits: usize,
    pub undo_steps: usize,
    pub redo_steps: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    pub chunk: Arc<generation::Chunk>,
    pub touched: u64,
}

#[derive(Debug, Clone)]
pub struct StreamWorld {
    pub(crate) seed: u64,
    pub(crate) preset: BiomePreset,
    pub(crate) origin: WindowOrigin,
    pub(crate) capacity: usize,
    pub(crate) chunks: BTreeMap<ChunkCoord, CacheEntry>,
    pub(crate) requested: Option<WindowOrigin>,
    pub(crate) required: Vec<ChunkCoord>,
    pub(crate) tick: u64,
    pub(crate) generated_chunks: u64,
    pub(crate) cache_hits: u64,
    pub(crate) cache_evictions: u64,
    pub(crate) edits: BTreeMap<PartKey, Option<GlobalPlacement>>,
    pub(crate) edit_chunks: BTreeMap<ChunkCoord, BTreeSet<PartKey>>,
    pub(crate) next_user_id: u64,
    pub(crate) revision: u64,
    pub(crate) undo: VecDeque<GlobalChange>,
    pub(crate) redo: Vec<GlobalChange>,
    pub(crate) history_parts: usize,
    pub(crate) mapped: Option<crate::mapped::MapSource>,
}

impl StreamWorld {
    pub fn new(seed: u64, preset: BiomePreset, cache_capacity: usize) -> Result<Self, StreamError> {
        if !(WINDOW_CHUNK_COUNT..=256).contains(&cache_capacity) {
            return Err(StreamError::InvalidCacheCapacity(cache_capacity));
        }
        Ok(Self {
            seed,
            preset,
            origin: WindowOrigin::default(),
            capacity: cache_capacity,
            chunks: BTreeMap::new(),
            requested: None,
            required: Vec::new(),
            tick: 0,
            generated_chunks: 0,
            cache_hits: 0,
            cache_evictions: 0,
            edits: BTreeMap::new(),
            edit_chunks: BTreeMap::new(),
            next_user_id: 1,
            revision: 0,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_parts: 0,
            mapped: None,
        })
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn preset(&self) -> BiomePreset {
        self.preset
    }
    pub fn origin(&self) -> WindowOrigin {
        self.origin
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn sparse_edit_count(&self) -> usize {
        self.edits.len()
    }

    pub fn request_window(
        &mut self,
        origin: WindowOrigin,
    ) -> Result<GenerationProgress, StreamError> {
        let required = origin.chunks()?;
        for coord in &required {
            if let Some(entry) = self.chunks.get_mut(coord) {
                self.tick = self.tick.wrapping_add(1);
                entry.touched = self.tick;
                self.cache_hits = self.cache_hits.wrapping_add(1);
            }
        }
        self.requested = Some(origin);
        self.required = required;
        Ok(self.progress())
    }

    pub fn requested_origin(&self) -> Option<WindowOrigin> {
        self.requested
    }

    pub fn progress(&self) -> GenerationProgress {
        let ready = self
            .required
            .iter()
            .filter(|coord| self.chunks.contains_key(coord))
            .count();
        GenerationProgress {
            ready,
            total: self.required.len(),
            pending: self.required.len() - ready,
            complete: self.requested.is_some() && ready == self.required.len(),
            ..GenerationProgress::default()
        }
    }

    pub fn advance_generation(
        &mut self,
        chunk_budget: usize,
    ) -> Result<GenerationProgress, StreamError> {
        self.advance_generation_with_budget(GenerationBudget::chunks(chunk_budget))
    }

    pub fn advance_generation_with_budget(
        &mut self,
        budget: GenerationBudget,
    ) -> Result<GenerationProgress, StreamError> {
        if self.requested.is_none() {
            return Err(StreamError::NoWindowRequested);
        }
        let count = budget.max_chunks.min(budget.max_cells / CHUNK_CELLS);
        let missing: Vec<_> = self
            .required
            .iter()
            .filter(|coord| !self.chunks.contains_key(coord))
            .filter(|coord| {
                self.mapped
                    .as_ref()
                    .is_none_or(|source| source.contains_chunk(coord))
            })
            .copied()
            .take(count)
            .collect();
        let protected: BTreeSet<_> = self.required.iter().copied().collect();
        for &coord in &missing {
            let chunk = self.base_chunk(coord)?;
            if self.chunks.len() >= self.capacity {
                let oldest = self
                    .chunks
                    .iter()
                    .filter(|(key, _)| !protected.contains(key))
                    .min_by_key(|(_, entry)| entry.touched)
                    .map(|(&key, _)| key)
                    .ok_or(StreamError::LimitExceeded("protected chunk cache"))?;
                self.chunks.remove(&oldest);
                self.cache_evictions = self.cache_evictions.wrapping_add(1);
            }
            self.tick = self.tick.wrapping_add(1);
            self.chunks.insert(
                coord,
                CacheEntry {
                    chunk,
                    touched: self.tick,
                },
            );
            self.generated_chunks = self.generated_chunks.wrapping_add(1);
        }
        let mut progress = self.progress();
        progress.generated = missing.len();
        progress.generated_cells = missing.len() * CHUNK_CELLS;
        Ok(progress)
    }

    pub fn stats(&self) -> StreamStats {
        StreamStats {
            cached_chunks: self.chunks.len(),
            cache_capacity: self.capacity,
            cached_parts: self
                .chunks
                .values()
                .map(|entry| entry.chunk.parts.len())
                .sum(),
            cached_cells: self
                .chunks
                .values()
                .map(|entry| entry.chunk.occupied_cells)
                .sum(),
            generated_chunks: self.generated_chunks,
            cache_hits: self.cache_hits,
            cache_evictions: self.cache_evictions,
            sparse_edits: self.edits.len(),
            undo_steps: self.undo.len(),
            redo_steps: self.redo.len(),
        }
    }

    pub fn terrain_at(&self, x: i64, z: i64) -> TerrainSample {
        if self.mapped.is_some() {
            let mut height_plates = 0;
            let mut color = 29;
            if let Ok(chunk) = self.base_chunk(ChunkCoord::containing(x, z)) {
                for part in &chunk.parts {
                    if let Ok([sx, sy, sz]) = part.placement.size() {
                        let p = part.placement.position;
                        if x >= p.x
                            && x < p.x + i64::from(sx)
                            && z >= p.z
                            && z < p.z + i64::from(sz)
                            && p.y + i32::from(sy) > height_plates
                        {
                            height_plates = p.y + i32::from(sy);
                            color = part.placement.color;
                        }
                    }
                }
            }
            return TerrainSample {
                height_plates,
                color,
                biome: self.preset,
            };
        }
        generation::terrain(self.seed, self.preset, x, z)
    }

    pub(crate) fn base_chunk(
        &self,
        coord: ChunkCoord,
    ) -> Result<Arc<generation::Chunk>, StreamError> {
        if let Some(entry) = self.chunks.get(&coord) {
            return Ok(Arc::clone(&entry.chunk));
        }
        if let Some(source) = &self.mapped {
            return source
                .chunk(coord)
                .ok_or(StreamError::NotReady { remaining: 1 });
        }
        // Transaction validation may touch a remote part. This temporary chunk does not displace a
        // protected render window and is dropped immediately after that chunk's collision pass.
        Ok(Arc::new(generation::generate(
            self.seed,
            self.preset,
            coord,
        )?))
    }

    pub(crate) fn generated_placement(
        &self,
        key: PartKey,
    ) -> Result<Option<GlobalPlacement>, StreamError> {
        let PartKey::Generated { chunk, index } = key else {
            return Ok(None);
        };
        let base = self.base_chunk(chunk)?;
        let part = if self.mapped.is_some() {
            base.parts.iter().find(|part| part.key == key)
        } else {
            base.parts.get(index as usize)
        };
        Ok(part.map(|part| part.placement))
    }

    pub fn resolve(&self, key: PartKey) -> Result<Option<GlobalPlacement>, StreamError> {
        match self.edits.get(&key) {
            Some(value) => Ok(*value),
            None => self.generated_placement(key),
        }
    }

    pub(crate) fn record_history(&mut self, change: GlobalChange) {
        if change.is_empty() {
            return;
        }
        self.redo.clear();
        self.history_parts += change.changes.len();
        self.undo.push_back(change);
        while self.undo.len() > 128 || self.history_parts > MAX_HISTORY_PARTS {
            if let Some(oldest) = self.undo.pop_front() {
                self.history_parts -= oldest.changes.len();
            }
        }
    }

    pub fn global_position(&self, local: [i32; 3]) -> Result<GlobalPos, StreamError> {
        self.origin.global_cell(local)
    }
}
