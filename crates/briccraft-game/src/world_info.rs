use briccraft_stream::BiomePreset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameAttribution {
    pub label: String,
    pub url: String,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInfo {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub preset: Option<BiomePreset>,
    #[serde(default)]
    pub attributions: Vec<GameAttribution>,
    #[serde(default)]
    pub import_report: Option<String>,
}

impl WorldInfo {
    pub fn classic(seed: u64) -> Self {
        Self {
            id: format!("classic-{seed}"),
            name: "Sunlit Archipelago".into(),
            preset: None,
            attributions: Vec::new(),
            import_report: None,
        }
    }
    pub fn streamed(seed: u64, preset: BiomePreset) -> Self {
        Self {
            id: format!("{:?}-{seed}", preset).to_ascii_lowercase(),
            name: preset.name().into(),
            preset: Some(preset),
            attributions: Vec::new(),
            import_report: None,
        }
    }
}
