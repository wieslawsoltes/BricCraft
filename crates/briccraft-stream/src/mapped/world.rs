use super::{
    MAX_MAP_REGIONS, MAX_MAP_SOURCE_PARTS, MapAnchor, MapRegion, model, source::MapSource,
};
use crate::{
    BiomePreset, CHUNK_CELLS, GlobalPlacement, PartKey, StreamError, StreamWorld, WindowOrigin,
};
use std::collections::BTreeSet;

impl StreamWorld {
    pub fn new_mapped(seed: u64, anchor: MapAnchor, capacity: usize) -> Result<Self, StreamError> {
        let mut world = Self::new(seed, BiomePreset::Flat, capacity)?;
        world.mapped = Some(MapSource::new(anchor)?);
        Ok(world)
    }

    pub fn map_anchor(&self) -> Option<&MapAnchor> {
        self.mapped.as_ref().map(|source| &source.anchor)
    }

    pub fn mapped_regions(&self) -> Vec<[i64; 4]> {
        self.mapped
            .as_ref()
            .map(|s| s.regions.values().map(|r| r.bounds).collect())
            .unwrap_or_default()
    }

    pub fn mapped_source_parts(&self) -> usize {
        self.mapped.as_ref().map_or(0, |s| s.part_count)
    }

    pub fn map_region_loaded(&self, bounds: [i64; 4]) -> bool {
        self.mapped
            .as_ref()
            .and_then(|s| s.regions.get(&(bounds[0], bounds[1])))
            .is_some_and(|r| r.bounds == bounds)
    }

    /// Insert a fully validated source tile without charging resources, allocating
    /// user identities, rewriting existing parts or adding construction history.
    /// On failure neither source data nor the render cache changes.
    pub fn publish_map_region(&mut self, region: MapRegion) -> Result<bool, StreamError> {
        let source = self
            .mapped
            .as_ref()
            .ok_or_else(|| StreamError::InvalidSave("This world has no mapped source".into()))?;
        let (region, chunks) = source.prepare(region)?;
        let key = (region.bounds[0], region.bounds[1]);
        if let Some(existing) = source.regions.get(&key) {
            if existing.as_ref() != &region {
                return Err(StreamError::InvalidSave("An already loaded map region changed; existing geography and edits are retained".into()));
            }
            return Ok(false);
        }
        // A retained edit must never acquire newly generated geometry underneath
        // it. Normally the edited tile is pinned; this also rejects forged saves.
        for (coord, chunk) in &chunks {
            // prepare() already proved source/source separation. Only occupied
            // player edits need a second source occupancy pass.
            let Some(keys) = self.edit_chunks.get(coord) else {
                continue;
            };
            let mut cells = vec![false; CHUNK_CELLS];
            for part in &chunk.parts {
                crate::edits::for_each_cell(part.placement, *coord, |index, _| {
                    cells[index] = true;
                    Ok(())
                })?;
            }
            for key in keys {
                if let Some(Some(part)) = self.edits.get(key) {
                    crate::edits::for_each_cell(*part, *coord, |index, position| {
                        if cells[index] {
                            return Err(StreamError::Occupied(position));
                        }
                        cells[index] = true;
                        Ok(())
                    })?;
                }
            }
        }
        let mut count = source.regions.len() + 1;
        let mut parts = source.part_count + region.placements.len();
        let mut candidates: Vec<_> = source
            .regions
            .iter()
            .filter(|(_, r)| !self.map_region_pinned(r.bounds))
            .map(|(&key, region)| {
                let x = key.0 as i128 - self.origin.x as i128;
                let z = key.1 as i128 - self.origin.z as i128;
                (x * x + z * z, key, region.placements.len())
            })
            .collect();
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        let mut evictions = Vec::new();
        for (_, key, length) in candidates {
            if count <= MAX_MAP_REGIONS && parts <= MAX_MAP_SOURCE_PARTS {
                break;
            }
            evictions.push(key);
            count -= 1;
            parts -= length;
        }
        if count > MAX_MAP_REGIONS || parts > MAX_MAP_SOURCE_PARTS {
            return Err(StreamError::LimitExceeded(
                "mapped source cache with protected constructions",
            ));
        }
        let source = self.mapped.as_mut().expect("validated mapped world");
        for key in evictions {
            let bounds = source.remove(key).expect("planned source eviction");
            self.chunks
                .retain(|coord, _| !model::contains(bounds, coord.x * 16, coord.z * 16));
        }
        source.insert(region, chunks);
        // Existing immutable source and snapshots are unchanged. No revision or
        // undo conflict is introduced merely because an adjacent tile arrived.
        Ok(true)
    }

    fn map_region_pinned(&self, bounds: [i64; 4]) -> bool {
        let window = |origin: WindowOrigin| {
            [
                origin.x,
                origin.z,
                origin.x.saturating_add(96),
                origin.z.saturating_add(96),
            ]
        };
        if model::intersects(bounds, window(self.origin))
            || self
                .requested
                .is_some_and(|o| model::intersects(bounds, window(o)))
        {
            return true;
        }
        let placement_touches = |p: GlobalPlacement| {
            let Ok([x, _, z]) = p.size() else {
                return true;
            };
            model::intersects(
                bounds,
                [
                    p.position.x,
                    p.position.z,
                    p.position.x.saturating_add(x as i64),
                    p.position.z.saturating_add(z as i64),
                ],
            )
        };
        let key_owned = |key: PartKey| matches!(key, PartKey::Generated { chunk, .. } if model::contains(bounds, chunk.x.saturating_mul(16), chunk.z.saturating_mul(16)));
        self.edits
            .iter()
            .any(|(&key, value)| key_owned(key) || value.is_some_and(placement_touches))
            || self.undo.iter().chain(&self.redo).any(|change| {
                change.changes.iter().any(|entry| {
                    key_owned(entry.key)
                        || entry.before.is_some_and(placement_touches)
                        || entry.after.is_some_and(placement_touches)
                })
            })
    }

    /// Missing source tiles intersecting the current/requested render window,
    /// nearest first. The requested 96-stud window moves when the player leaves
    /// its central guard; no speculative route or neighboring tile prefetch occurs.
    pub fn map_demands(&self, focus: [f64; 3]) -> Result<Vec<[i64; 4]>, StreamError> {
        let Some(source) = &self.mapped else {
            return Ok(Vec::new());
        };
        if focus.iter().any(|v| !v.is_finite())
            || focus[0].abs() > 1.0e12
            || focus[2].abs() > 1.0e12
        {
            return Err(StreamError::InvalidPosition);
        }
        let x = focus[0].floor() as i64;
        let z = focus[2].floor() as i64;
        let size = i64::from(source.anchor.region_size);
        let mut missing = BTreeSet::new();
        for window in std::iter::once(self.origin).chain(self.requested) {
            window.validate()?;
            // A resident view spans at most two source regions on either axis.
            // Enumerate those rectangles directly instead of allocating and
            // center-sorting all 36 resident chunks for a read-only demand query.
            let first = source.anchor.region_at(window.x, window.z)?;
            let last = source.anchor.region_at(window.x + 95, window.z + 95)?;
            for z in (first[1]..=last[1]).step_by(size as usize) {
                for x in (first[0]..=last[0]).step_by(size as usize) {
                    let bounds = source.anchor.region_at(x, z)?;
                    if !self.map_region_loaded(bounds) {
                        missing.insert(bounds);
                    }
                }
            }
        }
        let mut missing: Vec<_> = missing.into_iter().collect();
        missing.sort_unstable_by_key(|b| {
            let dx = b[0] as i128 + size as i128 / 2 - x as i128;
            let dz = b[1] as i128 + size as i128 / 2 - z as i128;
            dx * dx + dz * dz
        });
        missing.truncate(9);
        Ok(missing)
    }
}
