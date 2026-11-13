//! Validated map imagery and asynchronous geographic-region publication.
use crate::bridge::with_runtime;
use briccraft_game::{GameAttribution, Session};
use briccraft_stream::{GlobalPlacement, GlobalPos, MapAnchor, MapRegion};
use briccraft_ui::MapTile;
use serde_json::json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=briccraftMapTile)]
pub fn map_tile(request: u32, zoom: u8, x: u32, y: u32, bytes: &[u8]) -> bool {
    let mut accepted = false;
    with_runtime(|r| {
        if r.map_tile_request.get() != request
            || !r.state.places_open.get_untracked()
            || !r.map_requested_tiles.borrow().contains(&(zoom, x, y))
        {
            return;
        }
        match crate::map_image::decode(bytes) {
            Ok(image) => {
                let mut tiles = r.state.map_tiles.get_untracked();
                tiles.retain(|tile| (tile.zoom, tile.x, tile.y) != (zoom, x, y));
                tiles.push(MapTile { zoom, x, y, image });
                r.state.map_tiles.set(tiles);
                accepted = true;
            }
            Err(error) => r.state.map_status.set(format!("Map tile: {error}")),
        }
    });
    accepted
}

#[wasm_bindgen(js_name=briccraftMapTileStatus)]
pub fn tile_status(request: u32, status: &str) {
    with_runtime(|r| {
        if r.map_tile_request.get() == request && r.state.places_open.get_untracked() {
            r.state.map_status.set(status.chars().take(320).collect());
        }
    });
}

/// A small host snapshot avoids serializing renderer, catalogue and design state
/// on every streaming poll. Generation tokens fence same-ID reloads as well.
#[wasm_bindgen(js_name=briccraftMapState)]
pub fn state() -> String {
    let mut source = "{}".to_owned();
    with_runtime(|r| {
        let game = r.session.borrow();
        let point = game.global_camera_position();
        let map_rect = r
            .state
            .places_open
            .get_untracked()
            .then(|| {
                let view = r.viewport.get_untracked()?;
                let top = ferroui_controls::TopLevel::get_top_level(Some(&view))?;
                let map = top.get_visual_descendants().find(|view| {
                    view.name().as_deref() == Some("PlaceMapViewport")
                        && view.is_effectively_visible()
                })?;
                let point = map.translate_point(ferroui_base::Point::new(0.0, 0.0), &top)?;
                let bounds = map.bounds();
                Some([point.x, point.y, bounds.width, bounds.height])
            })
            .flatten();
        let picker = r.state.map_view.get_untracked();
        let player = r.state.map_player.get_untracked().map(|player| json!({
            "latitude":player.latitude,"longitude":player.longitude,
            "bearing":player.bearing_degrees,"distance":player.distance_from_start_m,
        }));
        let picker = json!({
            "open":r.state.places_open.get_untracked(),
            "latitude":picker.latitude,"longitude":picker.longitude,"zoom":picker.zoom,
            "selected":[picker.selected_latitude,picker.selected_longitude],
            "radius":picker.radius,"meters_per_stud":picker.meters_per_stud,"terrain":picker.terrain,
            "size":briccraft_ui::map_view_size(r.state.layout.get_untracked()),
            "tiles":r.state.map_tiles.get_untracked().len(),"status":r.state.map_status.get_untracked(),
            "rect":map_rect,
            "player":player,
            "area_status":r.state.import_status.get_untracked(),
        });
        source = json!({
            "epoch":r.map_epoch.get(),"world_id":game.world_info.id,
            "anchor":game.map_anchor(),"regions":game.mapped_regions(),
            "demands":game.map_demands().unwrap_or_default(),"camera":point,
            "waiting":game.map_waiting(),"source_parts":game.mapped_source_parts(),
            "world_busy":r.state.world_busy.get_untracked(),
            "import_busy":r.state.import_busy.get_untracked(),
            "active":r.state.roam_active.get_untracked(),
            "status":r.state.roam_status.get_untracked(),
            "picker":picker,
        })
        .to_string();
    });
    source
}

#[wasm_bindgen(js_name=briccraftMapProgress)]
pub fn progress(epoch: u32, world_id: &str, status: &str, active: bool) {
    with_runtime(|r| {
        if r.map_epoch.get() == epoch && r.session.borrow().world_info.id == world_id {
            r.state.roam_active.set(active);
            r.state.roam_status.set(status.chars().take(640).collect());
        }
    });
}

/// Hold only the short save-and-swap phase. Fetching and conversion never lock
/// the game. A matching unlock cannot release another world operation's lock.
#[wasm_bindgen(js_name=briccraftMapLock)]
pub fn lock(epoch: u32, world_id: &str) -> bool {
    let mut acquired = false;
    with_runtime(|r| {
        if r.map_epoch.get() == epoch
            && r.session.borrow().world_info.id == world_id
            && !r.state.world_busy.get_untracked()
            && !r.state.import_busy.get_untracked()
        {
            r.map_lock.set(Some(epoch));
            r.state.world_busy.set(true);
            acquired = true;
        }
    });
    acquired
}

#[wasm_bindgen(js_name=briccraftMapUnlock)]
pub fn unlock(epoch: u32) {
    with_runtime(|r| {
        if r.map_lock.get() == Some(epoch) {
            r.map_lock.set(None);
            r.state.world_busy.set(false);
        }
    });
}

fn plan(
    source: &str,
    bounds: [i64; 4],
    anchor: &MapAnchor,
) -> Result<(MapRegion, Vec<GameAttribution>, String), String> {
    if source.len() > 32 * 1024 * 1024 {
        return Err("Map region exceeds 32 MiB".into());
    }
    let plan: briccraft_import::ImportPlan =
        serde_json::from_str(source).map_err(|e| e.to_string())?;
    if plan.placements.len() > 150_000 {
        return Err("Map region exceeds its part budget".into());
    }
    let projection = plan
        .report
        .projection
        .as_ref()
        .ok_or("Map region has no geographic projection")?;
    let same = |a: f64, b: f64| a.is_finite() && b.is_finite() && (a - b).abs() < 1e-8;
    if plan.report.source != briccraft_import::SourceKind::OpenStreetMap
        || !same(projection.latitude, anchor.latitude)
        || !same(projection.longitude, anchor.longitude)
        || !same(projection.meters_per_stud, anchor.meters_per_stud)
        || projection.origin != anchor.origin
    {
        return Err("Map region does not use this world's fixed geographic projection".into());
    }
    let northwest = anchor
        .unproject(bounds[0] as f64, bounds[1] as f64)
        .map_err(|e| e.to_string())?;
    let southeast = anchor
        .unproject(bounds[2] as f64, bounds[3] as f64)
        .map_err(|e| e.to_string())?;
    if !same(projection.bounds.north, northwest[0])
        || !same(projection.bounds.west, northwest[1])
        || !same(projection.bounds.south, southeast[0])
        || !same(projection.bounds.east, southeast[1])
    {
        return Err("Map region's source bounds do not match the requested area".into());
    }
    match (&plan.report.terrain, anchor.terrain) {
        (Some(terrain), true)
            if same(terrain.reference_elevation_m, anchor.reference_elevation_m)
                && same(terrain.effective_meters_per_plate, anchor.meters_per_plate)
                && same(terrain.effective_meters_per_stud, anchor.meters_per_stud) => {}
        (None, false) => {}
        _ => return Err("Map region changed the world's terrain datum or vertical scale".into()),
    }
    let report = serde_json::to_string(&plan.report).map_err(|e| e.to_string())?;
    if report.len() > 1_048_576 {
        return Err("Map source report exceeds 1 MiB".into());
    }
    let credits = plan
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
        .map(|p| {
            GlobalPlacement::new(
                p.part_id,
                p.color,
                GlobalPos::new(
                    i64::from(p.position[0]),
                    p.position[1],
                    i64::from(p.position[2]),
                ),
                p.rotation,
            )
        })
        .collect();
    Ok((MapRegion { bounds, placements }, credits, report))
}

#[wasm_bindgen(js_name=briccraftBeginMapWorld)]
pub fn begin(
    epoch: u32,
    previous_id: &str,
    id: &str,
    name: &str,
    anchor: &str,
    source: &str,
) -> bool {
    if id.is_empty() || id.len() > 128 || name.len() > 320 || anchor.len() > 4096 {
        return false;
    }
    let mut success = false;
    with_runtime(|r| {
        if r.map_epoch.get() != epoch
            || r.map_lock.get() != Some(epoch)
            || r.session.borrow().world_info.id != previous_id
        {
            return;
        }
        let result = (|| -> Result<Session, String> {
            let anchor: MapAnchor = serde_json::from_str(anchor).map_err(|e| e.to_string())?;
            let (region, credits, report) = plan(source, [0, 0, 128, 128], &anchor)?;
            let mut game = Session::new_mapped(42, anchor, region, name.into(), credits, true)
                .map_err(|e| e.to_string())?;
            let old = r.session.borrow();
            game.quality = old.quality;
            game.daylight = old.daylight;
            game.sound = old.sound;
            game.world_info.id = id.into();
            game.world_info.import_report = Some(report);
            game.world_info.validate().map_err(|e| e.to_string())?;
            Ok(game)
        })();
        match result {
            Ok(game) => {
                *r.session.borrow_mut() = game;
                r.renderer.borrow_mut().invalidate();
                r.state.close_panels();
                r.state.roam_active.set(true);
                r.state
                    .roam_status
                    .set("Free roam · nearby map terrain loads as you walk".into());
                if let Some(view) = r.viewport.get_untracked() {
                    let _ = view.focus();
                }
                success = true;
            }
            Err(error) => {
                r.session.borrow_mut().status = format!("Map world could not start: {error}");
                r.state
                    .roam_status
                    .set(format!("Map world could not start: {error}"));
            }
        }
        r.sync_hud();
    });
    success
}

#[wasm_bindgen(js_name=briccraftAppendMapRegion)]
pub fn append(epoch: u32, world_id: &str, bounds: &str, source: &str) -> bool {
    if bounds.len() > 256 {
        return false;
    }
    let mut accepted = false;
    with_runtime(|r| {
        if r.map_epoch.get() != epoch
            || r.session.borrow().world_info.id != world_id
            || r.state.world_busy.get_untracked()
            || r.state.import_busy.get_untracked()
        {
            return;
        }
        let result = (|| -> Result<(), String> {
            let bounds = serde_json::from_str(bounds).map_err(|e| e.to_string())?;
            let mut game = r.session.borrow_mut();
            let anchor = game
                .map_anchor()
                .ok_or("Open a mapped world before appending regions")?;
            let (region, credits, _) = plan(source, bounds, anchor)?;
            let mut info = game.world_info.clone();
            for credit in credits {
                if !info.attributions.contains(&credit) {
                    info.attributions.push(credit);
                }
            }
            info.validate().map_err(|e| e.to_string())?;
            game.append_map_region(region).map_err(|e| e.to_string())?;
            game.world_info = info;
            Ok(())
        })();
        match result {
            Ok(()) => accepted = true,
            Err(error) => r.state.roam_status.set(format!("Map streaming: {error}")),
        }
        r.sync_hud();
    });
    accepted
}
