use briccraft_parts::{part, PALETTE};
use briccraft_world::World;
use std::collections::{HashMap, HashSet};
use std::fmt;

const TEXTURE_WIDTH: usize = 1024;
const MAX_TEXTURE_SIDE: usize = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasStats {
    pub instances: usize,
    pub occupied_cells: usize,
    pub texture_bytes: usize,
    pub uploads: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtlasError {
    TooManyInstances,
    TextureTooLarge,
    InvalidPart(u16),
    InvalidColor(u8),
    InvalidGeometry(u32),
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyInstances => {
                write!(f, "world exceeds the renderer's 24-bit instance capacity")
            }
            Self::TextureTooLarge => write!(f, "world exceeds the renderer's texture capacity"),
            Self::InvalidPart(id) => write!(f, "world contains unknown part {id}"),
            Self::InvalidColor(id) => write!(f, "world contains unknown color {id}"),
            Self::InvalidGeometry(id) => {
                write!(f, "instance {id} is outside the packed world bounds")
            }
        }
    }
}

impl std::error::Error for AtlasError {}

#[derive(Debug)]
pub(crate) struct PixelAtlas {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl PixelAtlas {
    fn new(texels: usize) -> Result<Self, AtlasError> {
        let height = texels.max(1).div_ceil(TEXTURE_WIDTH);
        if height > MAX_TEXTURE_SIDE {
            return Err(AtlasError::TextureTooLarge);
        }
        let mut pixels = vec![0; TEXTURE_WIDTH * height * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        Ok(Self {
            pixels,
            width: TEXTURE_WIDTH,
            height,
        })
    }

    fn put(&mut self, offset: usize, rgb: [u8; 3]) {
        self.pixels[offset * 4..offset * 4 + 3].copy_from_slice(&rgb);
    }
}

pub(crate) struct SceneAtlas {
    pub cells: PixelAtlas,
    pub parts: PixelAtlas,
    pub packed_ids: HashMap<u32, u32>,
    pub vehicle_min: [i32; 3],
    pub vehicle_size: [u16; 3],
    pub vehicle_offset: usize,
    pub coarse_offset: usize,
    pub stats: AtlasStats,
}

impl SceneAtlas {
    pub fn build(world: &World, members: &[u32]) -> Result<Self, AtlasError> {
        let selected: HashSet<_> = members.iter().copied().collect();
        let instances: Vec<_> = world.instances().collect();
        if instances.len() >= 0x00ff_ffff {
            return Err(AtlasError::TooManyInstances);
        }
        let dimensions = world.dimensions();
        let volume = dimensions
            .iter()
            .map(|n| usize::from(*n))
            .product::<usize>();
        let mut lower = [i32::MAX; 3];
        let mut upper = [i32::MIN; 3];
        for instance in &instances {
            if selected.contains(&instance.id) {
                let p = part(instance.part_id).ok_or(AtlasError::InvalidPart(instance.part_id))?;
                let size = p.rotated_size(instance.rotation);
                for axis in 0..3 {
                    lower[axis] = lower[axis].min(instance.position[axis]);
                    upper[axis] = upper[axis].max(instance.position[axis] + i32::from(size[axis]));
                }
            }
        }
        let has_vehicle = lower[0] != i32::MAX;
        let vehicle_min = if has_vehicle { lower } else { [0; 3] };
        let vehicle_size: [u16; 3] = if has_vehicle {
            std::array::from_fn(|i| (upper[i] - lower[i]) as u16)
        } else {
            [0; 3]
        };
        let vehicle_volume = vehicle_size
            .iter()
            .map(|n| usize::from(*n))
            .product::<usize>();
        let coarse_size = [
            usize::from(dimensions[0]).div_ceil(4),
            (usize::from(dimensions[1]) + 1).div_ceil(6),
            usize::from(dimensions[2]).div_ceil(4),
        ];
        let coarse_offset = volume + vehicle_volume;
        let mut cells = PixelAtlas::new(coarse_offset + coarse_size.iter().product::<usize>())?;
        // Four texels per part: origin, dimensions, material, shape/rotation.
        let mut parts = PixelAtlas::new((instances.len() + 1) * 4)?;
        let mut packed_ids = HashMap::with_capacity(instances.len());
        for (index, instance) in instances.iter().enumerate() {
            let packed_id = (index + 1) as u32;
            packed_ids.insert(instance.id, packed_id);
            let p = part(instance.part_id).ok_or(AtlasError::InvalidPart(instance.part_id))?;
            let mut position = instance.position;
            if selected.contains(&instance.id) {
                for axis in 0..3 {
                    position[axis] -= vehicle_min[axis];
                }
            }
            if position.iter().any(|v| !(0..256).contains(v)) {
                return Err(AtlasError::InvalidGeometry(instance.id));
            }
            let color = *PALETTE
                .get(usize::from(instance.color))
                .ok_or(AtlasError::InvalidColor(instance.color))?;
            parts.put(packed_id as usize * 4, position.map(|v| v as u8));
            parts.put(
                packed_id as usize * 4 + 1,
                p.rotated_size(instance.rotation),
            );
            parts.put(packed_id as usize * 4 + 2, color);
            parts.put(
                packed_id as usize * 4 + 3,
                [
                    p.shape as u8,
                    instance.rotation % 4,
                    u8::from(selected.contains(&instance.id)),
                ],
            );
        }
        let mut occupied = 0;
        for (index, &id_plus_one) in world.occupancy().iter().enumerate() {
            if id_plus_one == 0 {
                continue;
            }
            let id = id_plus_one - 1;
            let Some(&packed) = packed_ids.get(&id) else {
                continue;
            };
            let offset = if selected.contains(&id) {
                let x = index % usize::from(dimensions[0]);
                let yz = index / usize::from(dimensions[0]);
                let z = yz % usize::from(dimensions[2]);
                let y = yz / usize::from(dimensions[2]);
                let local = [
                    x as i32 - vehicle_min[0],
                    y as i32 - vehicle_min[1],
                    z as i32 - vehicle_min[2],
                ];
                volume
                    + ((local[1] as usize * usize::from(vehicle_size[2]) + local[2] as usize)
                        * usize::from(vehicle_size[0])
                        + local[0] as usize)
            } else {
                index
            };
            cells.put(offset, encode_id(packed));
            if !selected.contains(&id) {
                let x = index % usize::from(dimensions[0]);
                let yz = index / usize::from(dimensions[0]);
                let z = yz % usize::from(dimensions[2]);
                let y = yz / usize::from(dimensions[2]);
                // Also flag the row above: studs extend into that placement cell.
                for cy in [y / 6, (y + 1) / 6] {
                    let coarse = (cy * coarse_size[2] + z / 4) * coarse_size[0] + x / 4;
                    cells.put(coarse_offset + coarse, [1, 0, 0]);
                }
            }
            occupied += 1;
        }
        let bytes = cells.pixels.len() + parts.pixels.len();
        Ok(Self {
            cells,
            parts,
            packed_ids,
            vehicle_min,
            vehicle_size,
            vehicle_offset: volume,
            coarse_offset,
            stats: AtlasStats {
                instances: instances.len(),
                occupied_cells: occupied,
                texture_bytes: bytes,
                uploads: 0,
            },
        })
    }
}

fn encode_id(id: u32) -> [u8; 3] {
    [id as u8, (id >> 8) as u8, (id >> 16) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_ids_preserve_every_byte_at_channel_boundaries() {
        for value in [0, 1, 255, 256, 65535, 65536, 0x00ff_ffff] {
            let [r, g, b] = encode_id(value);
            assert_eq!(u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16, value);
        }
    }

    #[test]
    fn data_texture_padding_is_opaque_to_preserve_encoded_channels() {
        let atlas = PixelAtlas::new(1025).unwrap();
        assert_eq!(atlas.height, 2);
        assert!(atlas.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn active_assembly_moves_to_local_volume_without_remaining_in_static_grid() {
        use briccraft_world::Placement;
        let mut world = World::empty(1);
        let active = world.place(Placement::new(3, 0, [4, 3, 4], 0)).unwrap();
        let fixed = world.place(Placement::new(3, 1, [20, 6, 20], 0)).unwrap();
        let atlas = SceneAtlas::build(&world, &[active]).unwrap();
        assert_eq!(atlas.vehicle_min, [4, 3, 4]);
        assert_eq!(atlas.vehicle_size, [2, 3, 4]);
        let static_index = world.index([4, 3, 4]).unwrap();
        assert_eq!(
            &atlas.cells.pixels[static_index * 4..static_index * 4 + 3],
            &[0, 0, 0]
        );
        let packed = atlas.packed_ids[&active];
        assert_eq!(
            &atlas.cells.pixels[atlas.vehicle_offset * 4..atlas.vehicle_offset * 4 + 3],
            &encode_id(packed)
        );
        let fixed_index = world.index([20, 6, 20]).unwrap();
        assert_eq!(
            &atlas.cells.pixels[fixed_index * 4..fixed_index * 4 + 3],
            &encode_id(atlas.packed_ids[&fixed])
        );
        assert_eq!(
            &atlas.parts.pixels[packed as usize * 16..packed as usize * 16 + 3],
            &[0, 0, 0]
        );
    }

    #[test]
    fn coarse_tiles_include_stud_overhang_above_a_tile_boundary() {
        use briccraft_world::Placement;
        let mut world = World::empty(1);
        world.place(Placement::new(1, 0, [7, 3, 7], 0)).unwrap();
        let atlas = SceneAtlas::build(&world, &[]).unwrap();
        let coarse_width = 96_usize.div_ceil(4);
        let coarse_depth = 96_usize.div_ceil(4);
        let below = (7 / 4) * coarse_width + 7 / 4;
        let above = coarse_depth * coarse_width + below;
        assert_eq!(atlas.cells.pixels[(atlas.coarse_offset + below) * 4], 1);
        assert_eq!(atlas.cells.pixels[(atlas.coarse_offset + above) * 4], 1);
    }
}
