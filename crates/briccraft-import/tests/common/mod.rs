#![allow(dead_code)]
use base64::Engine;
use briccraft_import::ImportPlan;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn embedded_buffer(bytes: &[u8]) -> Value {
    json!({"byteLength":bytes.len(),"uri":format!("data:application/octet-stream;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes))})
}

pub fn triangle_gltf() -> Value {
    let bytes = triangle_bytes();
    json!({"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"buffers":[embedded_buffer(&bytes)],
        "bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}]})
}

pub fn triangle_bytes() -> Vec<u8> {
    [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        .into_iter()
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect()
}

/// Verify the promised exact cover, using actual catalog dimensions.
pub fn cells(plan: &ImportPlan) -> HashMap<[i32; 3], u8> {
    let mut occupied = HashMap::new();
    for placement in &plan.placements {
        assert!(briccraft_parts::part(placement.part_id).is_some());
        assert!((placement.color as usize) < briccraft_parts::PALETTE.len());
        let [sx, sy, sz] = placement.size();
        for x in 0..sx as i32 {
            for y in 0..sy as i32 {
                for z in 0..sz as i32 {
                    let p = [
                        placement.position[0] + x,
                        placement.position[1] + y,
                        placement.position[2] + z,
                    ];
                    assert!(
                        occupied.insert(p, placement.color).is_none(),
                        "overlapping import parts at {p:?}"
                    );
                }
            }
        }
    }
    assert_eq!(occupied.len(), plan.report.voxels);
    assert_eq!(plan.placements.len(), plan.report.parts);
    occupied
}
