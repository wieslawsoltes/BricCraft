use crate::{
    error::{invalid, limit},
    HeightField, TerrainError, TerrainResult,
};
use briccraft_parts::{CATALOG, PALETTE};
use briccraft_world::Placement;

#[derive(Clone, Copy, Debug)]
pub struct TerrainPackLimits {
    pub max_parts: usize,
    pub max_work: usize,
}
impl Default for TerrainPackLimits {
    fn default() -> Self {
        Self {
            max_parts: 100_000,
            max_work: 20_000_000,
        }
    }
}

/// Exact-cover packing with one consumed-height cursor per column. Memory is
/// O(columns + resulting parts), regardless of the solid terrain's volume.
/// Surface colors occupy one plate; the editable material underneath is earth.
/// Parts remain within signed 16-stud chunks, so a chunk-aligned resident window
/// never drops a partially intersecting ground slab at its publication boundary.
pub fn pack_heightfield(
    field: &HeightField,
    limits: TerrainPackLimits,
    callback: &mut dyn FnMut(usize, usize) -> bool,
) -> TerrainResult<Vec<Placement>> {
    if field.cells.len() != field.width() * field.depth()
        || field.cells.iter().any(|c| {
            c.top <= field.base_y
                || c.top > field.base_y.saturating_add(4096)
                || c.color as usize >= PALETTE.len()
        })
    {
        return Err(invalid(
            "terrain columns have invalid heights, colors or dimensions",
        ));
    }
    let mut candidates = Vec::new();
    for part in CATALOG.iter().filter(|p| p.id < 40) {
        for rotation in 0..=u8::from(part.size[0] != part.size[2]) {
            candidates.push((part.id, rotation, part.rotated_size(rotation)));
        }
    }
    candidates.sort_by_key(|(id, rot, size)| {
        (
            std::cmp::Reverse(size.iter().map(|v| *v as usize).product::<usize>()),
            *id,
            *rot,
        )
    });
    let width = field.width();
    let depth = field.depth();
    let mut cursors = vec![field.base_y; field.cells.len()];
    let mut result = Vec::new();
    let mut work = 0usize;
    let mut covered = 0usize;
    let total = field.filled_cells();
    let bounds = field.bounds();
    if !callback(0, total) {
        return Err(TerrainError::Cancelled);
    }
    for z in 0..depth {
        for x in 0..width {
            let index = z * width + x;
            while cursors[index] < field.cells[index].top {
                let y = cursors[index];
                let color = material(field, index, y);
                let mut selected = None;
                let chunk_x = 16 - (bounds[0] + x as i32).rem_euclid(16) as usize;
                let chunk_z = 16 - (bounds[1] + z as i32).rem_euclid(16) as usize;
                'candidate: for &(id, rotation, size) in &candidates {
                    let [sx, sy, sz] = size.map(|v| v as usize);
                    if x + sx > width || z + sz > depth || sx > chunk_x || sz > chunk_z {
                        continue;
                    }
                    for dz in 0..sz {
                        for dx in 0..sx {
                            work = work.saturating_add(1);
                            limit("terrain packing work", work, limits.max_work)?;
                            let i = (z + dz) * width + x + dx;
                            if cursors[i] != y
                                || field.cells[i].top < y + sy as i32
                                || material(field, i, y) != color
                                || material(field, i, y + sy as i32 - 1) != color
                            {
                                continue 'candidate;
                            }
                        }
                    }
                    selected = Some((id, rotation, size));
                    break;
                }
                let (id, rotation, [sx, sy, sz]) =
                    selected.ok_or_else(|| invalid("catalog has no terrain unit plate"))?;
                limit("terrain parts", result.len() + 1, limits.max_parts)?;
                for dz in 0..sz as usize {
                    for dx in 0..sx as usize {
                        cursors[(z + dz) * width + x + dx] += sy as i32;
                    }
                }
                covered += sx as usize * sy as usize * sz as usize;
                result.push(Placement::new(
                    id,
                    color,
                    [bounds[0] + x as i32, y, bounds[1] + z as i32],
                    rotation,
                ));
                if result.len() % 256 == 0 && !callback(covered, total) {
                    return Err(TerrainError::Cancelled);
                }
            }
        }
    }
    if !callback(total, total) {
        return Err(TerrainError::Cancelled);
    }
    Ok(result)
}

fn material(field: &HeightField, index: usize, y: i32) -> u8 {
    if y == field.cells[index].top - 1 {
        field.cells[index].color
    } else {
        28
    }
}
