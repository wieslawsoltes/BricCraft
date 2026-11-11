use crate::{limits::limit, GridBounds, ImportLimits, ImportResult};
use std::collections::BTreeMap;

pub(crate) struct Voxels<'a> {
    pub cells: BTreeMap<[i32; 3], u8>,
    pub work: usize,
    pub limits: &'a ImportLimits,
}
impl<'a> Voxels<'a> {
    pub fn new(limits: &'a ImportLimits) -> Self {
        Self {
            cells: BTreeMap::new(),
            work: 0,
            limits,
        }
    }
    pub fn tick(&mut self) -> ImportResult<()> {
        self.work = self.work.saturating_add(1);
        limit("raster work", self.work, self.limits.max_work)
    }
    pub fn insert(&mut self, position: [i32; 3], color: u8) -> ImportResult<()> {
        self.tick()?;
        if !self.cells.contains_key(&position) {
            limit(
                "occupied voxels",
                self.cells.len() + 1,
                self.limits.max_voxels,
            )?;
        }
        self.cells.insert(position, color);
        Ok(())
    }
    pub fn bounds(&self) -> GridBounds {
        let mut bounds = GridBounds {
            min: [i32::MAX; 3],
            max: [i32::MIN; 3],
        };
        for p in self.cells.keys() {
            for a in 0..3 {
                bounds.min[a] = bounds.min[a].min(p[a]);
                bounds.max[a] = bounds.max[a].max(p[a] + 1);
            }
        }
        if self.cells.is_empty() {
            GridBounds::default()
        } else {
            bounds
        }
    }
}
