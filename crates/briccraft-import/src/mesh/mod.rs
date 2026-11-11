//! Static triangle geometry import; source scene transforms and vertex colors
//! are applied before conservative surface voxelization and exact brick packing.
mod accessor;
mod gltf;
mod obj;
mod texture;
mod texture_decode;
mod triangulate;
mod voxelize;

use crate::{
    error::invalid, limits::progress, packing, voxels::Voxels, ImportError, ImportLimits,
    ImportPlan, ImportProgress, ImportReport, ImportResult, ImportStage, SourceKind,
};
use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshOptions {
    pub origin: [i32; 3],
    /// Uniform fit of the longest dimension. Y uses 0.4-stud plate units.
    pub fit_size_studs: f64,
    /// When provided, preserve this physical source-unit scale instead of fitting.
    pub units_per_stud: Option<f64>,
    pub color: u8,
    pub fill_interior: bool,
    /// A fitted model scales uniformly to this height. Physical-scale imports error if too tall.
    pub max_height_plates: Option<u32>,
    pub limits: ImportLimits,
}
impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            origin: [0, 0, 0],
            fit_size_studs: 48.0,
            units_per_stud: None,
            color: 16,
            fill_interior: false,
            max_height_plates: None,
            limits: ImportLimits::default(),
        }
    }
}
#[derive(Clone)]
struct Triangle {
    pub positions: [DVec3; 3],
    pub colors: [[f64; 3]; 3],
    pub linear_colors: bool,
    pub texture: Option<std::rc::Rc<texture::Texture>>,
    pub uvs: [[f64; 2]; 3],
}
struct Mesh {
    pub triangles: Vec<Triangle>,
    pub report: ImportReport,
}

pub fn import_obj(source: &str, options: &MeshOptions) -> ImportResult<ImportPlan> {
    import_obj_with_progress(source, None, options, &mut |_| true)
}
pub fn import_obj_with_mtl(
    source: &str,
    mtl: &str,
    options: &MeshOptions,
) -> ImportResult<ImportPlan> {
    import_obj_with_progress(source, Some(mtl), options, &mut |_| true)
}
pub fn import_obj_with_progress(
    source: &str,
    mtl: Option<&str>,
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    validate(options, source.len() + mtl.map_or(0, str::len))?;
    progress(callback, ImportStage::Parse, 0, source.len())?;
    convert(
        obj::parse(source, mtl, options, callback)?,
        options,
        callback,
    )
}
pub fn import_glb(source: &[u8], options: &MeshOptions) -> ImportResult<ImportPlan> {
    import_glb_with_progress(source, options, &mut |_| true)
}
pub fn import_glb_with_progress(
    source: &[u8],
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    validate(options, source.len())?;
    progress(callback, ImportStage::Parse, 0, source.len())?;
    let (json, bin) = gltf::container(source)?;
    convert(
        gltf::parse(json, bin, SourceKind::Glb, options, callback)?,
        options,
        callback,
    )
}
pub fn import_gltf_json(source: &str, options: &MeshOptions) -> ImportResult<ImportPlan> {
    import_gltf_json_with_progress(source, options, &mut |_| true)
}
pub fn import_gltf_json_with_progress(
    source: &str,
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    validate(options, source.len())?;
    progress(callback, ImportStage::Parse, 0, source.len())?;
    convert(
        gltf::parse(source.as_bytes(), None, SourceKind::Gltf, options, callback)?,
        options,
        callback,
    )
}
fn validate(options: &MeshOptions, bytes: usize) -> ImportResult<()> {
    options.limits.input(bytes)?;
    if !options.fit_size_studs.is_finite() || !(1.0..=4096.0).contains(&options.fit_size_studs) {
        return Err(invalid("model fit size must be between 1 and 4096 studs"));
    }
    if options
        .units_per_stud
        .is_some_and(|v| !v.is_finite() || v <= 0.0)
    {
        return Err(invalid("source units per stud must be finite and positive"));
    }
    if options.color as usize >= briccraft_parts::PALETTE.len() {
        return Err(invalid("unknown default model color"));
    }
    if options
        .origin
        .iter()
        .any(|v| v.unsigned_abs() > 1_000_000_000)
    {
        return Err(invalid("model origin is too large"));
    }
    if options.max_height_plates == Some(0) {
        return Err(invalid("model height budget must be positive"));
    }
    Ok(())
}
fn convert(
    mut mesh: Mesh,
    options: &MeshOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    if mesh.triangles.is_empty() {
        return Err(ImportError::EmptyGeometry);
    }
    mesh.report.source_triangles = mesh.triangles.len();
    let mut voxels = Voxels::new(&options.limits);
    voxelize::render(
        &mesh.triangles,
        options,
        &mut voxels,
        &mut mesh.report,
        callback,
    )?;
    if voxels.cells.is_empty() {
        return Err(ImportError::EmptyGeometry);
    }
    mesh.report.voxels = voxels.cells.len();
    mesh.report.bounds = voxels.bounds();
    let placements = packing::pack(voxels, callback)?;
    mesh.report.parts = placements.len();
    mesh.report.warn("Static mesh surfaces are sampled onto the stud/plate grid. Fine details below that resolution cannot be preserved; colors are quantized to the brick palette.");
    Ok(ImportPlan {
        placements,
        report: mesh.report,
        attribution: Vec::new(),
    })
}
