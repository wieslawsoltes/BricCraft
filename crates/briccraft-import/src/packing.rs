use crate::{
    limits::{limit, progress},
    voxels::Voxels,
    ImportProgress, ImportResult, ImportStage,
};
use briccraft_parts::CATALOG;
use briccraft_world::Placement;

/// Exact-cover packing: a part replaces cells only if every covered cell exists
/// with the same color. It never fills holes or grows the sampled silhouette.
pub(crate) fn pack(
    mut grid: Voxels<'_>,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<Vec<Placement>> {
    let mut candidates = Vec::new();
    for part in CATALOG.iter().filter(|p| p.id < 40) {
        for rotation in 0..=u8::from(part.size[0] != part.size[2]) {
            let size = part.rotated_size(rotation);
            candidates.push((part.id, rotation, size));
        }
    }
    candidates.sort_by_key(|(id, rotation, size)| {
        (
            std::cmp::Reverse(size.iter().map(|v| *v as usize).product::<usize>()),
            *id,
            *rotation,
        )
    });
    let total = grid.cells.len();
    let mut placements = Vec::new();
    progress(callback, ImportStage::Pack, 0, total)?;
    while let Some((&position, &color)) = grid.cells.first_key_value() {
        let &(id, rotation, size) = candidates
            .iter()
            .find(|(_, _, size)| {
                (0..size[0] as i32).all(|x| {
                    (0..size[1] as i32).all(|y| {
                        (0..size[2] as i32).all(|z| {
                            grid.cells
                                .get(&[position[0] + x, position[1] + y, position[2] + z])
                                == Some(&color)
                        })
                    })
                })
            })
            .expect("the catalog contains a unit plate");
        limit("parts", placements.len() + 1, grid.limits.max_parts)?;
        for x in 0..size[0] as i32 {
            for y in 0..size[1] as i32 {
                for z in 0..size[2] as i32 {
                    grid.cells
                        .remove(&[position[0] + x, position[1] + y, position[2] + z]);
                }
            }
        }
        placements.push(Placement::new(id, color, position, rotation));
        if placements.len() % 256 == 0 {
            progress(callback, ImportStage::Pack, total - grid.cells.len(), total)?;
        }
    }
    progress(callback, ImportStage::Complete, total, total)?;
    Ok(placements)
}
