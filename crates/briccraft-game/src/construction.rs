use crate::Session;
use briccraft_parts::part;
use briccraft_world::{BuildOp, Placement};

impl Session {
    pub fn update_pick(&mut self) {
        if self.active_vehicle.is_some() {
            self.hit = None;
            self.preview = None;
            return;
        }
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
        self.status = match self.execute_construction(&[BuildOp::Place(placement)]) {
            Ok((_, reward)) => format!(
                "Placed {} · R rotates · Ctrl Z undoes.{}",
                part(self.selected_part).map_or("brick", |p| p.name),
                reward
            ),
            Err(error) => error,
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
        let id = hit.id;
        let value = self
            .world
            .instance(id)
            .and_then(|p| briccraft_play::part_cost(p.part_id))
            .unwrap_or(0);
        self.status = match self.execute_construction(&[BuildOp::Remove(id)]) {
            Ok((_, reward)) => {
                if self.gameplay.is_expedition() {
                    format!("Recycled part · +{value} studs. Ctrl Z restores it and repays the refund.{reward}")
                } else {
                    "Part removed. Ctrl Z restores it.".into()
                }
            }
            Err(error) => error,
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
}
