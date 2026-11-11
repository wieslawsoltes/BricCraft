//! Portable design files are browser services; all authoring UI is native FerroUI.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftDesignFile)]
    fn host_design(action: &str, source: &str);
}

pub fn open() {
    #[cfg(target_os = "emscripten")]
    host_design("import", "");
}

pub fn export(source: &str) {
    #[cfg(target_os = "emscripten")]
    host_design("export", source);
    #[cfg(not(target_os = "emscripten"))]
    let _ = source;
}
