use crate::Session;
use briccraft_world::{Blueprint, BuildOp};

impl Session {
    pub fn spawn_blueprint(&mut self, name: &str) {
        if self.active_vehicle.is_some() {
            self.status = "Park your vehicle before inserting another construction.".into();
            return;
        }
        let Some(blueprint) = named(name) else {
            return;
        };
        let quote = self.blueprint_quote(name).expect("known blueprint quote");
        if !quote.affordable {
            self.status = format!(
                "{} costs {} studs; you have {}. Recycle parts or complete objectives first.",
                blueprint.name(),
                quote.studs,
                self.gameplay.studs()
            );
            return;
        }
        let center = self
            .hit
            .as_ref()
            .map_or([48, 18, 45], |h| [h.cell[0], h.cell[1] + 1, h.cell[2]]);
        let mut candidates = Vec::new();
        if blueprint == Blueprint::Sailboat {
            // A watercraft must start in water; use full-footprint depth rather than one sampled corner.
            let size = blueprint.size(self.rotation);
            for (x, z) in [
                (88, 22),
                (2, 60),
                (2, 20),
                (82, 22),
                (82, 56),
                (12, 2),
                (50, 2),
                (24, 82),
                (60, 82),
            ] {
                let clear = (x..x + size[0]).all(|px| {
                    (z..z + size[2])
                        .all(|pz| self.world.surface_height(px as f32 + 0.5, pz as f32 + 0.5) < 1.8)
                });
                if clear {
                    candidates.push(([x, 5, z], self.rotation));
                }
            }
        } else if blueprint == Blueprint::Locomotive && !self.is_streamed() {
            // East straight: +Z travel, track centre x74, rail top y8 world units.
            let size = blueprint.size(0);
            for z in [44, 32, 56] {
                candidates.push(([74 - size[0] / 2, 20, z - size[2] / 2], 0));
            }
        } else {
            if blueprint == Blueprint::Airplane && !self.is_streamed() {
                candidates.push(([27, 19, 40], 0));
            }
            let size = blueprint.size(self.rotation);
            for radius in [0_i32, 4, 8, 12, 18, 24, 32] {
                for (dx, dz) in [
                    (radius, 0),
                    (-radius, 0),
                    (0, radius),
                    (0, -radius),
                    (radius, radius),
                    (-radius, -radius),
                ] {
                    let x = (center[0] + dx).clamp(4, 90 - size[0]);
                    let z = (center[2] + dz).clamp(4, 90 - size[2]);
                    let samples = [
                        self.world.surface_height(x as f32 + 0.5, z as f32 + 0.5),
                        self.world
                            .surface_height((x + size[0] - 1) as f32 + 0.5, z as f32 + 0.5),
                        self.world
                            .surface_height(x as f32 + 0.5, (z + size[2] - 1) as f32 + 0.5),
                        self.world.surface_height(
                            (x + size[0] - 1) as f32 + 0.5,
                            (z + size[2] - 1) as f32 + 0.5,
                        ),
                    ];
                    let low = samples.into_iter().fold(f32::INFINITY, f32::min);
                    let high = samples.into_iter().fold(0.0, f32::max);
                    // Real-map ground uses its fixed elevation reference, so a
                    // perfectly dry flat area may sit below the classic sea plane.
                    let minimum_ground = if self.map_anchor().is_some() {
                        0.1
                    } else {
                        briccraft_world::SEA_LEVEL
                    };
                    if matches!(blueprint, Blueprint::Rover | Blueprint::Airplane)
                        && (high - low > 0.85 || low < minimum_ground)
                    {
                        continue;
                    }
                    candidates.push(([x, (high / 0.4).round() as i32, z], self.rotation));
                }
            }
        }
        let mut result = None;
        for (origin, rotation) in candidates {
            let operations: Vec<_> = blueprint
                .placements(origin, rotation)
                .into_iter()
                .map(BuildOp::Place)
                .collect();
            if let Ok((ids, reward)) = self.execute_construction(&operations) {
                result = Some((ids, origin, blueprint.size(rotation), reward));
                break;
            }
        }
        if let Some((ids, origin, size, mut reward)) = result {
            let count = ids.len();
            self.last_blueprint = ids;
            if let Some(state) = &mut self.streaming {
                state.last_blueprint_keys = self
                    .last_blueprint
                    .iter()
                    .filter_map(|id| state.snapshot.part_key(*id))
                    .collect();
            }
            if matches!(
                blueprint,
                Blueprint::House | Blueprint::Tower | Blueprint::Windmill | Blueprint::Bridge
            ) {
                reward.push_str(&crate::gameplay::reward_text(
                    self.gameplay.record(briccraft_play::Activity::Structure),
                ));
            }
            self.camera.focus(
                [
                    origin[0] as f32 + size[0] as f32 * 0.5,
                    origin[1] as f32 * 0.4 + size[1] as f32 * 0.2,
                    origin[2] as f32 + size[2] as f32 * 0.5,
                ],
                size[0].max(size[2]) as f32 * 1.8 + 12.0,
            );
            self.status = format!(
                "{} assembled from {count} parts. Choose Drive to bring a vehicle to life.{reward}",
                blueprint.name()
            );
        } else {
            self.status =
                "There is not enough clear space for this construction here. Choose another area."
                    .into();
        }
        self.update_pick();
    }
}

pub(crate) fn named(name: &str) -> Option<Blueprint> {
    Some(match name {
        "rover" | "car" => Blueprint::Rover,
        "plane" => Blueprint::Airplane,
        "train" => Blueprint::Locomotive,
        "boat" => Blueprint::Sailboat,
        "house" => Blueprint::House,
        "tower" => Blueprint::Tower,
        "windmill" => Blueprint::Windmill,
        "bridge" => Blueprint::Bridge,
        _ => return None,
    })
}
