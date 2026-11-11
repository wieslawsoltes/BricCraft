use crate::grid::{CollisionGrid, Grid, EPSILON, SKIN};
use crate::CharacterConfig;
use glam::{IVec3, Vec2, Vec3};

#[derive(Clone, Copy)]
pub(crate) struct HorizontalMove {
    pub feet: Vec3,
    pub normals: [Vec2; 4],
    pub contacts: usize,
}

/// Earliest contact between a moving circle and a rectangle. Flat faces and
/// rounded Minkowski corners are separate analytic tests, so diagonal motion
/// neither clips corners nor inherits the axis-order bias of X-then-Z movement.
fn circle_cast(p: Vec2, delta: Vec2, min: Vec2, max: Vec2, radius: f32) -> Option<(f32, Vec2)> {
    let mut best = 1.0 + EPSILON;
    let mut normal = Vec2::ZERO;
    for axis in 0..2 {
        if delta[axis].abs() <= EPSILON {
            continue;
        }
        let other = 1 - axis;
        let sign = -delta[axis].signum();
        let face = if delta[axis] > 0.0 {
            min[axis] - radius
        } else {
            max[axis] + radius
        };
        let t = (face - p[axis]) / delta[axis];
        let along = p[other] + delta[other] * t;
        if t >= -EPSILON && t <= best && along >= min[other] && along <= max[other] {
            best = t.max(0.0);
            normal = Vec2::ZERO;
            normal[axis] = sign;
        }
    }
    let a = delta.length_squared();
    if a > EPSILON * EPSILON {
        for (corner, quadrant) in [
            (min, Vec2::splat(-1.0)),
            (Vec2::new(min.x, max.y), Vec2::new(-1.0, 1.0)),
            (Vec2::new(max.x, min.y), Vec2::new(1.0, -1.0)),
            (max, Vec2::ONE),
        ] {
            let offset = p - corner;
            let b = offset.dot(delta);
            let c = offset.length_squared() - radius * radius;
            let discriminant = b * b - a * c;
            if discriminant < 0.0 {
                continue;
            }
            let t = (-b - discriminant.sqrt()) / a;
            if t < -EPSILON || t > best {
                continue;
            }
            let relative = p + delta * t - corner;
            if (relative * quadrant).min_element() < -EPSILON {
                continue;
            }
            let n = relative.normalize_or_zero();
            if n.dot(delta) < -EPSILON {
                best = t.max(0.0);
                normal = n;
            }
        }
    }
    (best <= 1.0 && normal.length_squared() > 0.5).then_some((best, normal))
}

fn first_contact<G: CollisionGrid + ?Sized>(
    grid: &Grid<'_, G>,
    config: &CharacterConfig,
    feet: Vec3,
    delta: Vec2,
) -> Option<(f32, Vec2)> {
    let end = feet + Vec3::new(delta.x, 0.0, delta.y);
    let radius = Vec3::new(config.radius, 0.0, config.radius);
    let (min, max) = grid.cells(
        feet.min(end) - radius + Vec3::Y * EPSILON,
        feet.max(end) + radius + Vec3::Y * (config.height - EPSILON),
    );
    let mut best: Option<(f32, Vec2)> = None;
    for z in min.z..=max.z {
        for x in min.x..=max.x {
            if !(min.y..=max.y).any(|y| grid.source.is_solid(IVec3::new(x, y, z))) {
                continue;
            }
            let (lo, hi) = grid.column_rect(x, z);
            if let Some(hit) = circle_cast(Vec2::new(feet.x, feet.z), delta, lo, hi, config.radius)
            {
                if best.is_none_or(|old| hit.0 < old.0) {
                    best = Some(hit);
                }
            }
        }
    }
    best
}

pub(crate) fn horizontal<G: CollisionGrid + ?Sized>(
    grid: &Grid<'_, G>,
    config: &CharacterConfig,
    mut feet: Vec3,
    mut delta: Vec2,
) -> HorizontalMove {
    let mut result = HorizontalMove {
        feet,
        normals: [Vec2::ZERO; 4],
        contacts: 0,
    };
    for _ in 0..4 {
        if delta.length_squared() <= EPSILON * EPSILON {
            break;
        }
        let Some((time, normal)) = first_contact(grid, config, feet, delta) else {
            feet += Vec3::new(delta.x, 0.0, delta.y);
            break;
        };
        let advance = (time - SKIN / delta.length()).max(0.0);
        feet += Vec3::new(delta.x, 0.0, delta.y) * advance;
        delta *= 1.0 - time;
        delta -= normal * delta.dot(normal).min(0.0);
        result.normals[result.contacts] = normal;
        result.contacts += 1;
    }
    result.feet = feet;
    result
}

/// A continuous vertical sweep. The occupied volume between the old and new
/// head/feet is queried, so a fast fall cannot skip a one-plate floor.
pub(crate) fn vertical<G: CollisionGrid + ?Sized>(
    grid: &Grid<'_, G>,
    config: &CharacterConfig,
    feet: Vec3,
    delta: f32,
) -> (f32, bool) {
    if delta.abs() <= EPSILON {
        return (delta, false);
    }
    let (min, max) = grid.cells(
        feet + Vec3::new(-config.radius, delta.min(0.0) - SKIN, -config.radius),
        feet + Vec3::new(
            config.radius,
            config.height + delta.max(0.0) + SKIN,
            config.radius,
        ),
    );
    let mut allowed = delta;
    let mut hit = false;
    if delta < 0.0 && feet.y + delta < SKIN {
        allowed = (SKIN - feet.y).min(0.0);
        hit = true;
    }
    for z in min.z..=max.z {
        for x in min.x..=max.x {
            if !grid.overlaps_column(feet, config.radius, x, z) {
                continue;
            }
            for y in min.y..=max.y {
                if !grid.source.is_solid(IVec3::new(x, y, z)) {
                    continue;
                }
                if delta < 0.0 {
                    let top = (y + 1) as f32 * grid.size.y;
                    let distance = top - feet.y;
                    if distance <= EPSILON && distance >= allowed - SKIN {
                        allowed = (distance + SKIN).min(0.0);
                        hit = true;
                    }
                } else {
                    let bottom = y as f32 * grid.size.y;
                    let distance = bottom - (feet.y + config.height);
                    if distance >= -EPSILON && distance <= allowed + SKIN {
                        allowed = (distance - SKIN).max(0.0);
                        hit = true;
                    }
                }
            }
        }
    }
    (allowed, hit)
}
