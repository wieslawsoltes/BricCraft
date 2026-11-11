//! Exports the real generated scene to the native-Skia shader smoke renderer.
//! `cargo run -p briccraft-render-data --example export_scene -- /tmp/briccraft-scene.json`
use briccraft_parts::{PALETTE, part};
use briccraft_world::{DEFAULT_SEED, World};
use std::fmt::Write;

fn main() -> std::io::Result<()> {
    let world = World::generate(DEFAULT_SEED);
    let mut json = format!(
        "{{\"dimensions\":{:?},\"position\":[56,21,86],\"target\":[45,6,42],\"pieces\":[",
        world.dimensions()
    );
    for (index, instance) in world.instances().enumerate() {
        if index > 0 {
            json.push(',');
        }
        let descriptor = part(instance.part_id).expect("validated world part");
        write!(
            &mut json,
            "[{:?},{:?},{:?},{},{}]",
            instance.position,
            descriptor.rotated_size(instance.rotation),
            PALETTE[usize::from(instance.color)],
            descriptor.shape as u8,
            instance.rotation
        )
        .expect("write to string");
    }
    json.push_str("]}");
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/briccraft-scene.json".into());
    std::fs::write(&destination, json)?;
    println!("Exported {} parts to {destination}", world.len());
    Ok(())
}
