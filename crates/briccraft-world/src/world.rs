use crate::{BuildError, BuildOp, Instance, Placement, WORLD_SIZE};
use briccraft_parts::{part, Shape, PALETTE};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct World {
    pub seed: u64,
    pub(crate) size: [u16; 3],
    pub(crate) slots: Vec<Option<Instance>>,
    pub(crate) cells: Vec<u32>,
    pub(crate) active: usize,
    pub(crate) revision: u64,
}

impl World {
    pub fn empty(seed: u64) -> Self {
        Self {
            seed,
            size: WORLD_SIZE,
            slots: Vec::new(),
            cells: vec![0; WORLD_SIZE.into_iter().map(usize::from).product()],
            active: 0,
            revision: 0,
        }
    }
    pub fn dimensions(&self) -> [u16; 3] {
        self.size
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn len(&self) -> usize {
        self.active
    }
    pub fn is_empty(&self) -> bool {
        self.active == 0
    }
    pub fn instances(&self) -> impl Iterator<Item = &Instance> {
        self.slots.iter().filter_map(Option::as_ref)
    }
    pub fn instance(&self, id: u32) -> Option<&Instance> {
        self.slots.get(id as usize)?.as_ref()
    }
    pub fn occupancy(&self) -> &[u32] {
        &self.cells
    }
    pub fn index(&self, cell: [i32; 3]) -> Option<usize> {
        if cell
            .iter()
            .zip(self.size)
            .any(|(&v, bound)| v < 0 || v >= i32::from(bound))
        {
            return None;
        }
        Some(
            (cell[1] as usize * self.size[2] as usize + cell[2] as usize) * self.size[0] as usize
                + cell[0] as usize,
        )
    }
    pub fn cell(&self, position: [i32; 3]) -> Option<&Instance> {
        let id = self.cells[self.index(position)?];
        id.checked_sub(1).and_then(|id| self.instance(id))
    }
    pub fn occupied(&self, position: [i32; 3]) -> bool {
        self.cell(position).is_some()
    }

    pub(crate) fn footprint(&self, placement: &Placement) -> Result<Vec<usize>, BuildError> {
        let definition =
            part(placement.part_id).ok_or(BuildError::UnknownPart(placement.part_id))?;
        if placement.color as usize >= PALETTE.len() {
            return Err(BuildError::InvalidColor(placement.color));
        }
        if placement.rotation > 3 {
            return Err(BuildError::InvalidRotation(placement.rotation));
        }
        let size = definition.rotated_size(placement.rotation);
        let p = placement.position;
        for axis in 0..3 {
            if p[axis] < 0
                || p[axis]
                    .checked_add(i32::from(size[axis]))
                    .is_none_or(|end| end > i32::from(self.size[axis]))
            {
                return Err(BuildError::OutOfBounds(p));
            }
        }
        let mut cells = Vec::with_capacity(size.into_iter().map(usize::from).product());
        for y in p[1]..p[1] + i32::from(size[1]) {
            for z in p[2]..p[2] + i32::from(size[2]) {
                let base =
                    (y as usize * self.size[2] as usize + z as usize) * self.size[0] as usize;
                for x in p[0]..p[0] + i32::from(size[0]) {
                    cells.push(base + x as usize);
                }
            }
        }
        Ok(cells)
    }

    pub(crate) fn coordinate(&self, index: usize) -> [i32; 3] {
        let x = index % self.size[0] as usize;
        let rest = index / self.size[0] as usize;
        [
            x as i32,
            (rest / self.size[2] as usize) as i32,
            (rest % self.size[2] as usize) as i32,
        ]
    }

    pub fn can_place(&self, placement: &Placement) -> Result<(), BuildError> {
        for index in self.footprint(placement)? {
            if self.cells[index] != 0 {
                return Err(BuildError::Occupied(self.coordinate(index)));
            }
        }
        Ok(())
    }

    pub fn place(&mut self, placement: Placement) -> Result<u32, BuildError> {
        let changes = self.apply(&[BuildOp::Place(placement)])?;
        Ok(changes.added[0].id)
    }

    pub fn remove(&mut self, id: u32) -> Result<Instance, BuildError> {
        Ok(self.apply(&[BuildOp::Remove(id)])?.removed.remove(0))
    }

    pub fn extract(&mut self, ids: &[u32]) -> Result<Vec<Instance>, BuildError> {
        let ops: Vec<_> = ids.iter().copied().map(BuildOp::Remove).collect();
        Ok(self.apply(&ops)?.removed)
    }

    pub fn restore(&mut self, instances: &[Instance]) -> Result<(), BuildError> {
        self.apply_exact(&[], instances)
    }

    pub fn surface_height(&self, x: f32, z: f32) -> f32 {
        self.surface_height_excluding(x, z, &[])
    }

    pub fn surface_height_excluding(&self, x: f32, z: f32, ignore: &[u32]) -> f32 {
        if !x.is_finite() || !z.is_finite() {
            return 0.0;
        }
        let [x, z] = [x.floor() as i32, z.floor() as i32];
        if x < 0 || z < 0 || x >= self.size[0] as i32 || z >= self.size[2] as i32 {
            return 0.0;
        }
        for y in (0..self.size[1] as i32).rev() {
            if self
                .cell([x, y, z])
                .is_some_and(|instance| !ignore.contains(&instance.id))
            {
                return (y + 1) as f32 * 0.4;
            }
        }
        0.0
    }

    /// Face-connected construction, bounded so selecting terrain cannot allocate an unbounded flood.
    pub fn connected_assembly(&self, id: u32, max_parts: usize) -> Vec<u32> {
        if self.instance(id).is_none() || max_parts == 0 {
            return Vec::new();
        }
        let mut visited = HashSet::from([id]);
        let mut queue = VecDeque::from([id]);
        let mut out = Vec::new();
        while let Some(current) = queue.pop_front() {
            out.push(current);
            if out.len() == max_parts {
                break;
            }
            let instance = self.instance(current).expect("queued active instance");
            let p = instance.position;
            let s = instance.size().map(i32::from);
            for axis in 0..3 {
                let a = (axis + 1) % 3;
                let b = (axis + 2) % 3;
                for side in [p[axis] - 1, p[axis] + s[axis]] {
                    for u in p[a]..p[a] + s[a] {
                        for v in p[b]..p[b] + s[b] {
                            let mut cell = p;
                            cell[axis] = side;
                            cell[a] = u;
                            cell[b] = v;
                            if let Some(neighbor) = self.cell(cell) {
                                // Tires roll on a floor; their vertical contact is not a snapped joint.
                                // Side contacts still find axles, hubs, and the chassis.
                                if axis == 1
                                    && [instance.part_id, neighbor.part_id].into_iter().any(|id| {
                                        part(id).is_some_and(|definition| {
                                            definition.shape == Shape::Wheel
                                        })
                                    })
                                {
                                    continue;
                                }
                                if visited.len() < max_parts && visited.insert(neighbor.id) {
                                    queue.push_back(neighbor.id);
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
