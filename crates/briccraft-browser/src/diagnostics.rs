//! Read-only diagnostics distinguish prepared effects from actual presentation.
use crate::bridge::with_runtime;
use ferroui_browser::interop::canvas_helper::{
    RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL,
};
use ferroui_browser::rendering::RenderStatistics;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=briccraftState)]
pub fn state() -> String {
    let mut result = "{}".to_string();
    with_runtime(|r| {
        let game = r.session.borrow();
        let render = r.renderer.borrow().stats();
        let effect_reuses = r.renderer.borrow().reused_effects();
        let shader_samples = r.renderer.borrow().samples();
        // These count frames actually presented by FerroUI, independently of
        // effect snapshots prepared by the game or the selected startup mode.
        let presented = RenderStatistics::current();
        let world_rendering = r.world_resolution.borrow().stats();
        let interface = r.state.layout.get_untracked();
        let render_kind = match presented.frame_kind {
            RENDER_TARGET_KIND_WEB_GL => "webgl",
            RENDER_TARGET_KIND_SOFTWARE => "software",
            _ => "none",
        };
        // Keep each object bounded: deeply nested, growing json! expansions
        // otherwise consume the macro recursion budget despite a flat schema.
        let renderer = serde_json::json!({
            "uploads":render.uploads,"bytes":render.texture_bytes,"effectFrames":render.effect_frames,
            "uniformFrames":render.uniform_only_frames,"activeCells":render.active_cells,
            "shaderQuality":render.shader_quality,"shaderVariant":render.shader_variant,
            "samples":shader_samples,"effectReuses":effect_reuses,
        });
        let rendering = serde_json::json!({
            "kind":render_kind,"frameKind":presented.frame_kind,"glMajorVersion":presented.frame_gl_major_version,
            "frames":presented.frames,"frameThread":presented.frame_thread,
            "width":presented.frame_width,"height":presented.frame_height,
            "ticks":presented.ticks,"proxiedCalls":presented.tick_proxied_calls,"panics":presented.render_thread_panics,
        });
        let world_rendering = serde_json::json!({
            "scale":world_rendering.render_scale,
            "width":world_rendering.pixel_width,"height":world_rendering.pixel_height,
            "effectWidth":world_rendering.effect_width,"effectHeight":world_rendering.effect_height,
            "pixels":world_rendering.pixel_width as u64 * world_rendering.pixel_height as u64,
            "pixelBudget":world_rendering.pixel_budget,"minimumPixels":world_rendering.minimum_pixels,
            "resolutionMode":world_rendering.mode_name(),"samples":world_rendering.samples,
            "requestedSamples":world_rendering.requested_samples,
            "requestedQuality":world_rendering.requested_quality,
            "effectiveQuality":world_rendering.effective_quality,
            "presentedFps":world_rendering.presented_fps,
        });
        let interface = serde_json::json!({
            "width":interface.width,"height":interface.height,"insets":interface.insets,
            "compact":interface.compact,"landscape":interface.landscape,
            "touch":interface.touch,"keyboard":interface.keyboard,
            "modal":r.state.modal_untracked(),"panelWidth":interface.panel_width(),
            "catalogOpen":r.state.catalog_open.get_untracked(),"studioOpen":r.state.imports_open.get_untracked(),
            "gameplayOpen":r.state.gameplay_open.get_untracked(),
            "settingsOpen":r.state.settings_open.get_untracked(),
            "placesOpen":r.state.places_open.get_untracked(),
            "blueprintsOpen":r.state.blueprints_open.get_untracked(),
            "designsOpen":r.state.blueprints_open.get_untracked() && r.state.blueprint_section.get_untracked() == 1,
            "sourceCount":r.state.credits.get_untracked().len(),
        });
        let serde_json::Value::Object(world) = serde_json::json!({
            "worldId":game.world_info.id,"worldName":game.world_info.name,"preset":game.world_info.preset,
            "streamed":game.is_streamed(),"origin":{"x":game.world_origin().x.to_string(),"z":game.world_origin().z.to_string()},
            "cachedChunks":game.stream_stats().map(|s|s.cached_chunks),"sceneGeneration":game.scene_generation,
            "attributions":game.world_info.attributions,"importBusy":r.state.import_busy.get_untracked(),
            "worldBusy":r.state.world_busy.get_untracked(),"worldsOpen":r.state.worlds_open.get_untracked(),"worldCount":r.state.worlds.get_untracked().len(),
        }) else {
            unreachable!("world diagnostics are an object")
        };
        let mut snapshot = serde_json::json!({
            "parts":game.world.len(),"revision":game.world.revision(),"catalog":briccraft_parts::CATALOG.len(),
            "selectedPart":game.selected_part,"color":game.color,"mode":game.mode_name(),"status":game.status,"seed":game.seed.to_string(),
            "camera":game.camera.position.to_array(),"globalCamera":game.global_camera_position(),"cursor":game.cursor,
            "vehicle":game.active_vehicle.as_ref().map(|v|v.vehicle.position),
            "ready":r.world_effect.get_untracked().is_some(),"fps":r.state.fps.get_untracked(),
        });
        let fields = snapshot
            .as_object_mut()
            .expect("session diagnostics are an object");
        fields.extend(world);
        fields.insert(
            "gameplay".into(),
            crate::runtime::gameplay::diagnostics(&game),
        );
        fields.insert("renderer".into(), renderer);
        fields.insert(
            "designs".into(),
            crate::runtime::designs::diagnostics(&game),
        );
        fields.insert("rendering".into(), rendering);
        fields.insert("worldRendering".into(), world_rendering);
        fields.insert("interface".into(), interface);
        fields.insert("map".into(),serde_json::json!({
            "active":r.state.roam_active.get_untracked(),"status":r.state.roam_status.get_untracked(),
            "regions":game.mapped_regions().len(),"waiting":game.map_waiting(),
            "sourceParts":game.mapped_source_parts(),
            "geographicPosition":game.map_anchor().and_then(|a|a.unproject(game.global_camera_position()[0],game.global_camera_position()[2]).ok()),
        }));
        result = snapshot.to_string();
    });
    result
}
