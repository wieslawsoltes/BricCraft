use super::*;

impl Runtime {
    pub fn sync_hud(&self) {
        let game = self.session.borrow();
        let state = self.state;
        let selected_part = game.selected_part;
        let selected_color = game.color;
        let part = briccraft_parts::part(game.selected_part);
        let name = part.map_or_else(String::new, |p| p.name.into());
        let size = part.map_or_else(String::new, |p| {
            let s = p.rotated_size(game.rotation);
            format!(
                "{} × {} studs · {} plates · {}°",
                s[0],
                s[2],
                s[1],
                u16::from(game.rotation) * 90
            )
        });
        let status = game.status.clone();
        let mode = game.mode_name().to_string();
        let stats = format!(
            "{} parts · {} pieces to discover",
            game.world.len(),
            briccraft_parts::CATALOG.len()
        );
        let seed = game.seed;
        let quality = game.quality;
        let sound = game.sound;
        let grid = game.grid;
        let paused = game.paused;
        let daylight = f64::from(game.daylight);
        let mechanics = if let Some(active) = &game.active_vehicle {
            format!(
                "{:?} · {:.1} studs/s · {:.0} RPM · gear {}",
                active.vehicle.kind,
                active.vehicle.speed,
                active.vehicle.engine_rpm,
                active.vehicle.gear
            )
        } else {
            "Connect a chassis, motor, axles and wheels. Insert a blueprint or build your own."
                .into()
        };
        drop(game);
        batch(|| {
            state.selected_part.set_if_changed(selected_part);
            state.selected_color.set_if_changed(selected_color);
            state.selected_name.set_if_changed(name);
            state.selected_size.set_if_changed(size);
            state.status.set_if_changed(status);
            state.mode.set_if_changed(mode);
            state.stats.set_if_changed(stats);
            state.fps.set_if_changed(self.fps.get());
            state.seed.set_if_changed(seed);
            state.quality.set_if_changed(quality);
            state.sound.set_if_changed(sound);
            state.grid.set_if_changed(grid);
            state.paused.set_if_changed(paused);
            state.daylight.set_if_changed(daylight);
            state.mechanics.set_if_changed(mechanics);
        });
    }
}
