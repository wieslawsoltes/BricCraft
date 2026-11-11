//! Native design actions and small presentation models share the domain rules.
use super::*;
use briccraft_ui::{DesignLibraryView, SavedDesignCard};

impl Runtime {
    pub(super) fn dispatch_design(&self, action: &UiAction) -> bool {
        let mut place = false;
        let mut export = None;
        match action {
            UiAction::ImportDesign => {
                crate::design_host::open();
                return true;
            }
            UiAction::CaptureDesign(_)
            | UiAction::PlaceDesign(_)
            | UiAction::RenameDesign { .. }
            | UiAction::DeleteDesign(_)
            | UiAction::ExportDesign(_) => {}
            _ => return false,
        }
        {
            let mut game = self.session.borrow_mut();
            let result = match action {
                UiAction::CaptureDesign(name) => game.capture_design(name).map(|_| ()),
                UiAction::PlaceDesign(id) => {
                    place = true;
                    game.place_design(*id).map(|_| ())
                }
                UiAction::RenameDesign { id, name } => game.rename_design(*id, name),
                UiAction::DeleteDesign(id) => game.delete_design(*id),
                UiAction::ExportDesign(id) => game.export_design(*id).map(|source| {
                    export = Some(source);
                }),
                _ => unreachable!("design action was matched above"),
            };
            if let Err(error) = result {
                game.status = error;
                place = false;
            }
        }
        if let Some(source) = export {
            crate::design_host::export(&source);
        }
        if place {
            self.focus_world_unless_editing();
        } else {
            self.show_designs();
        }
        self.sync_hud();
        true
    }

    pub(crate) fn show_designs(&self) {
        self.state.close_panels();
        self.state.blueprint_section.set(1);
        self.state.blueprints_open.set(true);
    }
}

pub(super) fn project(game: &Session) -> DesignLibraryView {
    let capture = game.design_capture_status();
    DesignLibraryView {
        designs: game
            .designs()
            .iter()
            .filter_map(|design| {
                game.design_quote(design.id).map(|quote| SavedDesignCard {
                    id: design.id,
                    name: design.name.clone(),
                    parts: design.part_count(),
                    size: design.dimensions(),
                    studs: quote.studs,
                    affordable: quote.affordable,
                })
            })
            .collect(),
        capture_ready: capture.ready,
        capture_detail: capture.detail,
        capture_parts: capture.part_count,
    }
}

pub(crate) fn diagnostics(game: &Session) -> serde_json::Value {
    let capture = game.design_capture_status();
    let designs: Vec<_> = game
        .designs()
        .iter()
        .map(|design| {
            let quote = game.design_quote(design.id).unwrap_or_default();
            serde_json::json!({
                "id":design.id,"name":design.name,"parts":design.part_count(),
                "size":design.dimensions(),"components":design.component_count(),
                "studs":quote.studs,"affordable":quote.affordable,"charged":quote.charged,
                "source":design.metadata(),
            })
        })
        .collect();
    serde_json::json!({
        "items":designs,
        "capture":{"ready":capture.ready,"detail":capture.detail,"parts":capture.part_count},
    })
}
