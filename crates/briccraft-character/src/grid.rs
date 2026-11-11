use crate::CharacterConfig;
use glam::{IVec3, Vec2, Vec3};

/// Occupancy in a local, axis-aligned volume starting at the origin.
///
/// Cell dimensions must be finite and positive; dimensions must be nonnegative.
/// `is_solid` is called only for cells inside the declared volume. Horizontal space
/// outside it and space above it are empty, allowing a streamed window to rebase.
/// The plane Y=0 provides a stable bottom boundary, including below empty ocean cells.
pub trait CollisionGrid {
    fn dimensions(&self) -> IVec3;
    fn cell_size(&self) -> Vec3;
    fn is_solid(&self, cell: IVec3) -> bool;
}

pub(crate) const SKIN: f32 = 0.0001;
pub(crate) const EPSILON: f32 = 0.00001;

pub(crate) struct Grid<'a, G: CollisionGrid + ?Sized> {
    pub source: &'a G,
    pub size: Vec3,
    pub dimensions: IVec3,
}

impl<'a, G: CollisionGrid + ?Sized> Grid<'a, G> {
    pub fn new(source: &'a G) -> Option<Self> {
        let size = source.cell_size();
        let dimensions = source.dimensions();
        if !size.is_finite() || size.min_element() <= 0.0 || dimensions.min_element() < 0 {
            return None;
        }
        Some(Self {
            source,
            size,
            dimensions,
        })
    }

    /// Inclusive, volume-clipped cell bounds. Empty ranges naturally visit no cells.
    pub fn cells(&self, min: Vec3, max: Vec3) -> (IVec3, IVec3) {
        (
            (min / self.size).floor().as_ivec3().max(IVec3::ZERO),
            (max / self.size)
                .floor()
                .as_ivec3()
                .min(self.dimensions - IVec3::ONE),
        )
    }

    pub fn clear(&self, feet: Vec3, config: &CharacterConfig) -> bool {
        if !feet.is_finite() || feet.y < -EPSILON {
            return false;
        }
        let radius = config.radius;
        let (min, max) = self.cells(
            feet + Vec3::new(-radius, EPSILON, -radius),
            feet + Vec3::new(radius, config.height - EPSILON, radius),
        );
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if !self.overlaps_column(feet, radius, x, z) {
                    continue;
                }
                for y in min.y..=max.y {
                    if self.source.is_solid(IVec3::new(x, y, z)) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn column_rect(&self, x: i32, z: i32) -> (Vec2, Vec2) {
        let min = Vec2::new(x as f32 * self.size.x, z as f32 * self.size.z);
        (min, min + Vec2::new(self.size.x, self.size.z))
    }

    pub fn overlaps_column(&self, feet: Vec3, radius: f32, x: i32, z: i32) -> bool {
        let p = Vec2::new(feet.x, feet.z);
        let (min, max) = self.column_rect(x, z);
        p.distance_squared(p.clamp(min, max)) < radius * radius - EPSILON * EPSILON
    }
}
