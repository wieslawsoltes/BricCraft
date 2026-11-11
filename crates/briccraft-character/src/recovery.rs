use crate::grid::{CollisionGrid, Grid, SKIN};
use crate::CharacterConfig;
use glam::Vec3;

/// Finds the nearest clear cell-top above an invalid position. Unlike a height-map
/// snap, a valid room below a roof is left exactly where it is. The search is bounded
/// by the local volume's height, even when a flight camera starts inside a mountain.
pub(crate) fn recover<G: CollisionGrid + ?Sized>(
    grid: &Grid<'_, G>,
    feet: Vec3,
    config: &CharacterConfig,
) -> (Vec3, bool) {
    if grid.clear(feet, config) {
        return (feet, false);
    }
    let mut candidate = if feet.is_finite() { feet } else { Vec3::ZERO };
    candidate.y = candidate.y.max(SKIN);
    if grid.clear(candidate, config) {
        return (candidate, true);
    }
    let first = (candidate.y / grid.size.y).floor() as i32 + 1;
    for y in first..=grid.dimensions.y {
        candidate.y = y as f32 * grid.size.y + SKIN;
        if grid.clear(candidate, config) {
            return (candidate, true);
        }
    }
    // No solid cells exist above this plane. Preserve X/Z, including streamed coordinates.
    candidate.y = candidate
        .y
        .max(grid.dimensions.y as f32 * grid.size.y + SKIN);
    (candidate, true)
}
