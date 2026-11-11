use super::{
    accessor::{index, Accessors},
    texture::Textures,
    Mesh, MeshOptions, Triangle,
};
use crate::{
    error::invalid,
    limits::{limit, progress},
    ImportError, ImportProgress, ImportReport, ImportResult, ImportStage, SourceKind,
};
use glam::{DMat4, DQuat, DVec3};
use serde_json::Value;
use std::collections::HashSet;

fn u32_at(bytes: &[u8], at: usize) -> ImportResult<u32> {
    let b = bytes
        .get(at..at + 4)
        .ok_or_else(|| invalid("truncated GLB header"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
pub(super) fn container(bytes: &[u8]) -> ImportResult<(&[u8], Option<&[u8]>)> {
    if bytes.len() < 20 || u32_at(bytes, 0)? != 0x46546c67 || u32_at(bytes, 4)? != 2 {
        return Err(invalid("expected a glTF 2.0 GLB container"));
    }
    if u32_at(bytes, 8)? as usize != bytes.len() {
        return Err(invalid("GLB declared length differs from file length"));
    }
    let mut at = 12;
    let mut json = None;
    let mut bin = None;
    while at < bytes.len() {
        let length = u32_at(bytes, at)? as usize;
        let kind = u32_at(bytes, at + 4)?;
        let start = at
            .checked_add(8)
            .ok_or_else(|| invalid("GLB chunk overflow"))?;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| invalid("truncated GLB chunk"))?;
        if length % 4 != 0 {
            return Err(invalid("GLB chunk length must be aligned to 4 bytes"));
        }
        if at == 12 && kind != 0x4e4f534a {
            return Err(invalid("GLB first chunk must be JSON"));
        }
        match kind {
            0x4e4f534a => {
                if json.replace(&bytes[start..end]).is_some() {
                    return Err(invalid("duplicate GLB JSON chunk"));
                }
            }
            0x004e4942 => {
                if bin.replace(&bytes[start..end]).is_some() {
                    return Err(invalid("duplicate GLB BIN chunk"));
                }
            }
            _ => {}
        }
        at = end;
    }
    Ok((json.ok_or_else(|| invalid("GLB has no JSON chunk"))?, bin))
}
fn vector<const N: usize>(value: &Value, default: [f64; N]) -> ImportResult<[f64; N]> {
    if value.is_null() {
        return Ok(default);
    }
    let a = value
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or_else(|| invalid("glTF transform/material vector has wrong length"))?;
    let mut result = [0.0; N];
    for i in 0..N {
        result[i] = a[i]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or_else(|| invalid("glTF vector contains an invalid number"))?;
    }
    Ok(result)
}
fn transform(node: &Value) -> ImportResult<DMat4> {
    if !node["matrix"].is_null() {
        if ["translation", "rotation", "scale"]
            .iter()
            .any(|key| !node[*key].is_null())
        {
            return Err(invalid("glTF node may use a matrix or TRS, not both"));
        }
        let matrix = vector(&node["matrix"], [0.0; 16])?;
        if matrix[3].abs() > 1e-8
            || matrix[7].abs() > 1e-8
            || matrix[11].abs() > 1e-8
            || (matrix[15] - 1.0).abs() > 1e-8
        {
            return Err(invalid("glTF node matrix is not affine"));
        }
        Ok(DMat4::from_cols_array(&matrix))
    } else {
        let rotation = DQuat::from_array(vector(&node["rotation"], [0.0, 0.0, 0.0, 1.0])?);
        if rotation.length_squared() < 1e-20 {
            return Err(invalid("glTF node quaternion is zero"));
        }
        Ok(DMat4::from_scale_rotation_translation(
            DVec3::from_array(vector(&node["scale"], [1.0; 3])?),
            rotation.normalize(),
            DVec3::from_array(vector(&node["translation"], [0.0; 3])?),
        ))
    }
}

pub(super) fn parse(
    json: &[u8],
    bin: Option<&[u8]>,
    source: SourceKind,
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<Mesh> {
    let document: Value = serde_json::from_slice(json)?;
    if document["asset"]["version"].as_str() != Some("2.0") {
        return Err(ImportError::Unsupported(
            "only glTF 2.0 static triangle assets are supported".into(),
        ));
    }
    let mut mesh = Mesh {
        triangles: Vec::new(),
        report: ImportReport::new(source),
    };
    mesh.report.source_name = "glTF static scene".into();
    for extension in document["extensionsRequired"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let name = extension
            .as_str()
            .ok_or_else(|| invalid("glTF extension name must be a string"))?;
        if name.starts_with("KHR_materials_") || name == "KHR_texture_transform" {
            mesh.report.warn(
                "Advanced source material effects are replaced with opaque molded brick materials.",
            );
        } else {
            return Err(ImportError::Unsupported(format!(
                "required glTF extension{name}; export an uncompressed static mesh"
            )));
        }
    }
    if document["animations"]
        .as_array()
        .is_some_and(|a| !a.is_empty())
    {
        mesh.report
            .warn("Animations are not baked: the mesh uses the scene's static default transforms.");
    }
    if let Some(copyright) = document["asset"]["copyright"].as_str() {
        mesh.report
            .warn(format!("Source asset copyright: {copyright}"));
    }
    let accessors = Accessors::new(&document, bin, &options.limits)?;
    let mut textures = Textures::default();
    let nodes = document["nodes"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    limit("scene nodes", nodes.len(), options.limits.max_features)?;
    let mut roots = Vec::new();
    if let Some(scenes) = document["scenes"].as_array() {
        if !scenes.is_empty() {
            let scene = if document["scene"].is_null() {
                0
            } else {
                index(&document["scene"], "default scene")?
            };
            let scene = scenes
                .get(scene)
                .ok_or_else(|| invalid("glTF default scene is out of range"))?;
            if let Some(name) = scene["name"].as_str() {
                mesh.report.source_name = name.into();
            }
            roots = scene["nodes"]
                .as_array()
                .map(|a| a.iter().map(|v| index(v, "root node")).collect())
                .transpose()?
                .unwrap_or_default();
        }
    } else {
        let children: HashSet<_> = nodes
            .iter()
            .flat_map(|n| n["children"].as_array().into_iter().flatten())
            .map(|v| index(v, "child node"))
            .collect::<ImportResult<_>>()?;
        roots = (0..nodes.len()).filter(|i| !children.contains(i)).collect();
    }
    let mut visited = HashSet::new();
    let mut stack: Vec<_> = roots
        .into_iter()
        .rev()
        .map(|node| (node, DMat4::IDENTITY, 0usize))
        .collect();
    while let Some((node, parent, depth)) = stack.pop() {
        if depth > 128 || !visited.insert(node) {
            return Err(invalid(
                "glTF node graph contains a cycle/shared parent or exceeds 128 levels",
            ));
        }
        let node = nodes
            .get(node)
            .ok_or_else(|| invalid("glTF node index is out of range"))?;
        if !node["skin"].is_null() {
            return Err(ImportError::Unsupported(
                "skinned meshes must be exported as a static posed mesh before brick conversion"
                    .into(),
            ));
        }
        if node["weights"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_f64().unwrap_or(0.0) != 0.0))
        {
            return Err(ImportError::Unsupported(
                "active morph targets must be baked into a static mesh".into(),
            ));
        }
        let matrix = parent * transform(node)?;
        if !node["mesh"].is_null() {
            append_mesh(
                &document,
                &accessors,
                &mut textures,
                index(&node["mesh"], "mesh")?,
                matrix,
                options,
                &mut mesh,
            )?;
        }
        for child in node["children"].as_array().into_iter().flatten().rev() {
            stack.push((index(child, "child node")?, matrix, depth + 1));
        }
        progress(callback, ImportStage::Parse, visited.len(), nodes.len())?;
    }
    if nodes.is_empty() {
        let count = document["meshes"].as_array().map_or(0, Vec::len);
        for i in 0..count {
            append_mesh(
                &document,
                &accessors,
                &mut textures,
                i,
                DMat4::IDENTITY,
                options,
                &mut mesh,
            )?;
        }
    }
    Ok(mesh)
}

fn append_mesh(
    document: &Value,
    accessors: &Accessors<'_>,
    textures: &mut Textures,
    number: usize,
    matrix: DMat4,
    options: &MeshOptions,
    mesh: &mut Mesh,
) -> ImportResult<()> {
    let value = document["meshes"]
        .get(number)
        .ok_or_else(|| invalid("glTF mesh index is out of range"))?;
    if value["weights"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_f64().unwrap_or(0.0) != 0.0))
    {
        return Err(ImportError::Unsupported(
            "active mesh morph targets must be baked before conversion".into(),
        ));
    }
    let primitives = value["primitives"]
        .as_array()
        .ok_or_else(|| invalid("glTF mesh has no primitives"))?;
    for primitive in primitives {
        mesh.report.input_features += 1;
        let mode = primitive["mode"].as_u64().unwrap_or(4);
        if ![4, 5, 6].contains(&mode) {
            mesh.report.warn(
                "Point and line primitives are omitted; only triangle surfaces can be voxelized.",
            );
            continue;
        }
        if primitive["extensions"]
            .get("KHR_draco_mesh_compression")
            .is_some()
        {
            return Err(ImportError::Unsupported(
                "Draco-compressed geometry must be decompressed before import".into(),
            ));
        }
        let positions = accessors.read(index(
            &primitive["attributes"]["POSITION"],
            "POSITION accessor",
        )?)?;
        if positions.kind != "VEC3" || positions.component != 5126 {
            return Err(ImportError::Unsupported(
                "POSITION must be a float VEC3; export an unquantized mesh".into(),
            ));
        }
        limit(
            "primitive vertices",
            positions.values.len(),
            options.limits.max_vertices,
        )?;
        let positions: Vec<_> = positions
            .values
            .iter()
            .map(|p| matrix.transform_point3(DVec3::new(p[0], p[1], p[2])))
            .collect();
        if positions
            .iter()
            .any(|p| !p.is_finite() || p.abs().max_element() > 1e15)
        {
            return Err(invalid(
                "transformed glTF position is not finite or is too large",
            ));
        }
        let colors = if primitive["attributes"]["COLOR_0"].is_null() {
            None
        } else {
            let colors = accessors.read(index(
                &primitive["attributes"]["COLOR_0"],
                "COLOR_0 accessor",
            )?)?;
            if !["VEC3", "VEC4"].contains(&colors.kind.as_str())
                || colors.values.len() != positions.len()
                || ![5121, 5123, 5126].contains(&colors.component)
                || (colors.component != 5126 && !colors.normalized)
            {
                return Err(invalid("glTF vertex colors do not match the positions"));
            }
            Some(colors.values)
        };
        let material = if primitive["material"].is_null() {
            &Value::Null
        } else {
            document["materials"]
                .get(index(&primitive["material"], "material")?)
                .ok_or_else(|| invalid("glTF material index is out of range"))?
        };
        let base = vector(
            &material["pbrMetallicRoughness"]["baseColorFactor"],
            [1.0; 4],
        )?;
        let texture = textures.binding(
            document,
            accessors,
            &material["pbrMetallicRoughness"]["baseColorTexture"],
            &options.limits,
            &mut mesh.report,
        )?;
        let uvs = if let Some((_, set)) = &texture {
            let key = format!("TEXCOORD_{set}");
            let data = accessors.read(index(
                &primitive["attributes"][&key],
                "texture coordinate accessor",
            )?)?;
            if data.kind != "VEC2"
                || data.values.len() != positions.len()
                || ![5121, 5123, 5126].contains(&data.component)
                || (data.component != 5126 && !data.normalized)
            {
                return Err(invalid("glTF texture coordinates must match positions and use float or normalized unsigned VEC2 data"));
            }
            Some(data.values)
        } else {
            None
        };
        if material["alphaMode"]
            .as_str()
            .is_some_and(|v| v != "OPAQUE")
            || base[3] < 1.0
            || !material["extensions"].is_null()
        {
            mesh.report.warn("Transparency, coating and lighting effects become solid brick materials; geometric surfaces are retained.");
        }
        let indices = if primitive["indices"].is_null() {
            (0..positions.len()).collect::<Vec<_>>()
        } else {
            let data = accessors.read(index(&primitive["indices"], "index accessor")?)?;
            if data.kind != "SCALAR"
                || ![5121, 5123, 5125].contains(&data.component)
                || data.normalized
            {
                return Err(invalid("glTF indices must be unsigned SCALAR values"));
            }
            data.values.iter().map(|v| v[0] as usize).collect()
        };
        if indices.iter().any(|i| *i >= positions.len()) {
            return Err(invalid("glTF primitive references a missing vertex"));
        }
        if mode == 4 && indices.len() % 3 != 0 {
            return Err(invalid("glTF triangle index count is not divisible by 3"));
        }
        let count = if mode == 4 {
            indices.len() / 3
        } else {
            indices.len().saturating_sub(2)
        };
        limit(
            "triangles",
            mesh.triangles.len().saturating_add(count),
            options.limits.max_triangles,
        )?;
        for i in 0..count {
            let ids = match mode {
                4 => [indices[i * 3], indices[i * 3 + 1], indices[i * 3 + 2]],
                5 => {
                    if i % 2 == 0 {
                        [indices[i], indices[i + 1], indices[i + 2]]
                    } else {
                        [indices[i + 1], indices[i], indices[i + 2]]
                    }
                }
                _ => [indices[0], indices[i + 1], indices[i + 2]],
            };
            if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
                continue;
            }
            mesh.triangles.push(Triangle {
                positions: ids.map(|id| positions[id]),
                colors: ids.map(|id| {
                    let vertex = colors.as_ref().map_or([1.0; 4], |c| c[id]);
                    [0, 1, 2].map(|j| (vertex[j] * base[j]).clamp(0.0, 1.0))
                }),
                linear_colors: true,
                texture: texture.as_ref().map(|(image, _)| image.clone()),
                uvs: ids.map(|id| uvs.as_ref().map_or([0.0; 2], |v| [v[id][0], v[id][1]])),
            });
        }
        mesh.report.imported_features += 1;
    }
    Ok(())
}
