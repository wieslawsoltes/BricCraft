//! Host services are asynchronous; the native view never creates DOM controls.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftWorldHost)]
    fn host_world(action: &str, payload: &str);
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftImportSource)]
    fn host_source(action: &str, payload: &str);
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftCancelImport)]
    fn host_cancel();
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftOpenSource)]
    fn host_link(url: &str);
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftImportFinished)]
    fn host_finished(success: bool, summary: &str);
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftExportReport)]
    fn host_report(source: &str);
}

pub fn world(action: &str, payload: &str) {
    #[cfg(target_os = "emscripten")]
    host_world(action, payload);
    #[cfg(not(target_os = "emscripten"))]
    let _ = (action, payload);
}
pub fn source(action: &str, payload: &str) {
    #[cfg(target_os = "emscripten")]
    host_source(action, payload);
    #[cfg(not(target_os = "emscripten"))]
    let _ = (action, payload);
}
pub fn cancel() {
    #[cfg(target_os = "emscripten")]
    host_cancel();
}
pub fn link(url: &str) {
    #[cfg(target_os = "emscripten")]
    host_link(url);
    #[cfg(not(target_os = "emscripten"))]
    let _ = url;
}
pub fn finished(success: bool, summary: &str) {
    #[cfg(target_os = "emscripten")]
    host_finished(success, summary);
    #[cfg(not(target_os = "emscripten"))]
    let _ = (success, summary);
}
pub fn report(source: &str) {
    #[cfg(target_os = "emscripten")]
    host_report(source);
    #[cfg(not(target_os = "emscripten"))]
    let _ = source;
}
