use super::*;
use serde_json::json;

impl Runtime {
    pub(super) fn dispatch_map(&self, action: &UiAction) -> bool {
        match action {
            UiAction::ShowPlaces => {
                if self.state.world_busy.get_untracked() || self.state.import_busy.get_untracked() {
                    return true;
                }
                let open = !self.state.places_open.get_untracked();
                self.state.close_panels();
                if open {
                    // This result belongs to an attempt in the current picker.
                    // An earlier model/area import must not label a new place.
                    self.state.import_status.set(String::new());
                }
                self.state.places_open.set(open);
                self.refresh_map_tiles(true);
            }
            UiAction::MapViewportChanged => self.refresh_map_tiles(true),
            UiAction::LocateMapPlayer => {
                if self.state.world_busy.get_untracked() || self.state.import_busy.get_untracked() {
                    return true;
                }
                // Sample now rather than using the 300-ms HUD projection. The
                // command moves only the display map, never the world or pin.
                self.sync_hud();
                if let Some(player) = self.state.map_player.get_untracked() {
                    let mut view = self.state.map_view.get_untracked();
                    player.center_view(&mut view);
                    if !self.state.places_open.get_untracked() {
                        self.state.import_status.set(String::new());
                    }
                    self.state.close_panels();
                    self.state.map_view.set_if_changed(view);
                    self.state.places_open.set(true);
                    self.refresh_map_tiles(false);
                }
            }
            UiAction::StartMapRoam {
                latitude,
                longitude,
                meters_per_stud,
                terrain,
            } => {
                if self.state.world_busy.get_untracked() || self.state.import_busy.get_untracked() {
                    return true;
                }
                let mut game = self.session.borrow_mut();
                if !latitude.is_finite()
                    || !longitude.is_finite()
                    || !meters_per_stud.is_finite()
                    || latitude.abs() > 84.9
                    || longitude.abs() > 179.9
                    || !(0.5..=16.0).contains(meters_per_stud)
                {
                    game.status =
                        "Choose valid map coordinates and a scale from 0.5 to 16 metres per stud."
                            .into();
                    drop(game);
                    self.sync_hud();
                    return true;
                }
                self.map_epoch.set(self.map_epoch.get().wrapping_add(1));
                self.state.import_status.set(String::new());
                self.state.roam_active.set(true);
                self.state
                    .roam_status
                    .set("Preparing your starting area…".into());
                game.input.clear();
                crate::map_host::request(
                    "start",
                    &json!({
                        "epoch":self.map_epoch.get(), "world_id":game.world_info.id,
                        "latitude":latitude,"longitude":longitude,
                        "meters_per_stud":meters_per_stud,"terrain":terrain,
                        "name":self.state.map_view.get_untracked().label,
                    })
                    .to_string(),
                );
            }
            UiAction::RetryMapRoam => crate::map_host::request("retry", "{}"),
            UiAction::StopMapRoam => crate::map_host::request("stop", "{}"),
            _ => return false,
        }
        true
    }

    pub(super) fn refresh_map_tiles(&self, force: bool) {
        if !self.state.places_open.get_untracked() {
            if !self.map_requested_tiles.borrow().is_empty() {
                self.map_requested_tiles.borrow_mut().clear();
                self.map_tile_request
                    .set(self.map_tile_request.get().wrapping_add(1));
                crate::map_host::request("tiles-cancel", "{}");
            }
            return;
        }
        let view = self.state.map_view.get_untracked();
        let size = briccraft_ui::map_view_size(self.state.layout.get_untracked());
        let tiles = briccraft_ui::visible_map_tiles(&view, size);
        if !force && *self.map_requested_tiles.borrow() == tiles {
            return;
        }
        *self.map_requested_tiles.borrow_mut() = tiles.clone();
        let retained: Vec<_> = self
            .state
            .map_tiles
            .get_untracked()
            .into_iter()
            .filter(|tile| tiles.contains(&(tile.zoom, tile.x, tile.y)))
            .collect();
        let missing: Vec<_> = tiles
            .iter()
            .filter(|(z, x, y)| {
                !retained
                    .iter()
                    .any(|tile| tile.zoom == *z && tile.x == *x && tile.y == *y)
            })
            .copied()
            .collect();
        self.state.map_tiles.set(retained);
        let request = self.map_tile_request.get().wrapping_add(1);
        self.map_tile_request.set(request);
        if missing.is_empty() {
            self.state
                .map_status
                .set("Drag to explore · tap to choose your starting point".into());
        } else {
            self.state.map_status.set("Loading the map…".into());
        }
        crate::map_host::request(
            "tiles",
            &json!({"request":request,"tiles":missing}).to_string(),
        );
    }

    /// Invalidate every asynchronous result associated with a replaced session.
    pub(crate) fn map_world_changed(&self) {
        self.map_epoch.set(self.map_epoch.get().wrapping_add(1));
        self.state.roam_active.set(false);
        self.state.roam_status.set(String::new());
        self.state.map_player.set(None);
        if !self.state.import_busy.get_untracked() {
            self.state.import_status.set(String::new());
        }
    }
}
