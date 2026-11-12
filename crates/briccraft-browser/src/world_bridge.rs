//! Validated entry points for saved-world metadata and completed background imports.
use crate::bridge::with_runtime;
use briccraft_game::{GameAttribution, Session};
use briccraft_stream::{BiomePreset, GlobalPlacement, GlobalPos};
use briccraft_ui::WorldSummary;
use ferroui_view::prelude::*;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct LibraryEntry {
    id: String,
    name: String,
    detail: String,
    updated: String,
    #[serde(default)]
    archived: bool,
}

/// Acquire the host's file-operation lock without reentering the borrowed game session.
#[wasm_bindgen(js_name=briccraftWorldBusy)]
pub fn world_busy() {
    with_runtime(|r| r.state.world_busy.set(true));
}

#[wasm_bindgen(js_name=briccraftWorldReady)]
pub fn world_ready() {
    with_runtime(|r| r.state.world_busy.set(false));
}

#[wasm_bindgen(js_name=briccraftWorldList)]
pub fn world_list(source: &str) {
    if source.len() > 2 * 1024 * 1024 {
        return;
    }
    let Ok(entries) = serde_json::from_str::<Vec<LibraryEntry>>(source) else {
        return;
    };
    let entries = entries
        .into_iter()
        .take(1000)
        .map(|e| WorldSummary {
            id: e.id,
            name: e.name,
            detail: e.detail,
            updated: e.updated,
            archived: e.archived,
        })
        .collect::<Vec<_>>();
    with_runtime(|r| r.state.worlds.set(entries));
}

#[wasm_bindgen(js_name=briccraftWorldIdentity)]
pub fn identity(id: &str, name: &str) {
    if id.is_empty() || id.len() > 128 || name.len() > 320 {
        return;
    }
    with_runtime(|r| {
        {
            let mut game = r.session.borrow_mut();
            game.world_info.id = id.into();
            game.world_info.name = name.into();
        }
        r.sync_hud();
    });
}

#[wasm_bindgen(js_name=briccraftSerializeWorld)]
pub fn serialize() -> String {
    let mut source = String::new();
    with_runtime(|r| {
        let result = r.session.borrow_mut().to_json();
        match result {
            Ok(value) => source = value,
            Err(error) => r.session.borrow_mut().status = error.to_string(),
        }
    });
    source
}

#[wasm_bindgen(js_name=briccraftCreateWorld)]
pub fn create(preset: &str, name: &str, seed: &str, id: &str) -> bool {
    let Ok(seed) = seed.parse::<u64>() else {
        return false;
    };
    if id.len() > 128 || name.len() > 320 {
        return false;
    }
    let mut success = false;
    with_runtime(|r| {
        let result = match preset {
            "island" => Ok(Session::new(seed)),
            "meadow" => Session::new_streamed(seed, BiomePreset::Meadow),
            "alpine" => Session::new_streamed(seed, BiomePreset::Alpine),
            "desert" => Session::new_streamed(seed, BiomePreset::Desert),
            "coast" => Session::new_streamed(seed, BiomePreset::Coast),
            "flat" => Session::new_streamed(seed, BiomePreset::Flat),
            _ => return,
        };
        match result {
            Ok(mut game) => {
                game.world_info.id = id.into();
                if !name.trim().is_empty() {
                    game.world_info.name = name.trim().chars().take(80).collect();
                }
                *r.session.borrow_mut() = game;
                r.map_world_changed();
                r.renderer.borrow_mut().invalidate();
                r.state.close_panels();
                success = true;
            }
            Err(error) => r.session.borrow_mut().status = error.to_string(),
        }
        r.sync_hud();
    });
    success
}

#[wasm_bindgen(js_name=briccraftImportProgress)]
pub fn progress(status: &str, value: f64, busy: bool) {
    with_runtime(|r| {
        r.session.borrow_mut().status = status.into();
        batch(|| {
            r.state.import_busy.set(busy);
            r.state.import_status.set(status.into());
            r.state.import_progress.set(if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            });
        });
        r.sync_hud();
    });
}

#[wasm_bindgen(js_name=briccraftImportPlan)]
pub fn import_plan(source: &str, name: &str, append: bool) -> bool {
    if source.len() > 64 * 1024 * 1024 {
        progress("Converted model exceeds the 64 MB plan limit.", 0.0, false);
        return false;
    }
    let mut success = false;
    with_runtime(|r| {
        let mut game = r.session.borrow_mut();
        let result = (|| -> Result<(), String> {
            let plan: briccraft_import::ImportPlan =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            if plan.placements.len() > 150_000 {
                return Err("Import exceeds the 150,000-part transaction budget.".into());
            }
            if append && *r.import_world_id.borrow() != game.world_info.id {
                return Err("The target world changed. Select a construction target in this world and import again.".into());
            }
            let anchor = if append {
                Some(
                    r.import_anchor
                        .get()
                        .ok_or("Select a construction target before adding a model.")?,
                )
            } else {
                None
            };
            let mut source_report =
                serde_json::to_value(&plan.report).map_err(|e| e.to_string())?;
            let report = if let Some(anchor) = anchor {
                source_report["placement_origin"] = serde_json::json!({"x":anchor.x.to_string(),"y":anchor.y,"z":anchor.z.to_string()});
                let mut primary = game
                    .world_info
                    .import_report
                    .as_deref()
                    .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                    .filter(|value| value.is_object())
                    .unwrap_or_else(|| serde_json::json!({}));
                if !primary
                    .get("additional_imports")
                    .is_some_and(|v| v.is_array())
                {
                    primary["additional_imports"] = serde_json::json!([]);
                }
                primary["additional_imports"]
                    .as_array_mut()
                    .expect("array initialized")
                    .push(source_report);
                primary.to_string()
            } else {
                source_report.to_string()
            };
            if report.len() > 1_048_576 {
                return Err("The combined source report exceeds 1 MiB. Import this model into a separate world.".into());
            }
            let credit = plan
                .attribution
                .into_iter()
                .map(|a| GameAttribution {
                    label: a.text,
                    url: a.url,
                    license: a.license,
                })
                .collect();
            let placements = plan
                .placements
                .into_iter()
                .map(|p| -> Result<GlobalPlacement, String> {
                    let position = if let Some(anchor) = anchor {
                        GlobalPos::new(
                            anchor
                                .x
                                .checked_add(i64::from(p.position[0]) - 48)
                                .ok_or("Global X coordinate overflow")?,
                            p.position[1]
                                .checked_sub(18)
                                .and_then(|y| y.checked_add(anchor.y))
                                .ok_or("Vertical coordinate overflow")?,
                            anchor
                                .z
                                .checked_add(i64::from(p.position[2]) - 48)
                                .ok_or("Global Z coordinate overflow")?,
                        )
                    } else {
                        GlobalPos::new(
                            i64::from(p.position[0]),
                            p.position[1],
                            i64::from(p.position[2]),
                        )
                    };
                    Ok(GlobalPlacement::new(
                        p.part_id, p.color, position, p.rotation,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let label = if append {
                game.world_info.name.clone()
            } else {
                name.into()
            };
            game.begin_import(placements, label, credit)
                .map_err(|e| e.to_string())?;
            game.world_info.import_report = Some(report);
            r.import_anchor.set(None);
            Ok(())
        })();
        match result {
            Ok(()) => success = true,
            Err(error) => game.status = format!("Import failed: {error}"),
        }
        drop(game);
        r.sync_hud();
    });
    success
}
