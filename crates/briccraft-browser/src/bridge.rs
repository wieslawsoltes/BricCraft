//! The only browser boundary: startup, animation timing, keys, and user-requested storage.
use crate::{application::App, runtime::Runtime};
use briccraft_ui::UiAction;
use ferroui_base::media::FontManagerOptions;
use ferroui_browser::{
    BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderer, BrowserRenderingMode,
};
use ferroui_controls::{AppBuilder, TextBox, TopLevel};
use ferroui_fonts_inter::AppBuilderExtension;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

ferroui_base::ferro_thread_local! {
    pub static RUNTIME:RefCell<Option<Rc<Runtime>>>=const{RefCell::new(None)};
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=globalThis,js_name=briccraftSave)]
    fn host_save(source: &str);
    #[wasm_bindgen(js_namespace=globalThis,js_name=briccraftLoad)]
    fn host_load();
    #[wasm_bindgen(js_namespace=globalThis,js_name=briccraftExport)]
    fn host_export(source: &str);
    #[wasm_bindgen(js_namespace=globalThis,js_name=briccraftImport)]
    fn host_import();
    #[wasm_bindgen(js_namespace=globalThis,js_name=briccraftSound)]
    fn host_sound(kind: &str);
}
pub fn save(source: &str) {
    #[cfg(target_os = "emscripten")]
    host_save(source);
    #[cfg(not(target_os = "emscripten"))]
    let _ = source;
}
pub fn load() {
    #[cfg(target_os = "emscripten")]
    host_load();
}
pub fn export(source: &str) {
    #[cfg(target_os = "emscripten")]
    host_export(source);
    #[cfg(not(target_os = "emscripten"))]
    let _ = source;
}
pub fn import() {
    #[cfg(target_os = "emscripten")]
    host_import();
}
pub fn play(kind: &str) {
    #[cfg(target_os = "emscripten")]
    host_sound(kind);
    #[cfg(not(target_os = "emscripten"))]
    let _ = kind;
}

pub fn with_runtime(f: impl FnOnce(&Rc<Runtime>)) {
    // Drop the slot borrow before entering app code; a host callback may reenter the bridge.
    let runtime = RUNTIME.with(|slot| slot.borrow().clone());
    if let Some(runtime) = runtime {
        f(&runtime);
    }
}

#[wasm_bindgen(js_name=runMain)]
pub fn run_main(_query: &str) {
    let options = BrowserPlatformOptions {
        renderer: BrowserRenderer::Skia,
        rendering_mode: vec![
            BrowserRenderingMode::WebGL2,
            BrowserRenderingMode::Software2D,
        ],
        ..Default::default()
    };
    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
        .start_browser_app("out", Some(options));
}

#[wasm_bindgen(js_name=briccraftFrame)]
pub fn frame(time: f64, width: f64, height: f64) {
    with_runtime(|r| r.frame(time, width, height));
}

/// Only browser environment data crosses this boundary. Native FerroUI owns
/// safe-area layout, focus, hit testing, controls, and touch gestures.
#[wasm_bindgen(js_name=briccraftInputEnvironment)]
pub fn input_environment(touch: bool, covered_top: f64, covered_bottom: f64) {
    with_runtime(|runtime| {
        runtime.state.touch.set_if_changed(touch);
        runtime.input_occlusion.set([covered_top, covered_bottom]);
    });
}

/// Returns whether a key was consumed. The DOM adapter asks before cancelling
/// browser defaults; native text editing and button keyboard activation stay native.
#[wasm_bindgen(js_name=briccraftKey)]
pub fn key(code: &str, down: bool, command: bool, repeat: bool) -> bool {
    let mut handled = false;
    with_runtime(|runtime| {
        // Key releases always clear held state, even if focus moved to a text box.
        if !down {
            runtime.session.borrow_mut().input.set_key(code, false);
            return;
        }
        let state = runtime.state;
        let modal = state.modal_untracked();
        let focused = runtime
            .viewport
            .get_untracked()
            .and_then(|view| TopLevel::get_top_level(Some(&view)))
            .and_then(|top| top.focus_manager().get_focused_element());
        let typing = focused
            .as_ref()
            .is_some_and(|control| control.cast::<TextBox>().is_some());
        let world_focused = focused
            .as_ref()
            .is_none_or(|control| control.name().as_deref() == Some("WorldViewport"));
        if typing && code != "Escape" {
            return;
        }
        let driving = runtime.session.borrow().active_vehicle.is_some();
        let movement = matches!(
            code,
            "KeyW"
                | "KeyA"
                | "KeyS"
                | "KeyD"
                | "KeyE"
                | "KeyQ"
                | "ArrowUp"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
                | "Space"
                | "ShiftLeft"
                | "ShiftRight"
                | "ControlLeft"
                | "ControlRight"
        ) || code == "KeyB" && driving;
        // The Control key itself is also a flight input. Its modifier flag is
        // already set on keydown; only shortcut letters should be excluded.
        let flight_modifier = matches!(code, "ControlLeft" | "ControlRight");
        if (!command || flight_modifier) && !modal && (world_focused || driving) && movement {
            runtime.session.borrow_mut().input.set_key(code, true);
            handled = true;
            return;
        }
        if repeat {
            return;
        }
        let action = match (code, command) {
            ("KeyZ", true) => Some(UiAction::Undo),
            ("KeyY", true) => Some(UiAction::Redo),
            ("KeyS", true) => Some(UiAction::Save),
            ("Escape", _) => {
                if modal {
                    state.close_panels();
                    if let Some(view) = runtime.viewport.get_untracked() {
                        let _ = view.focus();
                    }
                    handled = true;
                    None
                } else if driving {
                    Some(UiAction::ExitVehicle)
                } else {
                    Some(UiAction::SetPaused(!state.paused.get_untracked()))
                }
            }
            ("KeyB", false) if !driving => {
                let open = !state.catalog_open.get_untracked();
                state.close_panels();
                state.catalog_open.set(open);
                handled = true;
                None
            }
            ("KeyG", false) => Some(UiAction::ShowWorlds),
            ("KeyH", false) => {
                let open = !state.help_open.get_untracked();
                state.close_panels();
                state.help_open.set(open);
                handled = true;
                None
            }
            ("KeyO", false) => {
                let open = !state.gameplay_open.get_untracked();
                state.close_panels();
                state.gameplay_open.set(open);
                handled = true;
                None
            }
            _ if modal => None,
            ("KeyR", false) => Some(UiAction::Rotate),
            ("KeyF" | "KeyV", false) => Some(UiAction::ActivateVehicle("auto".into())),
            ("Tab", false) if world_focused => Some(UiAction::ToggleExplore),
            ("Home", false) => Some(UiAction::ResetCamera),
            ("Delete" | "Backspace", false) if world_focused => Some(UiAction::Remove),
            ("Enter", false) if world_focused => Some(UiAction::Place),
            _ if !command && code.starts_with("Digit") => code[5..]
                .parse::<usize>()
                .ok()
                .and_then(|slot| slot.checked_sub(1))
                .and_then(|slot| briccraft_ui::HOTBAR_PARTS.get(slot))
                .map(|id| UiAction::SelectPart(*id)),
            _ => None,
        };
        if let Some(action) = action {
            handled = true;
            runtime.dispatch(action);
        }
    });
    handled
}

#[wasm_bindgen(js_name=briccraftBlur)]
pub fn blur() {
    with_runtime(|runtime| {
        runtime.session.borrow_mut().input.clear();
        let captured = runtime.pointer.borrow_mut().reset();
        for pointer in captured {
            pointer.capture(None);
        }
    });
}

/// Safari trackpad gesture fallback. Native hit testing keeps a gesture over a
/// palette or text editor from changing the world behind it. Touchscreen pinches
/// are processed by the native pointer handlers instead.
#[wasm_bindgen(js_name=briccraftGesture)]
pub fn gesture(x: f64, y: f64, zoom: f64) -> bool {
    if !x.is_finite() || !y.is_finite() || !zoom.is_finite() {
        return false;
    }
    let mut handled = false;
    with_runtime(|runtime| {
        let Some(view) = runtime.viewport.get_untracked() else {
            return;
        };
        let Some(top) = TopLevel::get_top_level(Some(&view)) else {
            return;
        };
        if top
            .input_hit_test(ferroui_base::Point::new(x, y))
            .is_some_and(|hit| hit.name().as_deref() == Some("WorldViewport"))
        {
            let mut game = runtime.session.borrow_mut();
            game.camera.zoom(zoom.clamp(-600.0, 600.0) as f32);
            game.update_pick();
            handled = true;
        }
    });
    handled
}

#[wasm_bindgen(js_name=briccraftImportWorld)]
pub fn import_world(source: &str) -> bool {
    let mut success = false;
    with_runtime(|runtime| {
        let result = runtime.session.borrow_mut().import_json(source);
        match result {
            Ok(()) => {
                runtime.renderer.borrow_mut().invalidate();
                runtime.state.close_panels();
                success = true;
            }
            Err(e) => runtime.session.borrow_mut().status = e.to_string(),
        }
        runtime.sync_hud();
    });
    success
}

#[wasm_bindgen(js_name=briccraftMessage)]
pub fn message(text: &str) {
    with_runtime(|r| {
        r.session.borrow_mut().status = text.to_string();
        r.sync_hud();
    });
}
