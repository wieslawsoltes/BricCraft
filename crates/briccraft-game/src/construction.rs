use crate::Session;
use briccraft_parts::part;
use briccraft_world::{Blueprint, BuildOp, Placement};

impl Session {
    pub fn update_pick(&mut self) {
        let cursor = if self.camera.mode == crate::CameraMode::Orbit {
            self.cursor
        } else {
            [0.0, 0.0]
        };
        let ray = self.camera.ray(cursor, self.aspect);
        self.hit = self
            .world
            .raycast(self.camera.position.to_array(), ray, 250.0);
        self.preview = self.hit.as_ref().map(|hit| {
            let size =
                part(self.selected_part).map_or([1, 1, 1], |p| p.rotated_size(self.rotation));
            let mut cell = hit.cell;
            if let Some(instance) = self.world.instance(hit.id) {
                let instance_size = instance.size();
                for axis in 0..3 {
                    if hit.normal[axis] > 0 {
                        cell[axis] = instance.position[axis] + i32::from(instance_size[axis]);
                    } else if hit.normal[axis] < 0 {
                        cell[axis] = instance.position[axis] - i32::from(size[axis]);
                    }
                }
            }
            Placement {
                part_id: self.selected_part,
                color: self.color,
                position: cell,
                rotation: self.rotation,
            }
        });
    }
    pub fn place(&mut self) {
        if self.active_vehicle.is_some() {
            self.status = "Park the vehicle to edit the world.".into();
            return;
        }
        self.update_pick();
        let Some(placement) = self.preview else {
            self.status = "Point at a brick face to attach a part.".into();
            return;
        };
        self.status = match self
            .history
            .execute(&mut self.world, &[BuildOp::Place(placement)])
        {
            Ok(_) => format!(
                "Placed {} · R rotates · Ctrl Z undoes.",
                part(self.selected_part).map_or("brick", |p| p.name)
            ),
            Err(e) => e.to_string(),
        };
        self.update_pick();
    }
    pub fn remove(&mut self) {
        if self.active_vehicle.is_some() {
            self.status = "Park the vehicle to edit the world.".into();
            return;
        }
        self.update_pick();
        let Some(hit) = &self.hit else {
            return;
        };
        self.status = match self
            .history
            .execute(&mut self.world, &[BuildOp::Remove(hit.id)])
        {
            Ok(_) => "Part removed. Ctrl Z restores it.".into(),
            Err(e) => e.to_string(),
        };
        self.update_pick();
    }
    pub fn eyedropper(&mut self) {
        if let Some(hit) = &self.hit {
            if let Some(instance) = self.world.instance(hit.id) {
                self.selected_part = instance.part_id;
                self.color = instance.color;
                self.rotation = instance.rotation;
                self.status = format!(
                    "Picked {} and its color.",
                    part(instance.part_id).map_or("part", |p| p.name)
                );
            }
        }
    }
    pub fn spawn_blueprint(&mut self, name: &str) {
        if self.active_vehicle.is_some() {
            self.status = "Park your vehicle before inserting another construction.".into();
            return;
        }
        let blueprint = match name {
            "rover" | "car" => Blueprint::Rover,
            "plane" => Blueprint::Airplane,
            "train" => Blueprint::Locomotive,
            "boat" => Blueprint::Sailboat,
            "house" => Blueprint::House,
            "tower" => Blueprint::Tower,
            "windmill" => Blueprint::Windmill,
            "bridge" => Blueprint::Bridge,
            _ => return,
        };
        let center = self
            .hit
            .as_ref()
            .map_or([48, 18, 45], |h| [h.cell[0], h.cell[1] + 1, h.cell[2]]);
        // Search the nearby buildable area; a template is committed atomically, never partly inserted.
        let mut result = None;
        'search: for radius in [0_i32, 4, 8, 12, 18, 24, 32] {
            for (dx, dz) in [
                (radius, 0),
                (-radius, 0),
                (0, radius),
                (0, -radius),
                (radius, radius),
                (-radius, -radius),
            ] {
                let x = (center[0] + dx).clamp(4, 80);
                let z = (center[2] + dz).clamp(4, 80);
                let y =
                    (self.world.surface_height(x as f32 + 0.5, z as f32 + 0.5) / 0.4).ceil() as i32;
                let placements = blueprint.placements([x, y, z], self.rotation);
                let operations: Vec<_> = placements.into_iter().map(BuildOp::Place).collect();
                if let Ok(ids) = self.history.execute(&mut self.world, &operations) {
                    result = Some(ids);
                    break 'search;
                }
            }
        }
        if let Some(ids) = result {
            let count = ids.len();
            self.last_blueprint = ids;
            self.status = format!(
                "{} assembled from {count} parts. Choose Drive to bring a vehicle to life.",
                blueprint.name()
            );
        } else {
            self.status =
                "There is not enough clear space here for this construction. Choose another area."
                    .into();
        }
        self.update_pick();
    }
}
