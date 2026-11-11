use crate::{error::invalid, limits::limit, ImportError, ImportLimits, ImportResult};
use base64::Engine;
use serde_json::Value;

pub(super) fn index(value: &Value, name: &str) -> ImportResult<usize> {
    value
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| invalid(format!("glTF {name} must be a nonnegative index")))
}
fn offset(value: &Value) -> ImportResult<usize> {
    if value.is_null() {
        Ok(0)
    } else {
        index(value, "byte offset")
    }
}
pub(super) struct Data {
    pub kind: String,
    pub component: u32,
    pub normalized: bool,
    pub values: Vec<[f64; 4]>,
}
pub(super) struct Accessors<'a> {
    json: &'a Value,
    buffers: Vec<Vec<u8>>,
    limits: &'a ImportLimits,
}
impl<'a> Accessors<'a> {
    pub fn view_bytes(&self, view: usize) -> ImportResult<&[u8]> {
        let (data, start, end, _) = self.view(view)?;
        Ok(&data[start..end])
    }
    pub fn new(
        json: &'a Value,
        bin: Option<&[u8]>,
        limits: &'a ImportLimits,
    ) -> ImportResult<Self> {
        let descriptors = json["buffers"]
            .as_array()
            .ok_or_else(|| invalid("glTF has no buffers"))?;
        let mut buffers = Vec::new();
        let mut bytes = 0usize;
        for (i, buffer) in descriptors.iter().enumerate() {
            let length = index(&buffer["byteLength"], "buffer length")?;
            limit("decoded buffer bytes", length, limits.max_input_bytes)?;
            let mut data = if let Some(uri) = buffer["uri"].as_str() {
                let Some((header, data)) = uri.split_once(',') else {
                    return Err(ImportError::Unsupported(
                        "external glTF buffers are not fetched; export GLB or embedded data URIs"
                            .into(),
                    ));
                };
                if !header.starts_with("data:") || !header.ends_with(";base64") {
                    return Err(ImportError::Unsupported(
                        "glTF buffers must use base64 data URIs or a GLB BIN chunk".into(),
                    ));
                }
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|_| invalid("glTF buffer has invalid base64"))?
            } else if i == 0 {
                bin.ok_or_else(|| invalid("glTF buffer 0 has no URI or GLB BIN chunk"))?
                    .to_vec()
            } else {
                return Err(invalid("only buffer 0 may use the GLB BIN chunk"));
            };
            if data.len() < length
                || buffer["uri"].is_null() && data.len() > length.saturating_add(3)
            {
                return Err(invalid(
                    "glTF buffer byteLength does not match supplied binary data",
                ));
            }
            bytes = bytes.saturating_add(data.len());
            limit("decoded buffer bytes", bytes, limits.max_input_bytes)?;
            // GLB padding and extra URI bytes are not addressable buffer data.
            data.truncate(length);
            buffers.push(data);
        }
        Ok(Self {
            json,
            buffers,
            limits,
        })
    }
    fn view(&self, view: usize) -> ImportResult<(&[u8], usize, usize, Option<usize>)> {
        let value = self.json["bufferViews"]
            .get(view)
            .ok_or_else(|| invalid("glTF bufferView index is out of range"))?;
        let buffer = index(&value["buffer"], "buffer")?;
        let data = self
            .buffers
            .get(buffer)
            .ok_or_else(|| invalid("glTF buffer index is out of range"))?;
        let start = offset(&value["byteOffset"])?;
        let length = index(&value["byteLength"], "bufferView length")?;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| invalid("glTF bufferView exceeds its buffer"))?;
        let stride = if value["byteStride"].is_null() {
            None
        } else {
            let stride = index(&value["byteStride"], "byte stride")?;
            if !(4..=252).contains(&stride) || stride % 4 != 0 {
                return Err(invalid(
                    "glTF byteStride must be a multiple of four between 4 and 252",
                ));
            }
            Some(stride)
        };
        Ok((data, start, end, stride))
    }
    pub fn read(&self, accessor: usize) -> ImportResult<Data> {
        let value = self.json["accessors"]
            .get(accessor)
            .ok_or_else(|| invalid("glTF accessor index is out of range"))?;
        let count = index(&value["count"], "accessor count")?;
        if count == 0 {
            return Err(invalid("glTF accessor count must be positive"));
        }
        limit(
            "accessor elements",
            count,
            self.limits
                .max_vertices
                .max(self.limits.max_triangles.saturating_mul(3)),
        )?;
        let kind = value["type"]
            .as_str()
            .ok_or_else(|| invalid("glTF accessor has no type"))?;
        let components = match kind {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => {
                return Err(ImportError::Unsupported(format!(
                    "mesh accessor type {kind}"
                )))
            }
        };
        let component = u32::try_from(index(&value["componentType"], "component type")?)
            .map_err(|_| invalid("glTF component type is out of range"))?;
        let size = component_size(component)?;
        let normalized = value["normalized"].as_bool().unwrap_or(false);
        if normalized && ![5121, 5123].contains(&component) {
            return Err(invalid(
                "only unsigned byte/short mesh attributes may be normalized",
            ));
        }
        let mut values = vec![[0.0; 4]; count];
        if !value["bufferView"].is_null() {
            let (data, start, end, stride) =
                self.view(index(&value["bufferView"], "bufferView")?)?;
            let relative = offset(&value["byteOffset"])?;
            let stride = stride.unwrap_or(size * components);
            if stride < size * components || stride % size != 0 || relative % size != 0 {
                return Err(invalid("glTF accessor has invalid alignment/stride"));
            }
            let base = start
                .checked_add(relative)
                .ok_or_else(|| invalid("glTF accessor offset overflow"))?;
            if base % size != 0 {
                return Err(invalid("glTF accessor offset is not component aligned"));
            }
            checked_span(base, stride, count, size * components, end)?;
            for (i, out) in values.iter_mut().enumerate() {
                let at = base
                    .checked_add(
                        i.checked_mul(stride)
                            .ok_or_else(|| invalid("glTF accessor stride overflow"))?,
                    )
                    .ok_or_else(|| invalid("glTF accessor range overflow"))?;
                *out = read_element(data, at, end, component, components, normalized)?;
            }
        }
        if let Some(sparse) = value.get("sparse") {
            let amount = index(&sparse["count"], "sparse count")?;
            if amount > count {
                return Err(invalid("sparse accessor has more values than its count"));
            }
            let indices = &sparse["indices"];
            let index_type = u32::try_from(index(&indices["componentType"], "sparse index type")?)
                .map_err(|_| invalid("glTF sparse index type is out of range"))?;
            if ![5121, 5123, 5125].contains(&index_type) {
                return Err(invalid("sparse indices must be unsigned integers"));
            }
            let (idata, istart, iend, istride) =
                self.view(index(&indices["bufferView"], "sparse indices bufferView")?)?;
            let (vdata, vstart, vend, vstride) = self.view(index(
                &sparse["values"]["bufferView"],
                "sparse values bufferView",
            )?)?;
            let ibase = istart
                .checked_add(offset(&indices["byteOffset"])?)
                .ok_or_else(|| invalid("sparse offset overflow"))?;
            let vbase = vstart
                .checked_add(offset(&sparse["values"]["byteOffset"])?)
                .ok_or_else(|| invalid("sparse offset overflow"))?;
            let isize = component_size(index_type)?;
            if istride.is_some() || vstride.is_some() || ibase % isize != 0 || vbase % size != 0 {
                return Err(invalid(
                    "glTF sparse data must be tightly packed and component aligned",
                ));
            }
            checked_span(ibase, isize, amount, isize, iend)?;
            checked_span(vbase, size * components, amount, size * components, vend)?;
            let mut previous = None;
            for i in 0..amount {
                let target =
                    read_element(idata, ibase + i * isize, iend, index_type, 1, false)?[0] as usize;
                if target >= count || previous.is_some_and(|p| target <= p) {
                    return Err(invalid(
                        "sparse indices must be increasing and within accessor count",
                    ));
                }
                values[target] = read_element(
                    vdata,
                    vbase + i * size * components,
                    vend,
                    component,
                    components,
                    normalized,
                )?;
                previous = Some(target);
            }
        }
        Ok(Data {
            kind: kind.into(),
            component,
            normalized,
            values,
        })
    }
}
fn checked_span(
    base: usize,
    stride: usize,
    count: usize,
    element_size: usize,
    end: usize,
) -> ImportResult<()> {
    if count == 0 {
        return Err(invalid("glTF sparse/accessor count must be positive"));
    }
    base.checked_add(
        (count - 1)
            .checked_mul(stride)
            .ok_or_else(|| invalid("glTF accessor span overflow"))?,
    )
    .and_then(|last| last.checked_add(element_size))
    .filter(|last| *last <= end)
    .ok_or_else(|| invalid("glTF accessor exceeds its bufferView"))?;
    Ok(())
}
fn component_size(component: u32) -> ImportResult<usize> {
    match component {
        5121 => Ok(1),
        5123 => Ok(2),
        5125 | 5126 => Ok(4),
        _ => Err(ImportError::Unsupported(format!(
            "mesh component type {component}; export float positions and unsigned indices/colors"
        ))),
    }
}
fn read_element(
    data: &[u8],
    at: usize,
    end: usize,
    component: u32,
    components: usize,
    normalized: bool,
) -> ImportResult<[f64; 4]> {
    let size = component_size(component)?;
    let last = at
        .checked_add(size * components)
        .filter(|last| *last <= end && *last <= data.len())
        .ok_or_else(|| invalid("glTF accessor extends beyond its bufferView"))?;
    let bytes = &data[at..last];
    let mut out = [0.0; 4];
    for i in 0..components {
        let b = &bytes[i * size..(i + 1) * size];
        out[i] = match component {
            5121 => b[0] as f64 / if normalized { 255.0 } else { 1.0 },
            5123 => {
                u16::from_le_bytes([b[0], b[1]]) as f64 / if normalized { 65535.0 } else { 1.0 }
            }
            5125 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            5126 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            _ => unreachable!(),
        };
        if !out[i].is_finite() {
            return Err(invalid("glTF accessor contains a non-finite number"));
        }
    }
    Ok(out)
}
