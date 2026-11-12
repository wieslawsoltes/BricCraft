use crate::{
    CHUNK_CELLS, ChunkCoord, GlobalChange, GlobalPlacement, GlobalPos, MAX_SPARSE_EDITS,
    MAX_TRANSACTION_CHUNKS, MAX_TRANSACTION_PARTS, PartChange, PartKey, StreamError, StreamWorld,
    WorldSnapshot,
};
use briccraft_parts::{PALETTE, part};
use briccraft_world::{BuildError, World};
use std::collections::{BTreeMap, BTreeSet};

impl StreamWorld {
    /// Place a construction atomically, including across chunk/window boundaries.
    pub fn place_parts(
        &mut self,
        placements: &[GlobalPlacement],
    ) -> Result<GlobalChange, StreamError> {
        let change = self.new_parts_change(placements)?;
        self.apply_validated(&change)?;
        self.next_user_id = self
            .next_user_id
            .checked_add(placements.len() as u64)
            .ok_or(StreamError::LimitExceeded("part identity"))?;
        self.record_history(change.clone());
        Ok(change)
    }

    pub fn move_parts(
        &mut self,
        keys: &[PartKey],
        placements: &[GlobalPlacement],
    ) -> Result<GlobalChange, StreamError> {
        if keys.len() != placements.len() {
            return Err(StreamError::InvalidSave(
                "Move keys and placements must have equal lengths".into(),
            ));
        }
        let mut changes = Vec::with_capacity(keys.len());
        for (&key, &after) in keys.iter().zip(placements) {
            let before = self.resolve(key)?.ok_or(StreamError::MissingPart(key))?;
            changes.push(PartChange {
                key,
                before: Some(before),
                after: Some(after),
            });
        }
        let change = GlobalChange { changes };
        self.apply_validated(&change)?;
        self.record_history(change.clone());
        Ok(change)
    }

    pub fn remove_parts(&mut self, keys: &[PartKey]) -> Result<GlobalChange, StreamError> {
        let mut changes = Vec::with_capacity(keys.len());
        for &key in keys {
            let before = self.resolve(key)?.ok_or(StreamError::MissingPart(key))?;
            changes.push(PartChange {
                key,
                before: Some(before),
                after: None,
            });
        }
        let change = GlobalChange { changes };
        self.apply_validated(&change)?;
        self.record_history(change.clone());
        Ok(change)
    }

    /// Compare the current editable local World to its last captured state. Newly allocated local
    /// IDs receive global keys exactly once; removed IDs retain their key for identity-preserving
    /// local undo. A global transaction from elsewhere requires refreshing the snapshot first.
    pub fn capture_edits(
        &mut self,
        snapshot: &mut WorldSnapshot,
        current: &World,
    ) -> Result<GlobalChange, StreamError> {
        if snapshot.seed != self.seed
            || snapshot.preset != self.preset
            || snapshot.revision != self.revision
            || current.seed != self.seed
        {
            return Err(StreamError::StaleSnapshot);
        }
        let mut keys = snapshot.keys.clone();
        let mut next = self.next_user_id;
        let mut changes = Vec::new();
        for (&id, old) in &snapshot.baseline {
            if current.instance(id).is_none() {
                changes.push(PartChange {
                    key: keys[&id],
                    before: Some(GlobalPlacement::from_local(
                        snapshot.origin,
                        old.placement(),
                    )?),
                    after: None,
                });
            }
        }
        for instance in current.instances() {
            if snapshot.baseline.get(&instance.id) == Some(instance) {
                continue;
            }
            let key = if let Some(&key) = keys.get(&instance.id) {
                key
            } else {
                let key = PartKey::User(next);
                next = next
                    .checked_add(1)
                    .ok_or(StreamError::LimitExceeded("part identity"))?;
                keys.insert(instance.id, key);
                key
            };
            let before = snapshot
                .baseline
                .get(&instance.id)
                .map(|i| GlobalPlacement::from_local(snapshot.origin, i.placement()))
                .transpose()?;
            changes.push(PartChange {
                key,
                before,
                after: Some(GlobalPlacement::from_local(
                    snapshot.origin,
                    instance.placement(),
                )?),
            });
        }
        let change = GlobalChange { changes };
        self.apply_validated(&change)?;
        self.next_user_id = next;
        self.record_history(change.clone());
        snapshot.keys = keys;
        snapshot.baseline = current
            .instances()
            .map(|instance| (instance.id, instance.clone()))
            .collect();
        snapshot.world = current.clone();
        snapshot.revision = self.revision;
        Ok(change)
    }

    pub(crate) fn new_parts_change(
        &self,
        placements: &[GlobalPlacement],
    ) -> Result<GlobalChange, StreamError> {
        if placements.len() > MAX_TRANSACTION_PARTS {
            return Err(StreamError::LimitExceeded("transaction part"));
        }
        self.next_user_id
            .checked_add(placements.len() as u64)
            .ok_or(StreamError::LimitExceeded("part identity"))?;
        Ok(GlobalChange {
            changes: placements
                .iter()
                .enumerate()
                .map(|(i, &placement)| PartChange {
                    key: PartKey::User(self.next_user_id + i as u64),
                    before: None,
                    after: Some(placement),
                })
                .collect(),
        })
    }

    pub(crate) fn prepare_change(
        &self,
        change: &GlobalChange,
    ) -> Result<BTreeMap<ChunkCoord, Vec<usize>>, StreamError> {
        if change.changes.len() > MAX_TRANSACTION_PARTS {
            return Err(StreamError::LimitExceeded("transaction part"));
        }
        let mut keys = BTreeSet::new();
        let mut chunks: BTreeMap<ChunkCoord, Vec<usize>> = BTreeMap::new();
        for (i, entry) in change.changes.iter().enumerate() {
            if !keys.insert(entry.key) {
                return Err(StreamError::DuplicatePart(entry.key));
            }
            if self.resolve(entry.key)? != entry.before {
                return Err(StreamError::HistoryConflict);
            }
            if let Some(after) = entry.after {
                validate_placement(after)?;
                for chunk in placement_chunks(after)? {
                    chunks.entry(chunk).or_default().push(i);
                }
            }
        }
        if chunks.len() > MAX_TRANSACTION_CHUNKS {
            return Err(StreamError::LimitExceeded("transaction chunk"));
        }
        Ok(chunks)
    }

    pub(crate) fn validate_chunk_change(
        &self,
        chunk: ChunkCoord,
        change: &GlobalChange,
        indices: &[usize],
        changing: &BTreeSet<PartKey>,
    ) -> Result<(), StreamError> {
        let mut cells = vec![false; CHUNK_CELLS];
        let base = self.base_chunk(chunk)?;
        for generated in &base.parts {
            if changing.contains(&generated.key) || self.edits.contains_key(&generated.key) {
                continue;
            }
            for_each_cell(generated.placement, chunk, |index, _| {
                cells[index] = true;
                Ok(())
            })?;
        }
        if let Some(keys) = self.edit_chunks.get(&chunk) {
            for key in keys {
                if changing.contains(key) {
                    continue;
                }
                if let Some(Some(placement)) = self.edits.get(key) {
                    for_each_cell(*placement, chunk, |index, _| {
                        cells[index] = true;
                        Ok(())
                    })?;
                }
            }
        }
        for &index in indices {
            let placement = change.changes[index]
                .after
                .expect("only additions have chunk indices");
            for_each_cell(placement, chunk, |index, position| {
                if cells[index] {
                    return Err(StreamError::Occupied(position));
                }
                cells[index] = true;
                Ok(())
            })?;
        }
        Ok(())
    }

    pub(crate) fn apply_validated(&mut self, change: &GlobalChange) -> Result<(), StreamError> {
        if change.is_empty() {
            return Ok(());
        }
        let chunks = self.prepare_change(change)?;
        let changing = change.changes.iter().map(|c| c.key).collect();
        for (coord, indices) in chunks {
            self.validate_chunk_change(coord, change, &indices, &changing)?;
        }
        self.commit_change(change)
    }

    /// Called only after complete collision validation. Normalization is prepared before mutation.
    pub(crate) fn commit_change(&mut self, change: &GlobalChange) -> Result<(), StreamError> {
        if change.is_empty() {
            return Ok(());
        }
        let mut normalized = Vec::with_capacity(change.changes.len());
        let mut final_len = self.edits.len();
        for entry in &change.changes {
            let value = match entry.key {
                PartKey::Generated { .. } => {
                    let original = self
                        .generated_placement(entry.key)?
                        .ok_or(StreamError::MissingPart(entry.key))?;
                    if entry.after == Some(original) {
                        None
                    } else {
                        Some(entry.after)
                    }
                }
                PartKey::User(_) => entry.after.map(Some),
            };
            final_len = final_len - usize::from(self.edits.contains_key(&entry.key))
                + usize::from(value.is_some());
            normalized.push((entry.key, value));
        }
        if final_len > MAX_SPARSE_EDITS {
            return Err(StreamError::LimitExceeded("sparse edit"));
        }
        for (key, value) in normalized {
            if let Some(Some(old)) = self.edits.remove(&key) {
                for chunk in placement_chunks(old)? {
                    if let Some(keys) = self.edit_chunks.get_mut(&chunk) {
                        keys.remove(&key);
                        if keys.is_empty() {
                            self.edit_chunks.remove(&chunk);
                        }
                    }
                }
            }
            if let Some(value) = value {
                if let Some(placement) = value {
                    for chunk in placement_chunks(placement)? {
                        self.edit_chunks.entry(chunk).or_default().insert(key);
                    }
                }
                self.edits.insert(key, value);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}

pub(crate) fn validate_placement(placement: GlobalPlacement) -> Result<(), StreamError> {
    let definition = part(placement.part_id).ok_or(BuildError::UnknownPart(placement.part_id))?;
    if placement.color as usize >= PALETTE.len() {
        return Err(BuildError::InvalidColor(placement.color).into());
    }
    if placement.rotation > 3 {
        return Err(BuildError::InvalidRotation(placement.rotation).into());
    }
    let [sx, sy, sz] = definition.rotated_size(placement.rotation);
    if placement.position.y < 0
        || placement
            .position
            .y
            .checked_add(i32::from(sy))
            .is_none_or(|y| y > 72)
    {
        return Err(StreamError::InvalidSave(format!(
            "Part {} at plate Y={} exceeds the vertical range0..72",
            placement.part_id, placement.position.y
        )));
    }
    placement
        .position
        .x
        .checked_add(i64::from(sx))
        .ok_or(StreamError::CoordinateOverflow)?;
    placement
        .position
        .z
        .checked_add(i64::from(sz))
        .ok_or(StreamError::CoordinateOverflow)?;
    Ok(())
}

pub(crate) fn placement_chunks(placement: GlobalPlacement) -> Result<Vec<ChunkCoord>, StreamError> {
    let [sx, _, sz] = placement.size()?;
    let end_x = placement
        .position
        .x
        .checked_add(i64::from(sx) - 1)
        .ok_or(StreamError::CoordinateOverflow)?;
    let end_z = placement
        .position
        .z
        .checked_add(i64::from(sz) - 1)
        .ok_or(StreamError::CoordinateOverflow)?;
    let first = placement.position.chunk();
    let last = ChunkCoord::containing(end_x, end_z);
    let mut chunks = Vec::new();
    for z in first.z..=last.z {
        for x in first.x..=last.x {
            chunks.push(ChunkCoord::new(x, z));
        }
    }
    Ok(chunks)
}

pub(crate) fn for_each_cell(
    placement: GlobalPlacement,
    chunk: ChunkCoord,
    mut visit: impl FnMut(usize, GlobalPos) -> Result<(), StreamError>,
) -> Result<(), StreamError> {
    let origin = chunk.origin()?;
    let [sx, sy, sz] = placement.size()?;
    let start_x = placement.position.x.max(origin.x);
    let start_z = placement.position.z.max(origin.z);
    let end_x = placement
        .position
        .x
        .checked_add(i64::from(sx))
        .ok_or(StreamError::CoordinateOverflow)?
        .min(
            origin
                .x
                .checked_add(16)
                .ok_or(StreamError::CoordinateOverflow)?,
        );
    let end_z = placement
        .position
        .z
        .checked_add(i64::from(sz))
        .ok_or(StreamError::CoordinateOverflow)?
        .min(
            origin
                .z
                .checked_add(16)
                .ok_or(StreamError::CoordinateOverflow)?,
        );
    for y in placement.position.y..placement.position.y + i32::from(sy) {
        for z in start_z..end_z {
            for x in start_x..end_x {
                let index =
                    y as usize * 256 + (z - origin.z) as usize * 16 + (x - origin.x) as usize;
                visit(index, GlobalPos::new(x, y, z))?;
            }
        }
    }
    Ok(())
}
