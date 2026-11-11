use super::{triangulate, Mesh, MeshOptions, Triangle};
use crate::{
    error::invalid,
    limits::{limit, progress},
    ImportProgress, ImportReport, ImportResult, ImportStage, SourceKind,
};
use glam::DVec3;
use std::collections::HashMap;

fn numbers<'a>(values: impl Iterator<Item = &'a str>) -> ImportResult<Vec<f64>> {
    values
        .map(|v| {
            v.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && v.abs() < 1e15)
                .ok_or_else(|| invalid("OBJ contains an invalid or non-finite number"))
        })
        .collect()
}
fn material_library(source: Option<&str>) -> ImportResult<HashMap<String, [f64; 3]>> {
    let mut materials = HashMap::new();
    let mut name = String::new();
    for line in source.unwrap_or("").lines() {
        let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
        match fields.next() {
            Some("newmtl") => name = fields.collect::<Vec<_>>().join(" "),
            Some("Kd") => {
                let c = numbers(fields)?;
                if c.len() >= 3 {
                    materials.insert(
                        name.clone(),
                        [
                            c[0].clamp(0.0, 1.0),
                            c[1].clamp(0.0, 1.0),
                            c[2].clamp(0.0, 1.0),
                        ],
                    );
                }
            }
            _ => {}
        }
    }
    Ok(materials)
}
pub(super) fn parse(
    source: &str,
    mtl: Option<&str>,
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<Mesh> {
    let mut mesh = Mesh {
        triangles: Vec::new(),
        report: ImportReport::new(SourceKind::Obj),
    };
    mesh.report.source_name = "OBJ mesh".into();
    let mut positions = Vec::new();
    let mut colors: Vec<Option<[f64; 3]>> = Vec::new();
    let mut work = 0;
    let materials = material_library(mtl)?;
    let rgb = briccraft_parts::PALETTE[options.color as usize];
    let default = rgb.map(|c| c as f64 / 255.0);
    let mut material = default;
    let mut pending = String::new();
    let mut consumed = 0;
    for (line_number, line) in source.split_inclusive('\n').enumerate() {
        consumed += line.len();
        if line_number % 4096 == 0 {
            progress(callback, ImportStage::Parse, consumed, source.len())?;
        }
        let line = line.trim_end();
        if let Some(prefix) = line.strip_suffix('\\') {
            pending.push_str(prefix);
            pending.push(' ');
            continue;
        }
        let line = if pending.is_empty() {
            std::borrow::Cow::Borrowed(line)
        } else {
            pending.push_str(line);
            std::borrow::Cow::Owned(std::mem::take(&mut pending))
        };
        let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
        match fields.next() {
            Some("v") => {
                let values = numbers(fields)?;
                if ![3, 4, 6, 7].contains(&values.len()) {
                    return Err(invalid(format!("OBJ vertex on line {} must contain xyz, xyzw or xyzRGB", line_number + 1)));
                }
                let w = if values.len() == 4 || values.len() == 7 { values[3] } else { 1.0 };
                if w.abs() < 1e-20 { return Err(invalid("OBJ homogeneous vertex has zero w")); }
                limit("vertices", positions.len() + 1, options.limits.max_vertices)?;
                let position = DVec3::new(values[0] / w, values[1] / w, values[2] / w);
                if !position.is_finite() || position.abs().max_element() > 1e15 {
                    return Err(invalid("OBJ homogeneous position is not finite or is too large"));
                }
                positions.push(position);
                let offset = if values.len() == 7 { 4 } else { 3 };
                colors.push((values.len() >= 6).then(|| {
                    let c = [values[offset], values[offset + 1], values[offset + 2]];
                    let divisor = if c.iter().any(|v| *v > 1.0) { 255.0 } else { 1.0 };
                    c.map(|v| (v / divisor).clamp(0.0, 1.0))
                }));
            }
            Some("f") => {
                let indices: Vec<usize> = fields.map(|field| {
                    let raw = field.split('/').next().unwrap_or("").parse::<i64>()
                        .map_err(|_| invalid("OBJ face index is not an integer"))?;
                    let index = if raw > 0 { raw - 1 } else if raw < 0 {
                        positions.len() as i64 + raw
                    } else { return Err(invalid("OBJ indices are 1-based; zero is invalid")); };
                    if index < 0 || index >= positions.len() as i64 {
                        return Err(invalid(format!("OBJ face on line {} references a missing vertex", line_number + 1)));
                    }
                    Ok(index as usize)
                }).collect::<ImportResult<_>>()?;
                mesh.report.input_features += 1;
                for triangle in triangulate::polygon(&indices, &positions, &options.limits, &mut work)? {
                    limit("triangles", mesh.triangles.len() + 1, options.limits.max_triangles)?;
                    mesh.triangles.push(Triangle {
                        positions: triangle.map(|i| positions[i]), colors: triangle.map(|i| colors[i].unwrap_or(material)),
                        linear_colors: false, texture: None, uvs: [[0.0; 2]; 3],
                    });
                }
                mesh.report.imported_features += 1;
            }
            Some("usemtl") => {
                let name = fields.collect::<Vec<_>>().join(" ");
                material = materials.get(&name).copied().unwrap_or(default);
                if !materials.contains_key(&name) { mesh.report.warn("OBJ material definitions were not supplied; missing materials use the selected default color."); }
            }
            Some("mtllib") => {
                if mtl.is_none() { mesh.report.warn("External OBJ material libraries and texture images are not fetched. Supply an MTL string or use GLB vertex/material colors."); }
            }
            Some("vt") => mesh.report.warn("OBJ texture images are not sampled; vertex colors, MTL diffuse colors or the selected default color are used. Embed textures in a GLB to import their colors."),
            Some("curv" | "surf") => return Err(crate::ImportError::Unsupported("OBJ free-form curves/surfaces must be tessellated to faces before import".into())),
            _ => {}
        }
    }
    if !pending.is_empty() {
        return Err(invalid("OBJ ends inside a continued line"));
    }
    Ok(mesh)
}
