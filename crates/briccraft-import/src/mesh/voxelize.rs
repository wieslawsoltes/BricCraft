use super::{MeshOptions, Triangle};
use crate::{
    color,
    error::invalid,
    limits::{limit, progress},
    voxels::Voxels,
    ImportProgress, ImportReport, ImportResult, ImportStage,
};
use glam::DVec3;
use std::collections::{HashSet, VecDeque};

fn overlap(vertices: [DVec3; 3], center: DVec3) -> bool {
    let p = vertices.map(|v| v - center);
    for axis in 0..3 {
        if p.iter().all(|v| v[axis] > 0.50000001) || p.iter().all(|v| v[axis] < -0.50000001) {
            return false;
        }
    }
    let edges = [p[1] - p[0], p[2] - p[1], p[0] - p[2]];
    let separated = |axis: DVec3| {
        if axis.length_squared() < 1e-24 {
            return false;
        }
        let radius = axis.abs().element_sum() * 0.5 + 1e-9;
        let d = p.map(|v| v.dot(axis));
        d.iter().all(|v| *v > radius) || d.iter().all(|v| *v < -radius)
    };
    if separated(edges[0].cross(edges[1])) {
        return false;
    }
    for edge in edges {
        for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
            if separated(edge.cross(axis)) {
                return false;
            }
        }
    }
    true
}
fn color_at(triangle: &Triangle, p: [DVec3; 3], center: DVec3) -> u8 {
    let a = p[1] - p[0];
    let b = p[2] - p[0];
    let v = center - p[0];
    let denominator = a.dot(a) * b.dot(b) - a.dot(b).powi(2);
    let mut weights = if denominator.abs() > 1e-20 {
        let u = (b.dot(b) * v.dot(a) - a.dot(b) * v.dot(b)) / denominator;
        let w = (a.dot(a) * v.dot(b) - a.dot(b) * v.dot(a)) / denominator;
        [1.0 - u - w, u, w]
    } else {
        [1.0 / 3.0; 3]
    };
    weights = weights.map(|w| w.clamp(0.0, 1.0));
    let sum = weights.iter().sum::<f64>().max(1e-20);
    let mut rgb = [0.0; 3];
    for channel in 0..3 {
        for i in 0..3 {
            rgb[channel] += triangle.colors[i][channel] * weights[i] / sum;
        }
    }
    if let Some(texture) = &triangle.texture {
        let mut uv = [0.0; 2];
        for channel in 0..2 {
            for i in 0..3 {
                uv[channel] += triangle.uvs[i][channel] * weights[i] / sum;
            }
        }
        let texel = texture.sample(uv);
        for channel in 0..3 {
            rgb[channel] *= texel[channel];
        }
    }
    if triangle.linear_colors {
        rgb = rgb.map(color::linear_to_srgb);
    }
    color::nearest(rgb)
}

pub(super) fn render(
    triangles: &[Triangle],
    options: &MeshOptions,
    grid: &mut Voxels<'_>,
    report: &mut ImportReport,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<()> {
    let mut minimum = DVec3::splat(f64::INFINITY);
    let mut maximum = DVec3::splat(f64::NEG_INFINITY);
    for triangle in triangles {
        for p in triangle.positions {
            minimum = minimum.min(p);
            maximum = maximum.max(p);
        }
    }
    let extent = maximum - minimum;
    let longest = extent.max_element();
    if !longest.is_finite() || longest < 1e-12 {
        return Err(invalid("mesh has no finite spatial extent"));
    }
    let mut scale = options
        .units_per_stud
        .map_or(options.fit_size_studs / longest, |units| 1.0 / units);
    if let Some(height) = options.max_height_plates {
        let available = height as f64 * 0.4;
        if extent.y * scale > available {
            if options.units_per_stud.is_some() {
                return Err(invalid(format!("model requires {:.0} plates; available {height}; choose a larger units-per-stud value or uniform fit",(extent.y*scale/0.4).ceil())));
            }
            scale = available / extent.y;
            report.warn("The fitted model was uniformly scaled to the available height; its aspect ratio is preserved.");
        }
    }
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("model scale is not finite and positive"));
    }
    report.source_bounds = Some([minimum.to_array(), maximum.to_array()]);
    report.studs_per_source_unit = Some(scale);
    let anchor = DVec3::new(
        (minimum.x + maximum.x) * 0.5,
        minimum.y,
        (minimum.z + maximum.z) * 0.5,
    );
    let convert = |p: DVec3| {
        let q = (p - anchor) * scale;
        DVec3::new(
            q.x + options.origin[0] as f64,
            q.y / 0.4 + options.origin[1] as f64,
            q.z + options.origin[2] as f64,
        )
    };
    // Uniform fitting can land a few ULPs beyond an integer plate boundary.
    // Remove arithmetic noise before ceil/floor, without changing mesh scale.
    let snap = |value: f64| {
        let nearest = value.round();
        if (value - nearest).abs() <= 1e-7 {
            nearest
        } else {
            value
        }
    };
    let low = DVec3::from_array(convert(minimum).to_array().map(snap));
    let high = DVec3::from_array(convert(maximum).to_array().map(snap));
    if low.abs().max_element() > 1_500_000_000.0 || high.abs().max_element() > 1_500_000_000.0 {
        return Err(invalid("scaled model exceeds the global coordinate range"));
    }
    let bounds_min = [
        low.x.floor() as i32,
        low.y.floor() as i32,
        low.z.floor() as i32,
    ];
    let bounds_max = [
        (high.x.ceil() as i32).max(bounds_min[0] + 1),
        (high.y.ceil() as i32).max(bounds_min[1] + 1),
        (high.z.ceil() as i32).max(bounds_min[2] + 1),
    ];
    progress(callback, ImportStage::Voxelize, 0, triangles.len())?;
    for (index, triangle) in triangles.iter().enumerate() {
        if index % 128 == 0 {
            progress(callback, ImportStage::Voxelize, index, triangles.len())?;
        }
        let p = triangle.positions.map(convert);
        if (p[1] - p[0]).cross(p[2] - p[0]).length_squared() < 1e-20 {
            report.warn("Degenerate zero-area triangles were skipped.");
            continue;
        }
        let min = p[0].min(p[1]).min(p[2]);
        let max = p[0].max(p[1]).max(p[2]);
        let a = [
            ((min.x - 1e-7).floor() as i32).max(bounds_min[0]),
            ((min.y - 1e-7).floor() as i32).max(bounds_min[1]),
            ((min.z - 1e-7).floor() as i32).max(bounds_min[2]),
        ];
        let b = [
            ((max.x + 1e-7).floor() as i32).min(bounds_max[0] - 1),
            ((max.y + 1e-7).floor() as i32).min(bounds_max[1] - 1),
            ((max.z + 1e-7).floor() as i32).min(bounds_max[2] - 1),
        ];
        for x in a[0]..=b[0] {
            for y in a[1]..=b[1] {
                for z in a[2]..=b[2] {
                    grid.tick()?;
                    let center = DVec3::new(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5);
                    if overlap(p, center) {
                        grid.insert([x, y, z], color_at(triangle, p, center))?;
                    }
                }
            }
        }
    }
    if options.fill_interior {
        fill(grid, bounds_min, bounds_max, options.color, callback)?;
    }
    Ok(())
}

fn fill(
    grid: &mut Voxels<'_>,
    minimum: [i32; 3],
    maximum: [i32; 3],
    color: u8,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<()> {
    let low = minimum.map(|v| v - 1);
    let high = maximum.map(|v| v + 1);
    let volume = (0..3)
        .try_fold(1usize, |n, a| n.checked_mul((high[a] - low[a]) as usize))
        .ok_or_else(|| invalid("mesh fill volume overflow"))?;
    limit("interior flood-fill volume", volume, grid.limits.max_work)?;
    let mut outside = HashSet::from([low]);
    let mut queue = VecDeque::from([low]);
    while let Some(p) = queue.pop_front() {
        grid.tick()?;
        if outside.len() % 16384 == 0 {
            progress(callback, ImportStage::Voxelize, outside.len(), volume)?;
        }
        for d in [
            [-1, 0, 0],
            [1, 0, 0],
            [0, -1, 0],
            [0, 1, 0],
            [0, 0, -1],
            [0, 0, 1],
        ] {
            let q = [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
            if (0..3).all(|a| q[a] >= low[a] && q[a] < high[a])
                && !grid.cells.contains_key(&q)
                && outside.insert(q)
            {
                queue.push_back(q);
            }
        }
    }
    for x in minimum[0]..maximum[0] {
        for y in minimum[1]..maximum[1] {
            for z in minimum[2]..maximum[2] {
                let p = [x, y, z];
                if !outside.contains(&p) && !grid.cells.contains_key(&p) {
                    grid.insert(p, color)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_box_test_preserves_holes_in_bounding_boxes() {
        let t = [
            DVec3::ZERO,
            DVec3::new(4.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 4.0),
        ];
        assert!(overlap(t, DVec3::new(0.5, 0.5, 0.5)));
        assert!(!overlap(t, DVec3::new(3.5, 0.5, 3.5)));
        assert!(!overlap(t, DVec3::new(0.5, 2.0, 0.5)));
    }
}
