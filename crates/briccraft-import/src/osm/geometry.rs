pub(super) fn same(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

/// Join open relation member ways in either direction. Incomplete rings are
/// omitted, never closed across missing source geometry.
pub(super) fn rings(
    mut segments: Vec<Vec<[f64; 2]>>,
    work: &mut usize,
    maximum: usize,
) -> crate::ImportResult<(Vec<Vec<[f64; 2]>>, usize)> {
    let mut result = Vec::new();
    let mut incomplete = 0;
    while let Some(mut ring) = segments.pop() {
        loop {
            if ring.len() > 3 && same(ring[0], *ring.last().unwrap()) {
                result.push(ring);
                break;
            }
            let Some(end) = ring.last().copied() else {
                incomplete += 1;
                break;
            };
            let mut next = None;
            for (index, segment) in segments.iter().enumerate() {
                *work = work.saturating_add(1);
                crate::limits::limit("relation ring assembly work", *work, maximum)?;
                if segment.first().is_some_and(|p| same(*p, end))
                    || segment.last().is_some_and(|p| same(*p, end))
                {
                    next = Some(index);
                    break;
                }
            }
            let Some(index) = next else {
                incomplete += 1;
                break;
            };
            let mut segment = segments.swap_remove(index);
            if !same(segment[0], end) {
                segment.reverse();
            }
            ring.extend(segment.into_iter().skip(1));
        }
    }
    Ok((result, incomplete))
}

pub(super) fn contains(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut inside = false;
    for edge in ring.windows(2) {
        let [a, b] = [edge[0], edge[1]];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
pub(super) fn in_polygon(outer: &[Vec<[f64; 2]>], inner: &[Vec<[f64; 2]>], p: [f64; 2]) -> bool {
    outer.iter().any(|r| contains(r, p)) && !inner.iter().any(|r| contains(r, p))
}
pub(super) fn segment_distance_squared(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let length = d[0] * d[0] + d[1] * d[1];
    let t = if length > 1e-20 {
        ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / length
    } else {
        0.0
    };
    let t = t.clamp(0.0, 1.0);
    (p[0] - a[0] - d[0] * t).powi(2) + (p[1] - a[1] - d[1] * t).powi(2)
}
