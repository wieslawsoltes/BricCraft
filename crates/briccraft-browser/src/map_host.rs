//! Deferred browser services for map imagery and geographic world acquisition.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftMapHost)]
    fn host_map(action: &str, payload: &str);
}

pub fn request(action: &str, payload: &str) {
    #[cfg(target_os = "emscripten")]
    host_map(action, payload);
    #[cfg(not(target_os = "emscripten"))]
    let _ = (action, payload);
}
