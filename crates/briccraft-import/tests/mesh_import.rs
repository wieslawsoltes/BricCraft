mod common;
use base64::Engine;
use briccraft_import::{import_glb, import_gltf_json, import_obj, mesh, ImportError, MeshOptions};
use serde_json::{json, Value};
use std::collections::HashSet;

fn options() -> MeshOptions {
    MeshOptions {
        fit_size_studs: 8.0,
        ..MeshOptions::default()
    }
}
fn triangle_gltf() -> Value {
    let mut bytes = Vec::new();
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for coordinate in p {
            bytes.extend(coordinate.to_le_bytes());
        }
    }
    json!({"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
        "buffers":[{"byteLength":bytes.len(),"uri":format!("data:application/octet-stream;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes))}],
        "bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}]})
}

#[test]
fn real_khronos_glb_preserves_surface_geometry_and_vertex_colors() {
    let plan = import_glb(include_bytes!("fixtures/BoxVertexColors.glb"), &options()).unwrap();
    assert_eq!(plan.report.source_triangles, 12);
    assert_eq!(
        plan.report.source_bounds,
        Some([[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]])
    );
    assert_eq!(plan.report.bounds.min, [-4, 0, -4]);
    assert_eq!(plan.report.bounds.max, [4, 20, 4]);
    let occupied = common::cells(&plan);
    let colors: HashSet<_> = occupied.values().collect();
    assert!(colors.len() >= 6);
    assert!(!occupied.contains_key(&[0, 10, 0]));
    assert!(occupied.contains_key(&[-4, 10, 0]));
}

#[test]
fn solid_fill_adds_only_the_enclosed_interior() {
    let mut options = options();
    options.fill_interior = true;
    let plan = import_glb(include_bytes!("fixtures/BoxVertexColors.glb"), &options).unwrap();
    let occupied = common::cells(&plan);
    assert!(occupied.contains_key(&[0, 10, 0]));
    assert_eq!(occupied.len(), 8 * 20 * 8);
}

#[test]
fn obj_concave_face_and_relative_indices_keep_the_open_quadrant() {
    let source = "v 0 0 0\nv 4 0 0\nv 4 0 1\nv 1 0 1\nv 1 0 4\nv 0 0 4\nf -6 -5 -4 -3 -2 -1\n";
    let plan = import_obj(source, &options()).unwrap();
    assert_eq!(plan.report.source_triangles, 4);
    let occupied = common::cells(&plan);
    assert!(occupied.contains_key(&[-3, 0, 2]));
    assert!(!occupied.contains_key(&[2, 0, 2]));
}

#[test]
fn obj_materials_and_vertex_colors_are_used_without_network_fetches() {
    let source = "mtllib test.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl red\nf 1/1/1 2/2/1 3/3/1\n";
    let plan =
        mesh::import_obj_with_mtl(source, "newmtl red\nKd 0.77 0.16 0.11\n", &options()).unwrap();
    assert!(plan.placements.iter().all(|p| p.color == 0));
    let absent = import_obj(source, &options()).unwrap();
    assert!(absent
        .report
        .warnings
        .iter()
        .any(|w| w.contains("not fetched")));
    for invalid in [
        "v NaN 0 0\n",
        "v 0 0 0\nf 0 1 1\n",
        "v 0 0 0\nf -2 1 1\n",
        "v 1e15 0 0 1e-19\n",
    ] {
        assert!(
            import_obj(invalid, &options()).is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn gltf_scene_hierarchy_applies_column_major_trs_before_uniform_fit() {
    let mut document = triangle_gltf();
    document["nodes"] = json!([{"translation":[10,2,3],"children":[1]},{"scale":[2,3,4],"mesh":0}]);
    let plan = import_gltf_json(&document.to_string(), &options()).unwrap();
    assert_eq!(
        plan.report.source_bounds,
        Some([[10.0, 2.0, 3.0], [12.0, 5.0, 3.0]])
    );
    assert_eq!(plan.report.studs_per_source_unit, Some(8.0 / 3.0));
    assert_eq!(plan.report.source_triangles, 1);
    common::cells(&plan);
}

#[test]
fn a_vertical_budget_uniformly_scales_all_axes_and_physical_mode_rejects_overflow() {
    let mut options = options();
    options.origin = [1000, 18, -1000];
    options.max_height_plates = Some(10);
    let plan = import_glb(include_bytes!("fixtures/BoxVertexColors.glb"), &options).unwrap();
    assert_eq!(plan.report.studs_per_source_unit, Some(4.0));
    assert_eq!(plan.report.bounds.max[1], 28);
    assert_eq!(plan.report.bounds.max[0] - plan.report.bounds.min[0], 4);
    options.units_per_stud = Some(0.1);
    assert!(import_glb(include_bytes!("fixtures/BoxVertexColors.glb"), &options).is_err());
}

#[test]
fn accessor_bounds_external_buffers_and_required_compression_are_validated() {
    let mut document = triangle_gltf();
    document["accessors"][0]["byteOffset"] = json!(32);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
    document = triangle_gltf();
    document["buffers"][0]["uri"] = json!("https://example.invalid/model.bin");
    assert!(matches!(
        import_gltf_json(&document.to_string(), &options()),
        Err(ImportError::Unsupported(_))
    ));
    document = triangle_gltf();
    document["extensionsRequired"] = json!(["KHR_draco_mesh_compression"]);
    assert!(matches!(
        import_gltf_json(&document.to_string(), &options()),
        Err(ImportError::Unsupported(_))
    ));
    document = triangle_gltf();
    document["accessors"][0]["componentType"] = json!(4_294_972_422u64);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
    document = triangle_gltf();
    document["nodes"][0]["children"] = json!([0]);
    assert!(import_gltf_json(&document.to_string(), &options()).is_err());
}

#[test]
fn fitted_plate_boundaries_do_not_gain_an_extra_layer_from_float_rounding() {
    for height in [7, 19, 53, 54] {
        let options = MeshOptions {
            origin: [48, 18, 48],
            fit_size_studs: 48.0,
            max_height_plates: Some(height),
            ..MeshOptions::default()
        };
        let plan = import_glb(include_bytes!("fixtures/BoxVertexColors.glb"), &options).unwrap();
        assert_eq!(plan.report.bounds.max[1], 18 + height as i32);
    }
}

#[test]
fn malformed_glb_and_budget_exhaustion_produce_errors_without_partial_plans() {
    let source = include_bytes!("fixtures/BoxVertexColors.glb");
    for length in [0, 4, 12, 17, source.len() - 1] {
        assert!(import_glb(&source[..length], &options()).is_err());
    }
    let mut broken = source.to_vec();
    broken[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(import_glb(&broken, &options()).is_err());
    let mut options = options();
    options.limits.max_voxels = 10;
    assert!(matches!(
        import_glb(source, &options),
        Err(ImportError::BudgetExceeded { .. })
    ));
    options = MeshOptions::default();
    options.limits.max_parts = 1;
    assert!(matches!(
        import_glb(source, &options),
        Err(ImportError::BudgetExceeded { .. })
    ));
    assert!(matches!(
        mesh::import_glb_with_progress(source, &options, &mut |_| false),
        Err(ImportError::Cancelled)
    ));
}
