//! Offline converter for testing a real source through the same worker dispatcher.
//! cargo run -p briccraft-import-worker --example convert -- glb model.glb output.json
use std::{env, fs, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if !(4..=5).contains(&args.len()) {
        return Err(
            "Usage: convert <osm|obj|glb|gltf> <input> <output.json> [options.json]".into(),
        );
    }
    if fs::metadata(&args[2])?.len() > 32 * 1024 * 1024 {
        return Err("Input exceeds the 32 MiB default budget".into());
    }
    let bytes = fs::read(&args[2])?;
    let options = args
        .get(4)
        .map(fs::read_to_string)
        .transpose()?
        .unwrap_or_default();
    let started = Instant::now();
    let mut stage = None;
    let plan = briccraft_import_worker::convert(&args[1], &bytes, &options, &mut |p| {
        if stage != Some(p.stage) {
            eprintln!("{:?}: {} / {}", p.stage, p.completed, p.total);
            stage = Some(p.stage);
        }
        true
    })?;
    fs::write(&args[3], serde_json::to_vec(&plan)?)?;
    eprintln!(
        "{} parts, {} voxels, {} triangles, {:.3}s\n{}",
        plan.report.parts,
        plan.report.voxels,
        plan.report.source_triangles,
        started.elapsed().as_secs_f64(),
        serde_json::to_string_pretty(&plan.report)?
    );
    Ok(())
}
