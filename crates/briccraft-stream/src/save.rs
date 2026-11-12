use crate::{
    BiomePreset, ChunkCoord, DEFAULT_CACHE_CAPACITY, GENERATOR_VERSION, GlobalChange,
    GlobalPlacement, MAX_SPARSE_EDITS, PartChange, PartKey, STREAM_SAVE_VERSION, StreamError,
    StreamWorld, WindowOrigin,
    edits::{placement_chunks, validate_placement},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamSave {
    version: u32,
    generator_version: u32,
    seed: u64,
    preset: BiomePreset,
    origin: WindowOrigin,
    next_user_id: u64,
    edits: Vec<SavedEdit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mapped: Option<crate::mapped::SavedMap>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedEdit {
    key: PartKey,
    value: Option<GlobalPlacement>,
}

impl StreamWorld {
    /// Deterministic procedural terrain and the bounded cache are excluded. Global construction
    /// history survives rebases in memory; a loaded save starts a fresh undo session.
    pub fn to_json(&self) -> Result<String, StreamError> {
        let save = StreamSave {
            version: STREAM_SAVE_VERSION,
            generator_version: GENERATOR_VERSION,
            seed: self.seed,
            preset: self.preset,
            origin: self.origin,
            next_user_id: self.next_user_id,
            edits: self
                .edits
                .iter()
                .map(|(&key, &value)| SavedEdit { key, value })
                .collect(),
            mapped: self.mapped.as_ref().map(crate::mapped::MapSource::saved),
        };
        serde_json::to_string(&save).map_err(|error| StreamError::InvalidSave(error.to_string()))
    }

    pub fn from_json(source: &str) -> Result<Self, StreamError> {
        Self::from_json_with_capacity(source, DEFAULT_CACHE_CAPACITY)
    }

    pub fn from_json_with_capacity(source: &str, capacity: usize) -> Result<Self, StreamError> {
        if source.len() > 64 * 1024 * 1024 {
            return Err(StreamError::LimitExceeded("save byte"));
        }
        let save: StreamSave = serde_json::from_str(source)
            .map_err(|error| StreamError::InvalidSave(error.to_string()))?;
        if save.version != STREAM_SAVE_VERSION || save.generator_version != GENERATOR_VERSION {
            return Err(StreamError::InvalidSave(
                "Unsupported save or procedural generator version".into(),
            ));
        }
        save.origin.validate()?;
        if save.edits.len() > MAX_SPARSE_EDITS {
            return Err(StreamError::LimitExceeded("sparse edit"));
        }
        if save.next_user_id == 0 {
            return Err(StreamError::InvalidSave(
                "User identities start at one".into(),
            ));
        }
        let mut stream = Self::new(save.seed, save.preset, capacity)?;
        if let Some(mapped) = save.mapped {
            if save.preset != BiomePreset::Flat {
                return Err(StreamError::InvalidSave(
                    "Mapped sources use the empty external base, not a procedural preset".into(),
                ));
            }
            stream.mapped = Some(crate::mapped::MapSource::restore(mapped)?);
        }
        stream.origin = save.origin;
        stream.next_user_id = save.next_user_id;
        let mut seen = BTreeSet::new();
        let mut additions: BTreeMap<ChunkCoord, Vec<usize>> = BTreeMap::new();
        let mut changes = GlobalChange::default();
        // Resolve generated identities once per owner chunk while validating a potentially large save.
        let mut generated_chunks = BTreeMap::new();
        let mut edits = save.edits;
        // Untrusted save ordering must not turn one chunk's many edited parts into repeated terrain
        // regeneration. Sorting preserves bounded temporary memory and one pass per owner chunk.
        edits.sort_by_key(|edit| edit.key);
        for edit in edits {
            if !seen.insert(edit.key) {
                return Err(StreamError::DuplicatePart(edit.key));
            }
            let before = match edit.key {
                PartKey::Generated { chunk, index } => {
                    if !generated_chunks.contains_key(&chunk) {
                        // Retain only one validation chunk to keep deserialization memory bounded.
                        generated_chunks.clear();
                        generated_chunks.insert(chunk, stream.base_chunk(chunk)?);
                    }
                    let parts = &generated_chunks[&chunk].parts;
                    let original = if stream.mapped.is_some() {
                        parts.iter().find(|part| part.key == edit.key)
                    } else {
                        parts.get(index as usize)
                    };
                    Some(
                        original
                            .ok_or(StreamError::MissingPart(edit.key))?
                            .placement,
                    )
                }
                PartKey::User(id) => {
                    if id == 0 || id >= save.next_user_id || edit.value.is_none() {
                        return Err(StreamError::InvalidSave(
                            "Invalid user identity or empty user override".into(),
                        ));
                    }
                    None
                }
            };
            if let Some(value) = edit.value {
                validate_placement(value)?;
                for chunk in placement_chunks(value)? {
                    additions
                        .entry(chunk)
                        .or_default()
                        .push(changes.changes.len());
                    stream
                        .edit_chunks
                        .entry(chunk)
                        .or_default()
                        .insert(edit.key);
                }
            }
            changes.changes.push(PartChange {
                key: edit.key,
                before,
                after: edit.value,
            });
            stream.edits.insert(edit.key, edit.value);
        }
        for (chunk, indices) in additions {
            stream.validate_chunk_change(chunk, &changes, &indices, &seen)?;
        }
        // Cache generation remains lazy after load; requesting the saved origin uses the same budget API.
        Ok(stream)
    }
}
