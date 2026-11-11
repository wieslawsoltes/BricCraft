use std::ops::Index;

/// Compact source-to-GPU IDs. Selection needs one lookup per frame; packing
/// never needs a per-voxel lookup. A sorted vector avoids hash construction
/// and keeps sparse source identities from allocating a huge indexed array.
#[derive(Debug, Default)]
pub struct PartLookup {
    entries: Vec<(u32, u32)>,
}

impl PartLookup {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn insert(&mut self, source: u32, packed: u32) {
        self.entries.push((source, packed));
    }

    pub(crate) fn finish(&mut self) {
        self.entries.sort_unstable_by_key(|entry| entry.0);
    }

    pub fn get(&self, source: &u32) -> Option<&u32> {
        self.entries
            .binary_search_by_key(source, |entry| entry.0)
            .ok()
            .map(|index| &self.entries[index].1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn storage_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<(u32, u32)>()
    }
}

impl Index<&u32> for PartLookup {
    type Output = u32;

    fn index(&self, source: &u32) -> &Self::Output {
        self.get(source).expect("part id is absent from this atlas")
    }
}
