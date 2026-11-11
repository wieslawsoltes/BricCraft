//! Automation and portable files use the same validated recipe operations as native controls.
use crate::bridge::with_runtime;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=briccraftSerializeDesign)]
pub fn serialize_design(id: u32) -> String {
    let mut result = String::new();
    with_runtime(|runtime| {
        let exported = runtime.session.borrow().export_design(id);
        match exported {
            Ok(source) => result = source,
            Err(error) => runtime.session.borrow_mut().status = error,
        }
        runtime.sync_hud();
    });
    result
}

#[wasm_bindgen(js_name=briccraftImportDesign)]
pub fn import_design(source: &str) -> bool {
    let mut success = false;
    with_runtime(|runtime| {
        if runtime.state.world_busy.get_untracked() || runtime.state.import_busy.get_untracked() {
            runtime.session.borrow_mut().status =
                "Finish loading the world before importing a design.".into();
            runtime.sync_hud();
            return;
        }
        let result = runtime.session.borrow_mut().import_design(source);
        match result {
            Ok(_) => {
                success = true;
                runtime.show_designs();
            }
            Err(error) => runtime.session.borrow_mut().status = error,
        }
        runtime.sync_hud();
    });
    success
}
