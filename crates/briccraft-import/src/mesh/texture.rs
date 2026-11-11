use super::{
    accessor::{index, Accessors},
    texture_decode::{self, Image},
};
use crate::{
    color::srgb_to_linear, error::invalid, limits::limit, ImportLimits, ImportReport, ImportResult,
};
use base64::Engine;
use serde_json::Value;
use std::{collections::HashMap, rc::Rc};

pub(super) struct Texture {
    image: Rc<Image>,
    wrap: [u32; 2],
    linear: bool,
    offset: [f64; 2],
    scale: [f64; 2],
    rotation: f64,
}
impl Texture {
    pub fn sample(&self, mut uv: [f64; 2]) -> [f64; 3] {
        uv[0] *= self.scale[0];
        uv[1] *= self.scale[1];
        let (sin, cos) = self.rotation.sin_cos();
        let mut uv = [
            self.offset[0] + cos * uv[0] - sin * uv[1],
            self.offset[1] + sin * uv[0] + cos * uv[1],
        ];
        for (i, value) in uv.iter_mut().enumerate() {
            *value = match self.wrap[i] {
                33071 => value.clamp(0.0, 1.0),
                33648 => {
                    let t = value.rem_euclid(2.0);
                    if t <= 1.0 {
                        t
                    } else {
                        2.0 - t
                    }
                }
                _ => value.rem_euclid(1.0),
            };
        }
        let x = uv[0] * self.image.width as f64 - 0.5;
        let y = uv[1] * self.image.height as f64 - 0.5;
        if !self.linear {
            return self.pixel((x + 0.5).floor() as i64, (y + 0.5).floor() as i64);
        }
        let (ix, iy) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = (x - x.floor(), y - y.floor());
        let taps = [
            (ix, iy, (1.0 - fx) * (1.0 - fy)),
            (ix + 1, iy, fx * (1.0 - fy)),
            (ix, iy + 1, (1.0 - fx) * fy),
            (ix + 1, iy + 1, fx * fy),
        ];
        let mut result = [0.0; 3];
        for (x, y, weight) in taps {
            let pixel = self.pixel(x, y);
            for i in 0..3 {
                result[i] += pixel[i] * weight;
            }
        }
        result
    }
    fn pixel(&self, x: i64, y: i64) -> [f64; 3] {
        let coord = |value: i64, size: usize, mode: u32| -> usize {
            let size = size as i64;
            match mode {
                33071 => value.clamp(0, size - 1) as usize,
                33648 => {
                    let t = value.rem_euclid(size * 2);
                    if t < size {
                        t as usize
                    } else {
                        (size * 2 - 1 - t) as usize
                    }
                }
                _ => value.rem_euclid(size) as usize,
            }
        };
        let at = (coord(y, self.image.height, self.wrap[1]) * self.image.width
            + coord(x, self.image.width, self.wrap[0]))
            * 3;
        [0, 1, 2].map(|i| srgb_to_linear(self.image.rgb[at + i] as f64 / 255.0))
    }
}

#[derive(Default)]
pub(super) struct Textures {
    images: HashMap<usize, Rc<Image>>,
    bytes: usize,
}
impl Textures {
    pub fn binding(
        &mut self,
        document: &Value,
        accessors: &Accessors<'_>,
        info: &Value,
        limits: &ImportLimits,
        report: &mut ImportReport,
    ) -> ImportResult<Option<(Rc<Texture>, usize)>> {
        if info.is_null() {
            return Ok(None);
        }
        let descriptor = document["textures"]
            .get(index(&info["index"], "base-color texture")?)
            .ok_or_else(|| invalid("glTF texture index is out of range"))?;
        if descriptor["source"].is_null() {
            report.warn("A texture without a core PNG/JPEG source was omitted; its material factors remain available.");
            return Ok(None);
        }
        let source = index(&descriptor["source"], "texture image")?;
        let image = if let Some(image) = self.images.get(&source) {
            image.clone()
        } else {
            let descriptor = document["images"]
                .get(source)
                .ok_or_else(|| invalid("glTF image index is out of range"))?;
            let decoded;
            let data = if let Some(uri) = descriptor["uri"].as_str() {
                let Some((header, encoded)) = uri.split_once(',') else {
                    report.warn("External texture images are not fetched; embed PNG/JPEG images in GLB or a base64 data URI to preserve their colors.");
                    return Ok(None);
                };
                if !header.starts_with("data:") || !header.ends_with(";base64") {
                    return Err(invalid("embedded texture URI must contain base64 data"));
                }
                decoded = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|_| invalid("invalid texture base64"))?;
                limit(
                    "encoded texture bytes",
                    decoded.len(),
                    limits.max_input_bytes,
                )?;
                &decoded
            } else {
                accessors.view_bytes(index(&descriptor["bufferView"], "image bufferView")?)?
            };
            let image = Rc::new(texture_decode::decode(
                data,
                limits.max_input_bytes.saturating_sub(self.bytes),
            )?);
            self.bytes = self.bytes.saturating_add(image.rgb.len());
            limit("decoded texture bytes", self.bytes, limits.max_input_bytes)?;
            self.images.insert(source, image.clone());
            image
        };
        let sampler = if descriptor["sampler"].is_null() {
            &Value::Null
        } else {
            document["samplers"]
                .get(index(&descriptor["sampler"], "texture sampler")?)
                .ok_or_else(|| invalid("glTF sampler index is out of range"))?
        };
        let mut wrap = [10497; 2];
        for (i, key) in ["wrapS", "wrapT"].iter().enumerate() {
            if !sampler[*key].is_null() {
                wrap[i] = u32::try_from(index(&sampler[*key], "texture wrap mode")?)
                    .map_err(|_| invalid("texture wrap mode out of range"))?;
                if ![10497, 33071, 33648].contains(&wrap[i]) {
                    return Err(invalid("unknown glTF texture wrap mode"));
                }
            }
        }
        let transform = &info["extensions"]["KHR_texture_transform"];
        let pair = |key: &str, default: [f64; 2]| -> ImportResult<[f64; 2]> {
            if transform[key].is_null() {
                return Ok(default);
            }
            let values = transform[key]
                .as_array()
                .filter(|v| v.len() == 2)
                .ok_or_else(|| invalid("texture transform must have two components"))?;
            Ok([finite(&values[0])?, finite(&values[1])?])
        };
        let rotation = if transform["rotation"].is_null() {
            0.0
        } else {
            finite(&transform["rotation"])?
        };
        let tex_coord = if !transform["texCoord"].is_null() {
            index(&transform["texCoord"], "texture coordinate set")?
        } else if !info["texCoord"].is_null() {
            index(&info["texCoord"], "texture coordinate set")?
        } else {
            0
        };
        Ok(Some((
            Rc::new(Texture {
                image,
                wrap,
                linear: sampler["magFilter"].as_u64() != Some(9728),
                offset: pair("offset", [0.0; 2])?,
                scale: pair("scale", [1.0; 2])?,
                rotation,
            }),
            tex_coord,
        )))
    }
}
fn finite(value: &Value) -> ImportResult<f64> {
    value
        .as_f64()
        .filter(|v| v.is_finite() && v.abs() <= 1e9)
        .ok_or_else(|| invalid("invalid texture transform component"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sampling_wraps_transforms_and_interpolates_in_linear_space() {
        let mut texture = Texture {
            image: Rc::new(Image {
                width: 2,
                height: 1,
                rgb: vec![255, 0, 0, 0, 0, 255],
            }),
            wrap: [10497; 2],
            linear: true,
            offset: [0.0; 2],
            scale: [1.0; 2],
            rotation: 0.0,
        };
        assert_eq!(texture.sample([0.25, 0.5]), [1.0, 0.0, 0.0]);
        assert_eq!(texture.sample([0.5, 0.5]), [0.5, 0.0, 0.5]);
        assert_eq!(texture.sample([1.25, 0.5]), [1.0, 0.0, 0.0]);
        texture.wrap = [33071; 2];
        assert_eq!(texture.sample([1.25, 0.5]), [0.0, 0.0, 1.0]);
        texture.wrap = [33648; 2];
        assert_eq!(texture.sample([1.25, 0.5]), [0.0, 0.0, 1.0]);
        texture.wrap = [10497; 2];
        texture.offset = [0.5, 0.0];
        assert_eq!(texture.sample([0.25, 0.5]), [0.0, 0.0, 1.0]);
        assert!(texture.sample([1e30, -1e30]).iter().all(|v| v.is_finite()));
    }
}
