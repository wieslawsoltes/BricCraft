mod common;
use base64::Engine;
use briccraft_import::{import_gltf_json, MeshOptions};
use serde_json::json;

fn options() -> MeshOptions {
    MeshOptions {
        fit_size_studs: 4.0,
        ..MeshOptions::default()
    }
}

#[test]
fn interleaved_accessors_read_declared_stride() {
    let mut document = common::triangle_gltf();
    let mut bytes = Vec::new();
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for value in p.into_iter().chain([99.0]) {
            bytes.extend(value.to_le_bytes());
        }
    }
    document["buffers"][0] = common::embedded_buffer(&bytes);
    document["bufferViews"][0] = json!({"buffer":0,"byteLength":48,"byteStride":16});
    let plan = import_gltf_json(&document.to_string(), &options()).unwrap();
    assert_eq!(
        plan.report.source_bounds,
        Some([[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]])
    );
    document["bufferViews"][0]["byteStride"] = json!(14);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
}

#[test]
fn sparse_accessors_apply_values_and_reject_out_of_range_offsets() {
    let mut document = common::triangle_gltf();
    let mut bytes = vec![0, 1, 2, 0];
    bytes.extend(common::triangle_bytes());
    document["buffers"][0] = common::embedded_buffer(&bytes);
    document["bufferViews"] =
        json!([{"buffer":0,"byteLength":3},{"buffer":0,"byteOffset":4,"byteLength":36}]);
    document["accessors"] = json!([{"componentType":5126,"count":3,"type":"VEC3","sparse":{"count":3,"indices":{"bufferView":0,"componentType":5121},"values":{"bufferView":1}}}]);
    let plan = import_gltf_json(&document.to_string(), &options()).unwrap();
    assert_eq!(
        plan.report.source_bounds,
        Some([[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]])
    );
    document["accessors"][0]["sparse"]["values"]["byteOffset"] = json!(u64::MAX);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
    document["accessors"][0]["sparse"]["values"]["byteOffset"] = json!(0);
    bytes[1] = 0;
    document["buffers"][0] = common::embedded_buffer(&bytes);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
}

#[test]
fn normalized_triangle_indices_are_not_reinterpreted_as_vertices() {
    let mut document = common::triangle_gltf();
    let mut bytes = common::triangle_bytes();
    bytes.extend([0, 1, 2]);
    document["buffers"][0] = common::embedded_buffer(&bytes);
    document["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":36,"byteLength":3}));
    document["accessors"].as_array_mut().unwrap().push(
        json!({"bufferView":1,"componentType":5121,"normalized":true,"count":3,"type":"SCALAR"}),
    );
    document["meshes"][0]["primitives"][0]["indices"] = json!(1);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
}

#[test]
fn an_embedded_png_base_color_becomes_brick_color() {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[196, 40, 28])
            .unwrap();
    }
    let mut document = common::triangle_gltf();
    let mut bytes = common::triangle_bytes();
    for uv in [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]] {
        for value in uv {
            bytes.extend(value.to_le_bytes());
        }
    }
    document["buffers"][0] = common::embedded_buffer(&bytes);
    document["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":36,"byteLength":24}));
    document["accessors"]
        .as_array_mut()
        .unwrap()
        .push(json!({"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}));
    document["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_0"] = json!(1);
    document["meshes"][0]["primitives"][0]["material"] = json!(0);
    document["images"] = json!([{"uri":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(png))}]);
    document["textures"] = json!([{"source":0}]);
    document["materials"] = json!([{"pbrMetallicRoughness":{"baseColorTexture":{"index":0,"extensions":{"KHR_texture_transform":{"offset":[0.1,0.2],"rotation":0.5,"scale":[2,3]}}}}}]);
    let plan = import_gltf_json(&document.to_string(), &options()).unwrap();
    assert!(plan.placements.iter().all(|p| p.color == 0));
    document["images"][0]["uri"] = json!("https://example.invalid/texture.png");
    let external = import_gltf_json(&document.to_string(), &options()).unwrap();
    assert!(external
        .report
        .warnings
        .iter()
        .any(|w| w.contains("External texture")));
}
