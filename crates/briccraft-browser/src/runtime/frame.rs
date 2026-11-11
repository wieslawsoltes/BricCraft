use super::*;

impl Runtime {
    pub fn frame(&self, milliseconds: f64, _width: f64, _height: f64) {
        if !self.owner.is_alive() {
            return;
        }
        let previous = self.last_frame.replace(milliseconds);
        let elapsed = if previous == 0.0 {
            1.0 / 60.0
        } else {
            ((milliseconds - previous) / 1000.0).max(0.0)
        };
        // Wait for native layout before publishing an expensive world effect.
        // These are the same logical bounds and DPI the bitmap cache will use.
        let Some(view) = self.viewport.get_untracked() else {
            return;
        };
        let bounds = view.bounds();
        if !bounds.width.is_finite()
            || !bounds.height.is_finite()
            || bounds.width <= 0.0
            || bounds.height <= 0.0
        {
            return;
        }
        let Some(top_level) = ferroui_controls::TopLevel::get_top_level(Some(&view)) else {
            return;
        };
        if !self.state.modal_untracked()
            && top_level
                .focus_manager()
                .get_focused_element()
                .is_some_and(|focused| !focused.is_effectively_visible())
        {
            let _ = view.focus();
        }
        let layout = self
            .layout_metrics
            .borrow_mut()
            .get_or_insert_with(|| layout::LayoutMetrics::new(&top_level))
            .measure(
                [bounds.width, bounds.height],
                self.input_occlusion.get(),
                self.state.touch.get_untracked(),
            );
        batch(|| {
            self.state.compact.set_if_changed(layout.compact);
            self.state.layout.set_if_changed(layout);
        });
        let presented = ferroui_browser::rendering::RenderStatistics::current().frames;
        let requested_quality = self.session.borrow().quality;
        let resolution = self.world_resolution.borrow_mut().update(
            elapsed,
            presented,
            [bounds.width, bounds.height],
            top_level.render_scaling(),
            requested_quality,
        );
        if self.world_cache.render_at_scale() != resolution.render_scale {
            self.world_cache
                .set_render_at_scale(resolution.render_scale);
        }
        let modal = self.state.modal_untracked();
        let was_modal = self.modal_active.replace(modal);
        if modal && !was_modal {
            // A menu opened during a drag must cancel the old world gesture.
            // Capture loss runs native callbacks, so release outside borrows.
            let captured = self.pointer.borrow_mut().reset();
            for pointer in captured {
                pointer.capture(None);
            }
        }
        let mut game = self.session.borrow_mut();
        game.aspect = (bounds.width / bounds.height) as f32;
        if modal {
            game.input.clear();
        }
        game.tick(elapsed as f32);
        let import_progress = game.import_progress();
        let import_result = game.take_import_result();
        if self.scene_generation.replace(game.scene_generation) != game.scene_generation {
            self.renderer.borrow_mut().invalidate();
        }
        let preview = if modal || game.active_vehicle.is_some() {
            None
        } else {
            game.preview.map(|p| PlacementPreview {
                position: p.position,
                size: p.size(),
                valid: game.world.can_place(&p).is_ok() && game.selected_part_quote().affordable,
            })
        };
        let camera = &game.camera;
        let frame = RenderFrame {
            camera: Camera {
                position: camera.position.to_array(),
                yaw: camera.yaw,
                pitch: camera.pitch,
                vertical_fov: camera.vertical_fov,
            },
            time: game.time,
            daylight: game.daylight,
            water_level: 2.4,
            selected_id: game.hit.as_ref().filter(|_| !modal).map(|h| h.id),
            preview,
            quality: resolution.effective_quality,
            grid: game.grid,
            window_fade: if game.is_streamed() { 16.0 } else { 0.0 },
            vehicle: game.active_vehicle.as_ref().map(|a| VehicleRender {
                instance_ids: a.instance_ids.clone(),
                pivot: a.pivot,
                position: a.vehicle.position,
                yaw: a.render_yaw(),
                pitch: a.vehicle.pitch,
                roll: a.vehicle.roll,
                wheel_angle: a.vehicle.wheel_angle,
                engine_rpm: a.vehicle.engine_rpm,
                parts: Some(a.parts.clone()),
            }),
        };
        match self.renderer.borrow_mut().effect(&game.world, &frame) {
            Ok(effect) => self.world_effect.set(Some(effect)),
            Err(error) => game.status = format!("Rendering error: {error}"),
        }
        drop(game);
        if let Some(progress) = import_progress {
            self.state.import_busy.set_if_changed(true);
            self.state.import_progress.set_if_changed(
                0.82 + 0.17 * progress.validated_chunks as f64
                    / progress.total_chunks.max(1) as f64,
            );
            self.state.import_status.set_if_changed(format!(
                "Placing {} bricks · {} / {} regions checked",
                progress.total_parts, progress.validated_chunks, progress.total_chunks
            ));
        }
        if let Some(result) = import_result {
            let success = result.is_ok();
            let summary = result.unwrap_or_else(|error| error);
            self.state.import_busy.set(false);
            self.state.import_status.set(summary.clone());
            crate::world_host::finished(success, &summary);
        }
        let last_presented = self.presented_frames.replace(presented);
        if previous > 0.0 && elapsed > 0.0 {
            // Count FerroUI's presented frames over actual wall time. Averaging reciprocal intervals
            // lets brief callbacks conceal multi-second renderer stalls.
            let seconds = self.fps_elapsed.get() + elapsed;
            let frames = self.fps_frames.get() + presented.saturating_sub(last_presented);
            if seconds >= 0.5 {
                self.fps.set(frames as f64 / seconds);
                self.fps_elapsed.set(0.0);
                self.fps_frames.set(0);
            } else {
                self.fps_elapsed.set(seconds);
                self.fps_frames.set(frames);
            }
        }
        if milliseconds - self.last_hud.get() > 300.0 {
            self.last_hud.set(milliseconds);
            self.sync_hud();
        }
    }
}
