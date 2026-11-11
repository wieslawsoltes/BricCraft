use super::*;

impl Runtime {
    pub fn sync_hud(&self) {
        let game = self.session.borrow();
        let gameplay = gameplay::project(&game);
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
        let stats = if let Some(stream) = game.stream_stats() {
            format!(
                "{} nearby bricks · {} regions resident",
                game.world.len(),
                stream.cached_chunks
            )
        } else {
            format!(
                "{} parts · {} pieces to discover",
                game.world.len(),
                briccraft_parts::CATALOG.len()
            )
        };
        let world_name = game.world_info.name.clone();
        let world_id = game.world_info.id.clone();
        let streamed = game.is_streamed();
        let p = game.global_camera_position();
        let coordinates = if game.is_streamed() {
            format!("X {:.0} · Y {:.1} · Z {:.0}", p[0], p[1], p[2])
        } else {
            String::new()
        };
        let credits: Vec<_> = game
            .world_info
            .attributions
            .iter()
            .map(|source| briccraft_ui::SourceCredit {
                label: source.label.clone(),
                url: source.url.clone(),
                license: source.license.clone(),
            })
            .collect();
        let attribution = credits.first().map_or_else(String::new, |source| {
            let mut label: String = source.label.chars().take(72).collect();
            if source.label.chars().count() > 72 {
                label.push('…');
            }
            if credits.len() > 1 {
                label.push_str(&format!(" · +{} sources", credits.len() - 1));
            }
            label
        });
        let attribution_url = game
            .world_info
            .attributions
            .first()
            .map_or_else(String::new, |a| a.url.clone());
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
            format!("{} · {}", gameplay.workshop.title, gameplay.workshop.detail)
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
            state.world_name.set_if_changed(world_name);
            state.world_id.set_if_changed(world_id);
            state.streamed.set_if_changed(streamed);
            state.coordinates.set_if_changed(coordinates);
            state.attribution.set_if_changed(attribution);
            state.credits.set_if_changed(credits);
            state.attribution_url.set_if_changed(attribution_url);
            state.fps.set_if_changed(self.fps.get());
            state.seed.set_if_changed(seed);
            state.quality.set_if_changed(quality);
            state.sound.set_if_changed(sound);
            state.grid.set_if_changed(grid);
            state.paused.set_if_changed(paused);
            state.daylight.set_if_changed(daylight);
            state.mechanics.set_if_changed(mechanics);
            state.gameplay.set_if_changed(gameplay);
        });
    }
}
