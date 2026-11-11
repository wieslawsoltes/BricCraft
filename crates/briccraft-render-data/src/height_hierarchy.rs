//! Conservative empty-air bounds for the three primary traversal scales.
//!
//! A column stores the highest part bounding box, not a terrain approximation.
//! Consequently bridges, overhangs, buildings and imported models remain safe:
//! the shader may skip only air above the maximum, then resumes exact 3D DDA.

use crate::PixelAtlas;

pub(crate) struct HeightHierarchy {
    dimensions: [usize; 2],
    heights: Vec<u8>,
}

impl HeightHierarchy {
    pub fn new(dimensions: [usize; 3]) -> Self {
        let dimensions = [dimensions[0].div_ceil(4), dimensions[2].div_ceil(4)];
        Self {
            dimensions,
            heights: vec![0; dimensions[0] * dimensions[1]],
        }
    }

    pub fn len(&self) -> usize {
        self.heights.len()
    }

    pub fn include(&mut self, origin: [usize; 3], extent: [usize; 3]) {
        let top = (origin[1] + extent[1]) as u8;
        for z in origin[2] / 4..(origin[2] + extent[2]).div_ceil(4) {
            for x in origin[0] / 4..(origin[0] + extent[0]).div_ceil(4) {
                let height = &mut self.heights[z * self.dimensions[0] + x];
                *height = (*height).max(top);
            }
        }
    }

    pub fn write(self, atlas: &mut PixelAtlas, offset: usize) {
        let [width, depth] = self.dimensions;
        let mut encoded: Vec<[u8; 3]> = self.heights.iter().map(|height| [*height, 0, 0]).collect();
        // Repeat each parent maximum in its children's texel. One nearest
        // texture fetch then obtains all 4-, 16- and 64-stud bounds together.
        for (channel, stride) in [(1, 4), (2, 16)] {
            for z in (0..depth).step_by(stride) {
                for x in (0..width).step_by(stride) {
                    let end_x = (x + stride).min(width);
                    let end_z = (z + stride).min(depth);
                    let mut maximum = 0;
                    for row in z..end_z {
                        maximum = maximum.max(
                            self.heights[row * width + x..row * width + end_x]
                                .iter()
                                .copied()
                                .max()
                                .unwrap_or(0),
                        );
                    }
                    for row in z..end_z {
                        for column in x..end_x {
                            encoded[row * width + column][channel] = maximum;
                        }
                    }
                }
            }
        }
        for (index, value) in encoded.into_iter().enumerate() {
            atlas.put(offset + index, value);
        }
    }
}
