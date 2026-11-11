//! Browser-owned rendering preferences; independent of any saved world.
use super::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=globalThis, js_name=briccraftSaveGraphics)]
    fn host_save_graphics(mode: u8, samples: u8);
}

impl Runtime {
    pub(super) fn dispatch_graphics(&self, action: &UiAction) -> bool {
        let (mode, samples) = {
            let mut controller = self.world_resolution.borrow_mut();
            match action {
                UiAction::SetResolutionMode(value) => controller.set_mode(*value),
                UiAction::SetAntialiasing(value) => controller.set_samples(*value),
                _ => return false,
            }
            (controller.mode(), controller.requested_samples())
        };
        #[cfg(target_os = "emscripten")]
        host_save_graphics(mode, samples);
        batch(|| {
            self.state.resolution_mode.set_if_changed(mode);
            self.state.antialiasing.set_if_changed(samples);
        });
        true
    }

    pub(super) fn sync_graphics(&self) {
        let (mode, samples, stats) = {
            let controller = self.world_resolution.borrow();
            (
                controller.mode(),
                controller.requested_samples(),
                controller.stats(),
            )
        };
        let detail = ["Fast", "Balanced", "Cinematic"][stats.effective_quality.min(2) as usize];
        let summary = if stats.pixel_width > 0 {
            format!(
                "World {} × {} · {}× antialiasing · {}",
                stats.pixel_width, stats.pixel_height, stats.samples, detail
            )
        } else {
            "Preparing the world image…".into()
        };
        batch(|| {
            self.state.resolution_mode.set_if_changed(mode);
            self.state.antialiasing.set_if_changed(samples);
            self.state.render_status.set_if_changed(summary);
        });
    }
}
