use crate::data_texture::PixelAtlas;
use crate::metrics::{AtlasStats, MAX_ATLAS_TEXELS, MAX_PACKED_PARTS, MAX_VEHICLE_PARTS};
use crate::part_lookup::PartLookup;
use briccraft_parts::{PALETTE, part};
use briccraft_world::{Instance, World};
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtlasError {
    TooManyInstances,
    TooManyVehicleParts,
    VehicleTooLarge,
    TextureTooLarge,
    InvalidPart(u16),
    InvalidColor(u8),
    InvalidGeometry(u32),
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyInstances => write!(
                f,
                "active window exceeds the {MAX_PACKED_PARTS}-part renderer budget"
            ),
            Self::TooManyVehicleParts => write!(
                f,
                "moving construction exceeds the {MAX_VEHICLE_PARTS}-part budget"
            ),
            Self::VehicleTooLarge => write!(
                f,
                "moving construction exceeds the active window's dimensions"
            ),
            Self::TextureTooLarge => {
                write!(f, "active window exceeds the renderer's texture capacity")
            }
            Self::InvalidPart(id) => write!(f, "world contains unknown part {id}"),
            Self::InvalidColor(id) => write!(f, "world contains unknown color {id}"),
            Self::InvalidGeometry(id) => {
                write!(f, "instance {id} is outside the packed volume bounds")
            }
        }
    }
}

impl std::error::Error for AtlasError {}

pub struct SceneAtlas {
    pub cells: PixelAtlas,
    pub parts: PixelAtlas,
    /// Selection IDs belong to the active World. Detached vehicle IDs have a
    /// separate packed namespace and cannot replace a colliding static ID.
    pub packed_ids: PartLookup,
    pub static_min: [u16; 3],
    pub static_max: [u16; 3],
    pub vehicle_min: [i32; 3],
    pub vehicle_size: [u16; 3],
    pub vehicle_offset: usize,
    pub coarse_offset: usize,
    pub stats: AtlasStats,
}

impl SceneAtlas {
    pub fn build(
        world: &World,
        members: &[u32],
        detached: Option<&[Instance]>,
    ) -> Result<Self, AtlasError> {
        let selected: HashSet<_> = members.iter().copied().collect();
        let mut fixed = Vec::with_capacity(world.len());
        let mut moving = Vec::new();
        for instance in world.instances() {
            if selected.contains(&instance.id) {
                if detached.is_none() {
                    moving.push(instance);
                }
            } else {
                fixed.push(instance);
            }
        }
        if let Some(detached) = detached {
            moving.extend(detached);
        }
        if moving.len() > MAX_VEHICLE_PARTS {
            return Err(AtlasError::TooManyVehicleParts);
        }
        let count = fixed.len() + moving.len();
        if count > MAX_PACKED_PARTS {
            return Err(AtlasError::TooManyInstances);
        }

        let dimensions = world.dimensions();
        let grid = dimensions.map(usize::from);
        let volume = grid.iter().product::<usize>();
        let (vehicle_min, vehicle_size) = vehicle_bounds(&moving, dimensions)?;
        let vehicle_grid = vehicle_size.map(usize::from);
        let vehicle_volume = vehicle_grid.iter().product::<usize>();
        let coarse_size = [
            grid[0].div_ceil(4),
            (grid[1] + 1).div_ceil(6),
            grid[2].div_ceil(4),
        ];
        let coarse_tiles = coarse_size.iter().product::<usize>();
        let coarse_offset = volume + vehicle_volume;
        let mut cells = PixelAtlas::new(coarse_offset + coarse_tiles)?;
        let mut parts = PixelAtlas::new((count + 1) * 4)?;
        let mut packed_ids = PartLookup::with_capacity(count);
        let mut lower = dimensions;
        let mut upper = [0_u16; 3];
        let mut occupied = 0;

        for (index, instance) in fixed.iter().chain(&moving).enumerate() {
            let is_moving = index >= fixed.len();
            let packed_id = (index + 1) as u32;
            if !is_moving || detached.is_none() {
                packed_ids.insert(instance.id, packed_id);
            }
            let p = part(instance.part_id).ok_or(AtlasError::InvalidPart(instance.part_id))?;
            let size = p.rotated_size(instance.rotation);
            let mut origin = instance.position;
            if is_moving {
                for axis in 0..3 {
                    origin[axis] = (i64::from(origin[axis]) - i64::from(vehicle_min[axis])) as i32;
                }
            }
            let bounds = if is_moving { vehicle_size } else { dimensions };
            if instance.rotation > 3
                || (0..3).any(|axis| {
                    origin[axis] < 0
                        || origin[axis] > 255
                        || size[axis] == 0
                        || origin[axis] + i32::from(size[axis]) > i32::from(bounds[axis])
                })
            {
                return Err(AtlasError::InvalidGeometry(instance.id));
            }
            let color = *PALETTE
                .get(usize::from(instance.color))
                .ok_or(AtlasError::InvalidColor(instance.color))?;
            parts.put(packed_id as usize * 4, origin.map(|v| v as u8));
            parts.put(packed_id as usize * 4 + 1, size);
            parts.put(packed_id as usize * 4 + 2, color);
            parts.put(
                packed_id as usize * 4 + 3,
                [p.shape as u8, instance.rotation, u8::from(is_moving)],
            );

            let position = origin.map(|v| v as usize);
            let extent = size.map(usize::from);
            occupied += extent.iter().product::<usize>();
            if is_moving {
                cells.fill_box(volume, vehicle_grid, position, extent, packed_id);
            } else {
                cells.fill_box(0, grid, position, extent, packed_id);
                for axis in 0..3 {
                    lower[axis] = lower[axis].min(origin[axis] as u16);
                    upper[axis] = upper[axis].max(origin[axis] as u16 + u16::from(size[axis]));
                }
                // Flag the part's coarse range once, including its top stud
                // row. The previous voxel scan repeated these writes per cell.
                let a = [position[0] / 4, position[1] / 6, position[2] / 4];
                let b = [
                    (position[0] + extent[0] - 1) / 4,
                    (position[1] + extent[1]) / 6,
                    (position[2] + extent[2] - 1) / 4,
                ];
                let coarse_extent = std::array::from_fn(|axis| b[axis] - a[axis] + 1);
                cells.fill_box(coarse_offset, coarse_size, a, coarse_extent, 1);
            }
        }
        packed_ids.finish();
        let occupied_coarse_tiles = cells.pixels
            [coarse_offset * 4..(coarse_offset + coarse_tiles) * 4]
            .chunks_exact(4)
            .filter(|pixel| pixel[0] != 0)
            .count();
        let texture_bytes = cells.pixels.len() + parts.pixels.len();
        let lookup_bytes = packed_ids.storage_bytes();
        let staging_bytes = cells.pixels.capacity() + parts.pixels.capacity() + lookup_bytes;
        if fixed.is_empty() {
            lower = [0; 3];
        }
        Ok(Self {
            cells,
            parts,
            packed_ids,
            static_min: lower,
            static_max: upper,
            vehicle_min,
            vehicle_size,
            vehicle_offset: volume,
            coarse_offset,
            stats: AtlasStats {
                instances: count,
                static_instances: fixed.len(),
                vehicle_instances: moving.len(),
                occupied_cells: occupied,
                active_cells: volume,
                vehicle_cells: vehicle_volume,
                coarse_tiles,
                occupied_coarse_tiles,
                texture_bytes,
                lookup_bytes,
                staging_bytes,
                part_budget: MAX_PACKED_PARTS,
                cell_texel_budget: MAX_ATLAS_TEXELS,
                texture_budget_bytes: MAX_ATLAS_TEXELS * 4 * 2,
                ..AtlasStats::default()
            },
        })
    }
}

fn vehicle_bounds(
    parts: &[&Instance],
    dimensions: [u16; 3],
) -> Result<([i32; 3], [u16; 3]), AtlasError> {
    if parts.is_empty() {
        return Ok(([0; 3], [0; 3]));
    }
    let mut lower = [i32::MAX; 3];
    let mut upper = [i32::MIN; 3];
    for instance in parts {
        let p = part(instance.part_id).ok_or(AtlasError::InvalidPart(instance.part_id))?;
        let size = p.rotated_size(instance.rotation);
        for axis in 0..3 {
            let end = instance.position[axis]
                .checked_add(i32::from(size[axis]))
                .ok_or(AtlasError::InvalidGeometry(instance.id))?;
            lower[axis] = lower[axis].min(instance.position[axis]);
            upper[axis] = upper[axis].max(end);
        }
    }
    let extent: [i64; 3] =
        std::array::from_fn(|axis| i64::from(upper[axis]) - i64::from(lower[axis]));
    if (0..3).any(|axis| extent[axis] > i64::from(dimensions[axis])) {
        return Err(AtlasError::VehicleTooLarge);
    }
    Ok((lower, extent.map(|v| v as u16)))
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
