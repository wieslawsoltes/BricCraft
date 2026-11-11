use crate::{DesignError, MAX_DIMENSIONS, MAX_PARTS};
use briccraft_parts::{part, Shape, PALETTE};
use briccraft_world::Placement;

fn definition(placement: &Placement) -> Result<[u8; 3], DesignError> {
    let part = part(placement.part_id).ok_or(DesignError::UnknownPart(placement.part_id))?;
    if usize::from(placement.color) >= PALETTE.len() {
        return Err(DesignError::InvalidColor(placement.color));
    }
    if placement.rotation > 3 {
        return Err(DesignError::InvalidRotation(placement.rotation));
    }
    Ok(part.rotated_size(placement.rotation))
}

/// Normalize arbitrary source cells before narrowing their small, validated relative offsets.
pub(crate) fn normalize(source: &[Placement]) -> Result<Vec<Placement>, DesignError> {
    if source.is_empty() {
        return Err(DesignError::Empty);
    }
    if source.len() > MAX_PARTS {
        return Err(DesignError::TooManyParts);
    }
    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for placement in source {
        let size = definition(placement)?;
        for axis in 0..3 {
            min[axis] = min[axis].min(i64::from(placement.position[axis]));
            max[axis] = max[axis].max(i64::from(placement.position[axis]) + i64::from(size[axis]));
        }
    }
    if (0..3).any(|axis| max[axis] - min[axis] > i64::from(MAX_DIMENSIONS[axis])) {
        return Err(DesignError::TooLarge);
    }
    Ok(source
        .iter()
        .map(|source| {
            let mut placement = *source;
            placement.position =
                std::array::from_fn(|axis| (i64::from(source.position[axis]) - min[axis]) as i32);
            placement
        })
        .collect())
}

/// Pairwise envelopes avoid allocating a whole render-world occupancy grid for a tiny template.
pub(crate) fn validate(parts: &[Placement]) -> Result<([u8; 3], u16), DesignError> {
    if parts.is_empty() {
        return Err(DesignError::Empty);
    }
    if parts.len() > MAX_PARTS {
        return Err(DesignError::TooManyParts);
    }
    let mut min = [i32::MAX; 3];
    let mut max = [0_i32; 3];
    let mut ends = Vec::with_capacity(parts.len());
    for placement in parts {
        let size = definition(placement)?;
        let mut end = [0; 3];
        for axis in 0..3 {
            end[axis] = placement.position[axis]
                .checked_add(i32::from(size[axis]))
                .ok_or(DesignError::CoordinateOverflow)?;
            if placement.position[axis] < 0 || end[axis] > i32::from(MAX_DIMENSIONS[axis]) {
                return Err(DesignError::TooLarge);
            }
            min[axis] = min[axis].min(placement.position[axis]);
            max[axis] = max[axis].max(end[axis]);
        }
        ends.push(end);
    }
    if min != [0; 3] {
        return Err(DesignError::InvalidLibrary);
    }
    let mut neighbors = vec![Vec::new(); parts.len()];
    for a in 0..parts.len() {
        for b in a + 1..parts.len() {
            let overlap = std::array::from_fn::<_, 3, _>(|axis| {
                parts[a].position[axis] < ends[b][axis] && parts[b].position[axis] < ends[a][axis]
            });
            if overlap.into_iter().all(|v| v) {
                return Err(DesignError::Overlap);
            }
            for axis in 0..3 {
                let face = ends[a][axis] == parts[b].position[axis]
                    || ends[b][axis] == parts[a].position[axis];
                if face && overlap[(axis + 1) % 3] && overlap[(axis + 2) % 3] {
                    // Match world assembly selection: a wheel resting on a floor is not snapped to it.
                    let wheel_floor = axis == 1
                        && [parts[a].part_id, parts[b].part_id]
                            .into_iter()
                            .any(|id| part(id).is_some_and(|p| p.shape == Shape::Wheel));
                    if !wheel_floor {
                        neighbors[a].push(b);
                        neighbors[b].push(a);
                    }
                    break;
                }
            }
        }
    }
    let mut visited = vec![false; parts.len()];
    let mut components = 0_u16;
    for root in 0..parts.len() {
        if visited[root] {
            continue;
        }
        components += 1;
        let mut stack = vec![root];
        visited[root] = true;
        while let Some(index) = stack.pop() {
            for &neighbor in &neighbors[index] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
    }
    Ok((max.map(|v| v as u8), components))
}
