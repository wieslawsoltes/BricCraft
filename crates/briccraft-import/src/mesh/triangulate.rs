use crate::{error::invalid, limits::limit, ImportLimits, ImportResult};
use glam::{DVec2, DVec3};

/// Ear clipping preserves concave OBJ polygons; a triangle fan does not.
pub(super) fn polygon(
    indices: &[usize],
    positions: &[DVec3],
    limits: &ImportLimits,
    work: &mut usize,
) -> ImportResult<Vec<[usize; 3]>> {
    if indices.len() < 3 {
        return Err(invalid("an OBJ face needs at least three vertices"));
    }
    if indices.len() > 4096 {
        return Err(invalid(
            "an OBJ face has more than 4096 vertices; triangulate it in the authoring tool",
        ));
    }
    if indices.len() == 3 {
        return Ok(vec![[indices[0], indices[1], indices[2]]]);
    }
    let mut normal = DVec3::ZERO;
    for i in 0..indices.len() {
        let a = positions[indices[i]];
        let b = positions[indices[(i + 1) % indices.len()]];
        normal += (a - b).cross(a + b);
    }
    let n = normal.abs();
    let drop = if n.x >= n.y && n.x >= n.z {
        0
    } else if n.y >= n.z {
        1
    } else {
        2
    };
    let points: Vec<_> = indices
        .iter()
        .map(|i| {
            let p = positions[*i];
            match drop {
                0 => DVec2::new(p.y, p.z),
                1 => DVec2::new(p.x, p.z),
                _ => DVec2::new(p.x, p.y),
            }
        })
        .collect();
    let area: f64 = (0..points.len())
        .map(|i| points[i].perp_dot(points[(i + 1) % points.len()]))
        .sum();
    if area.abs() < 1e-14 {
        return Err(invalid("degenerate OBJ polygon"));
    }
    let sign = area.signum();
    let mut remaining: Vec<_> = (0..indices.len()).collect();
    let mut triangles = Vec::new();
    while remaining.len() > 3 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            let [pa, pb, pc] = [points[a], points[b], points[c]];
            if (pb - pa).perp_dot(pc - pb) * sign <= 1e-14 {
                continue;
            }
            let mut occupied = false;
            for &j in &remaining {
                *work = work.saturating_add(1);
                limit("polygon triangulation work", *work, limits.max_work)?;
                if j == a || j == b || j == c {
                    continue;
                }
                let p = points[j];
                if (pb - pa).perp_dot(p - pa) * sign >= -1e-14
                    && (pc - pb).perp_dot(p - pb) * sign >= -1e-14
                    && (pa - pc).perp_dot(p - pc) * sign >= -1e-14
                {
                    occupied = true;
                    break;
                }
            }
            if !occupied {
                ear = Some((i, [indices[a], indices[b], indices[c]]));
                break;
            }
        }
        let Some((index, triangle)) = ear else {
            return Err(invalid(
                "OBJ polygon is self-intersecting or degenerate; export triangulated faces",
            ));
        };
        triangles.push(triangle);
        remaining.remove(index);
    }
    triangles.push([
        indices[remaining[0]],
        indices[remaining[1]],
        indices[remaining[2]],
    ]);
    Ok(triangles)
}
