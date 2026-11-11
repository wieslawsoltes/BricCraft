use crate::GeoReference;
use briccraft_world::Placement;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridBounds {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    OpenStreetMap,
    Obj,
    Gltf,
    Glb,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribution {
    pub text: String,
    pub url: String,
    pub license: String,
    pub license_url: String,
}
impl Attribution {
    pub fn openstreetmap() -> Self {
        Self {
            text: "© OpenStreetMap contributors".into(),
            url: "https://www.openstreetmap.org/copyright".into(),
            license: "ODbL-1.0".into(),
            license_url: "https://opendatacommons.org/licenses/odbl/1-0/".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeightEvidence {
    pub osm_id: i64,
    pub osm_type: String,
    pub height_m: f64,
    pub source: String,
    pub inferred: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub source: SourceKind,
    pub source_name: String,
    pub source_timestamp: Option<String>,
    pub projection: Option<GeoReference>,
    pub input_features: usize,
    pub imported_features: usize,
    pub buildings: usize,
    pub roads: usize,
    pub water: usize,
    #[serde(default)]
    pub landcover: usize,
    #[serde(default)]
    pub terrain: Option<briccraft_terrain::TerrainReport>,
    pub inferred_heights: usize,
    pub inferred_widths: usize,
    pub height_evidence: Vec<HeightEvidence>,
    pub source_triangles: usize,
    pub source_bounds: Option<[[f64; 3]; 2]>,
    pub studs_per_source_unit: Option<f64>,
    pub voxels: usize,
    pub parts: usize,
    pub bounds: GridBounds,
    pub warnings: Vec<String>,
}
impl ImportReport {
    pub(crate) fn new(source: SourceKind) -> Self {
        Self {
            source,
            source_name: String::new(),
            source_timestamp: None,
            projection: None,
            input_features: 0,
            imported_features: 0,
            buildings: 0,
            roads: 0,
            water: 0,
            landcover: 0,
            terrain: None,
            inferred_heights: 0,
            inferred_widths: 0,
            height_evidence: Vec::new(),
            source_triangles: 0,
            source_bounds: None,
            studs_per_source_unit: None,
            voxels: 0,
            parts: 0,
            bounds: GridBounds::default(),
            warnings: Vec::new(),
        }
    }
    pub(crate) fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.warnings.len() < 64 && !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportPlan {
    pub placements: Vec<Placement>,
    pub report: ImportReport,
    pub attribution: Vec<Attribution>,
}
impl ImportPlan {
    /// Host streaming code can consume bounded batches without local-window clipping.
    pub fn batches(&self, size: usize) -> impl Iterator<Item = &[Placement]> {
        self.placements.chunks(size.max(1))
    }
}
