use super::{MAX_MAP_REGIONS, MAX_MAP_SOURCE_PARTS, MapAnchor, MapRegion, model};
use crate::{
    CHUNK_CELLS, ChunkCoord, GlobalPart, PartKey, StreamError,
    edits::{placement_chunks, validate_placement},
    generation::Chunk,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct MapSource {
    pub anchor: MapAnchor,
    pub(super) regions: BTreeMap<(i64, i64), Arc<MapRegion>>,
    chunks: BTreeMap<ChunkCoord, Arc<Chunk>>,
    pub(super) part_count: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedMap {
    anchor: MapAnchor,
    regions: Vec<MapRegion>,
}

impl MapSource {
    pub(super) fn new(anchor: MapAnchor) -> Result<Self, StreamError> {
        anchor.validate()?;
        Ok(Self {
            anchor,
            regions: BTreeMap::new(),
            chunks: BTreeMap::new(),
            part_count: 0,
        })
    }

    pub fn saved(&self) -> SavedMap {
        SavedMap {
            anchor: self.anchor.clone(),
            regions: self.regions.values().map(|r| r.as_ref().clone()).collect(),
        }
    }

    pub fn restore(save: SavedMap) -> Result<Self, StreamError> {
        if save.regions.len() > MAX_MAP_REGIONS {
            return Err(StreamError::LimitExceeded("retained map region"));
        }
        let mut source = Self::new(save.anchor)?;
        for region in save.regions {
            let (region, chunks) = source.prepare(region)?;
            let key = (region.bounds[0], region.bounds[1]);
            if source.regions.contains_key(&key) {
                return Err(StreamError::InvalidSave(
                    "Duplicate geographic source region".into(),
                ));
            }
            if source.part_count.saturating_add(region.placements.len()) > MAX_MAP_SOURCE_PARTS {
                return Err(StreamError::LimitExceeded("retained map source part"));
            }
            source.insert(region, chunks);
        }
        Ok(source)
    }

    pub fn chunk(&self, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.chunks.get(&coord).cloned()
    }

    pub fn contains_chunk(&self, coord: &ChunkCoord) -> bool {
        self.chunks.contains_key(coord)
    }

    pub(super) fn prepare(
        &self,
        mut region: MapRegion,
    ) -> Result<(MapRegion, BTreeMap<ChunkCoord, Arc<Chunk>>), StreamError> {
        let bounds = region.bounds;
        if self.anchor.region_at(bounds[0], bounds[1])? != bounds {
            return Err(StreamError::InvalidSave(
                "Map region must cover one complete aligned source tile".into(),
            ));
        }
        if region.placements.len() > crate::MAX_TRANSACTION_PARTS {
            return Err(StreamError::LimitExceeded("map region part"));
        }
        self.anchor.unproject(bounds[0] as f64, bounds[1] as f64)?;
        self.anchor.unproject(bounds[2] as f64, bounds[3] as f64)?;
        // Source array order is not an identity. Stable spatial/catalog ordering
        // makes worker completion order and equivalent provider responses harmless.
        region.placements.sort_unstable_by_key(|p| {
            (
                p.position.chunk(),
                p.position,
                p.part_id,
                p.rotation,
                p.color,
            )
        });
        let mut chunks: BTreeMap<ChunkCoord, Vec<GlobalPart>> = BTreeMap::new();
        for z in bounds[1] / 16..bounds[3] / 16 {
            for x in bounds[0] / 16..bounds[2] / 16 {
                chunks.insert(ChunkCoord::new(x, z), Vec::new());
            }
        }
        let mut owner_indices: BTreeMap<ChunkCoord, u16> = BTreeMap::new();
        for &placement in &region.placements {
            validate_placement(placement)?;
            let size = placement.size()?;
            if !model::contains(bounds, placement.position.x, placement.position.z)
                || placement
                    .position
                    .x
                    .checked_add(i64::from(size[0]))
                    .is_none_or(|v| v > bounds[2])
                || placement
                    .position
                    .z
                    .checked_add(i64::from(size[2]))
                    .is_none_or(|v| v > bounds[3])
            {
                return Err(StreamError::InvalidSave(
                    "A map source part crosses its declared region boundary".into(),
                ));
            }
            let owner = placement.position.chunk();
            let index = owner_indices.entry(owner).or_default();
            let part = GlobalPart {
                key: PartKey::Generated {
                    chunk: owner,
                    index: *index,
                },
                placement,
            };
            *index = index
                .checked_add(1)
                .ok_or(StreamError::LimitExceeded("map chunk part identity"))?;
            for chunk in placement_chunks(placement)? {
                chunks
                    .get_mut(&chunk)
                    .expect("part contained by region")
                    .push(part);
            }
        }
        let mut output = BTreeMap::new();
        for (coord, parts) in chunks {
            let mut occupied = vec![false; CHUNK_CELLS];
            let mut occupied_cells = 0;
            for part in &parts {
                crate::edits::for_each_cell(part.placement, coord, |index, position| {
                    if occupied[index] {
                        return Err(StreamError::Occupied(position));
                    }
                    occupied[index] = true;
                    occupied_cells += 1;
                    Ok(())
                })?;
            }
            output.insert(
                coord,
                Arc::new(Chunk {
                    parts,
                    occupied_cells,
                }),
            );
        }
        Ok((region, output))
    }

    pub(super) fn insert(&mut self, region: MapRegion, chunks: BTreeMap<ChunkCoord, Arc<Chunk>>) {
        self.part_count += region.placements.len();
        self.regions
            .insert((region.bounds[0], region.bounds[1]), Arc::new(region));
        self.chunks.extend(chunks);
    }

    pub(super) fn remove(&mut self, key: (i64, i64)) -> Option<[i64; 4]> {
        let region = self.regions.remove(&key)?;
        self.part_count -= region.placements.len();
        self.chunks
            .retain(|coord, _| !model::contains(region.bounds, coord.x * 16, coord.z * 16));
        Some(region.bounds)
    }
}
