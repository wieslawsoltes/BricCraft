//! Renderer-independent projection of the world's reusable constructions.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedDesignCard {
    pub id: u32,
    pub name: String,
    pub parts: usize,
    /// Width in studs, height in plates, depth in studs.
    pub size: [u8; 3],
    pub studs: u64,
    pub affordable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesignLibraryView {
    pub designs: Vec<SavedDesignCard>,
    pub capture_ready: bool,
    pub capture_detail: String,
    pub capture_parts: usize,
}

impl Default for DesignLibraryView {
    fn default() -> Self {
        Self {
            designs: Vec::new(),
            capture_ready: false,
            capture_detail: "Point at a separate construction to save its design.".into(),
            capture_parts: 0,
        }
    }
}
