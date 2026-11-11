//! Public automation commands route through the same native UI actions.
use crate::bridge::with_runtime;
use briccraft_ui::UiAction;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=briccraftCommand)]
pub fn command(name: &str, value: &str) {
    with_runtime(|r| {
        let action = match name {
            "part" => value.parse().ok().map(UiAction::SelectPart),
            "color" => value.parse().ok().map(UiAction::SelectColor),
            "blueprint" => Some(UiAction::SpawnBlueprint(value.into())),
            "blueprints" => {
                r.state.close_panels();
                r.state.blueprint_section.set(0);
                r.state.blueprints_open.set(value != "false");
                None
            }
            "designs" => {
                if value == "false" {
                    r.state.close_panels();
                } else {
                    r.show_designs();
                }
                None
            }
            "design-capture" => Some(UiAction::CaptureDesign(value.into())),
            "design-place" => value.parse().ok().map(UiAction::PlaceDesign),
            "design-delete" => value.parse().ok().map(UiAction::DeleteDesign),
            "design-export" => value.parse().ok().map(UiAction::ExportDesign),
            "design-import" => Some(UiAction::ImportDesign),
            "design-rename" => serde_json::from_str::<serde_json::Value>(value)
                .ok()
                .and_then(|v| {
                    Some(UiAction::RenameDesign {
                        id: u32::try_from(v.get("id")?.as_u64()?).ok()?,
                        name: v.get("name")?.as_str()?.to_owned(),
                    })
                }),
            "drive" => Some(UiAction::ActivateVehicle(value.into())),
            "park" => Some(UiAction::ExitVehicle),
            "rotate" => Some(UiAction::Rotate),
            "place" => Some(UiAction::Place),
            "remove" => Some(UiAction::Remove),
            "undo" => Some(UiAction::Undo),
            "redo" => Some(UiAction::Redo),
            "save" => Some(UiAction::Save),
            "load" => Some(UiAction::Load),
            "camera" => Some(UiAction::ToggleExplore),
            "home" => Some(UiAction::ResetCamera),
            "quality" => value.parse().ok().map(UiAction::SetQuality),
            "resolution" => match value {
                "auto" => Some(UiAction::SetResolutionMode(0)),
                "quality" => Some(UiAction::SetResolutionMode(1)),
                "performance" => Some(UiAction::SetResolutionMode(2)),
                _ => value
                    .parse::<u8>()
                    .ok()
                    .filter(|v| *v <= 2)
                    .map(UiAction::SetResolutionMode),
            },
            "antialiasing" => value
                .parse::<u8>()
                .ok()
                .filter(|v| matches!(*v, 1 | 2 | 4))
                .map(UiAction::SetAntialiasing),
            "paused" => match value {
                "true" | "on" => Some(UiAction::SetPaused(true)),
                "false" | "off" => Some(UiAction::SetPaused(false)),
                _ => None,
            },
            "settings" => {
                r.state.close_panels();
                r.state.settings_open.set(value != "false");
                None
            }
            "daylight" => value.parse().ok().map(UiAction::SetDaylight),
            "catalog" => {
                r.state.close_panels();
                r.state.catalog_open.set(value != "false");
                None
            }
            "studio" => {
                r.state.close_panels();
                r.state.imports_open.set(value != "false");
                None
            }
            "expedition" => match value {
                "on" | "true" => Some(UiAction::SetExpedition(true)),
                "off" | "false" => Some(UiAction::SetExpedition(false)),
                _ => None,
            },
            "gameplay" | "objectives" => {
                r.state.close_panels();
                r.state.gameplay_open.set(value != "false");
                None
            }
            "new" => value.parse().ok().map(UiAction::NewWorld),
            "worlds" => Some(UiAction::ShowWorlds),
            "landmark" => Some(UiAction::ImportLandmark),
            "terrain-sample" => Some(UiAction::ImportTerrainSample),
            "cancel-import" => Some(UiAction::CancelImport),
            "sample-model" => Some(UiAction::ImportModel {
                size: 48.0,
                sample: true,
            }),
            "create-world" => Some(UiAction::CreateWorld {
                preset: value.into(),
                name: format!("{value} world"),
                seed: 42,
            }),
            _ => None,
        };
        if let Some(action) = action {
            r.dispatch(action);
        }
    });
}
