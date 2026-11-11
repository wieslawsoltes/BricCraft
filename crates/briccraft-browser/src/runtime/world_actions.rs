use super::*;
use serde_json::json;

impl Runtime {
    /// Return true for commands handled by the library/import service boundary.
    pub(super) fn dispatch_world(&self, action: &UiAction) -> bool {
        match action {
            UiAction::ShowWorlds => {
                let open=!self.state.worlds_open.get_untracked();
                self.state.close_panels(); self.state.worlds_open.set(open);
                crate::world_host::world("list","{}");
            }
            UiAction::CreateWorld {preset,name,seed} => self.world_request("create",json!({"preset":preset,"name":name,"seed":seed.to_string()})),
            UiAction::NewWorld(seed) => self.world_request("create",json!({"preset":"island","name":format!("Classic island {seed}"),"seed":seed.to_string()})),
            UiAction::OpenWorld(id) => self.world_request("open",json!({"id":id})),
            UiAction::SaveWorldAs(name) => self.world_request("copy",json!({"name":name})),
            UiAction::ArchiveWorld {id,archived} => crate::world_host::world("archive",&json!({"id":id,"archived":archived}).to_string()),
            UiAction::ImportMap {latitude,longitude,radius,meters_per_stud,endpoint,local,terrain,fit_height,vertical_exaggeration} => self.source_request("map",json!({
                "latitude":latitude,"longitude":longitude,"radius":radius,"meters_per_stud":meters_per_stud,"endpoint":endpoint,"local":local,
                "terrain":terrain,"fit_height":fit_height,"vertical_exaggeration":vertical_exaggeration,
            })),
            UiAction::ImportLandmark => self.source_request("map",json!({"landmark":true})),
            UiAction::ImportTerrainSample => self.source_request("map",json!({"terrain_sample":true})),
            UiAction::ImportModel {size,sample} => self.source_request("model",json!({"size":size,"sample":sample})),
            UiAction::AppendModel {size,map_scale} => self.append_model(*size,*map_scale),
            UiAction::CancelImport => {
                let accepted={
                    let mut game=self.session.borrow_mut();
                    let accepted=game.cancel_import();
                    if accepted { let _=game.take_import_result(); }
                    else { self.state.import_status.set(game.status.clone()); }
                    accepted
                };
                if accepted {
                    self.import_anchor.set(None);
                    crate::world_host::cancel();
                    self.state.import_busy.set(false);
                }
            }
            UiAction::OpenSource(url) => crate::world_host::link(url),
            UiAction::ExportReport => {
                let mut game=self.session.borrow_mut();
                if let Some(report)=&game.world_info.import_report { crate::world_host::report(report); }
                else { game.status="This procedural world has no source import report.".into(); }
            }
            _=>return false,
        }
        self.sync_hud();
        true
    }

    fn world_request(&self, action: &str, mut payload: serde_json::Value) {
        let mut game = self.session.borrow_mut();
        match game.to_json() {
            Ok(source) => {
                payload["snapshot"] = source.into();
                game.status = "Saving the current world before opening the next one…".into();
                self.state.world_busy.set(true);
                crate::world_host::world(action, &payload.to_string());
            }
            Err(error) => game.status = error.to_string(),
        }
    }

    fn source_request(&self, action: &str, mut payload: serde_json::Value) {
        let mut game = self.session.borrow_mut();
        match game.to_json() {
            Ok(source) => {
                payload["snapshot"] = source.into();
                payload["world_id"] = game.world_info.id.clone().into();
                crate::world_host::source(action, &payload.to_string());
            }
            Err(error) => game.status = error.to_string(),
        }
    }

    fn append_model(&self, size: f64, map_scale: bool) {
        let mut game = self.session.borrow_mut();
        if game.gameplay.is_expedition() {
            game.status = "Switch this world to Creative before importing additional geometry. Your expedition progress stays saved.".into();
            return;
        }
        if !game.is_streamed() {
            game.status = "Open a streamed world before adding a model.".into();
            return;
        }
        let p = game.preview.map(|p| p.position).unwrap_or_else(|| {
            let x = game.camera.target.x;
            let z = game.camera.target.z;
            [
                x.round() as i32,
                (game.world.surface_height(x, z) / 0.4).ceil() as i32,
                z.round() as i32,
            ]
        });
        let anchor = match game.world_origin().global_cell(p) {
            Ok(anchor) => anchor,
            Err(error) => {
                game.status = error.to_string();
                return;
            }
        };
        let scale = if map_scale {
            game.world_info
                .import_report
                .as_deref()
                .and_then(|report| serde_json::from_str::<serde_json::Value>(report).ok())
                .and_then(|report| report.get("projection")?.get("meters_per_stud")?.as_f64())
        } else {
            None
        };
        if map_scale && scale.is_none() {
            game.status =
                "This world has no map scale. Turn off meter scale to fit the model by studs."
                    .into();
            return;
        }
        if p[1] >= 71 {
            game.status = "Choose a lower construction target for this model.".into();
            return;
        }
        match game.to_json() {
            Ok(snapshot) => {
                self.import_anchor.set(Some(anchor));
                *self.import_world_id.borrow_mut() = game.world_info.id.clone();
                crate::world_host::source("model",&json!({"size":size,"sample":false,"append":true,
                    "world_id":game.world_info.id,"units_per_stud":scale,"max_height_plates":72-p[1],"snapshot":snapshot}).to_string());
            }
            Err(error) => game.status = error.to_string(),
        }
    }
}
