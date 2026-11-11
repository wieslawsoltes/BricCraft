use crate::atlas::AtlasError;
use crate::metrics::{ATLAS_WIDTH, MAX_ATLAS_TEXELS};

#[derive(Debug, PartialEq, Eq)]
pub struct PixelAtlas {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl PixelAtlas {
    pub fn new(texels: usize) -> Result<Self, AtlasError> {
        if texels > MAX_ATLAS_TEXELS {
            return Err(AtlasError::TextureTooLarge);
        }
        let height = texels.max(1).div_ceil(ATLAS_WIDTH);
        let mut pixels = vec![0; ATLAS_WIDTH * height * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        Ok(Self {
            pixels,
            width: ATLAS_WIDTH,
            height,
        })
    }

    pub fn put(&mut self, offset: usize, rgb: [u8; 3]) {
        self.pixels[offset * 4..offset * 4 + 3].copy_from_slice(&rgb);
    }

    /// Writes the validated cuboid directly. Packing scales with occupied
    /// cells, with no world-volume scan or hash-table lookup for every voxel.
    pub fn fill_box(
        &mut self,
        offset: usize,
        dimensions: [usize; 3],
        origin: [usize; 3],
        size: [usize; 3],
        id: u32,
    ) {
        let [r, g, b] = encode_id(id);
        let pixel = [r, g, b, 255];
        for y in origin[1]..origin[1] + size[1] {
            for z in origin[2]..origin[2] + size[2] {
                let start = (offset + (y * dimensions[2] + z) * dimensions[0] + origin[0]) * 4;
                for target in self.pixels[start..start + size[0] * 4].chunks_exact_mut(4) {
                    target.copy_from_slice(&pixel);
                }
            }
        }
    }
}

pub(crate) fn encode_id(id: u32) -> [u8; 3] {
    [id as u8, (id >> 8) as u8, (id >> 16) as u8]
}
