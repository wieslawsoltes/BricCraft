use super::*;

impl Runtime {
    pub fn frame(&self, milliseconds: f64, width: f64, height: f64) {
        if !self.owner.is_alive() {
            return;
        }
        let previous = self.last_frame.replace(milliseconds);
        let elapsed = if previous == 0.0 {
            1.0 / 60.0
        } else {
            ((milliseconds - previous) / 1000.0).clamp(0.0, 0.1)
        };
        self.state.compact.set_if_changed(width < 850.0);
        let mut game = self.session.borrow_mut();
        game.aspect = (width / height.max(1.0)) as f32;
        let modal = self.state.catalog_open.get_untracked()
            || self.state.blueprints_open.get_untracked()
            || self.state.help_open.get_untracked()
            || self.state.settings_open.get_untracked();
        if modal {
            game.input.clear();
        }
        game.tick(elapsed as f32);
        let preview = if modal || game.active_vehicle.is_some() {
            None
        } else {
            game.preview.map(|p| PlacementPreview {
                position: p.position,
                size: p.size(),
                valid: game.world.can_place(&p).is_ok(),
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
            quality: game.quality,
            grid: game.grid,
            vehicle: game.active_vehicle.as_ref().map(|a| VehicleRender {
                instance_ids: a.instance_ids.clone(),
                pivot: a.pivot,
                position: a.vehicle.position,
                yaw: a.vehicle.yaw,
                pitch: a.vehicle.pitch,
                roll: a.vehicle.roll,
                wheel_angle: a.vehicle.wheel_angle,
                engine_rpm: a.vehicle.engine_rpm,
            }),
        };
        match self.renderer.borrow_mut().effect(&game.world, &frame) {
            Ok(effect) => self.world_effect.set(Some(effect)),
            Err(error) => game.status = format!("Rendering error: {error}"),
        }
        drop(game);
        if elapsed > 0.0 {
            self.fps.set(self.fps.get() * 0.94 + (1.0 / elapsed) * 0.06);
        }
        if milliseconds - self.last_hud.get() > 300.0 {
            self.last_hud.set(milliseconds);
            self.sync_hud();
        }
    }
}
