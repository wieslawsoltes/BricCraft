use crate::Session;
use briccraft_mechanics::{Vehicle, VehicleKind};
use briccraft_world::{BuildOp, Placement};

pub struct ActiveVehicle {
    pub instance_ids: Vec<u32>,
    pub pivot: [f32; 3],
    pub vehicle: Vehicle,
}

impl Session {
    pub fn activate_vehicle(&mut self, kind: &str) {
        if self.active_vehicle.is_some() {
            return;
        }
        let ids: Vec<u32> = self
            .last_blueprint
            .iter()
            .copied()
            .filter(|id| self.world.instance(*id).is_some())
            .collect();
        let ids = if ids.is_empty() {
            self.hit
                .as_ref()
                .map(|h| self.world.connected_assembly(h.id, 256))
                .unwrap_or_default()
        } else {
            ids
        };
        if ids.is_empty() {
            self.status =
                "Insert a vehicle blueprint, or point at your connected vehicle construction."
                    .into();
            return;
        }
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        let mut wheels = 0;
        let mut propellers = 0;
        let mut wings = 0;
        let mut hulls = 0;
        for id in &ids {
            if let Some(instance) = self.world.instance(*id) {
                let p = briccraft_parts::part(instance.part_id).unwrap();
                wheels += usize::from(p.shape == briccraft_parts::Shape::Wheel);
                propellers += usize::from(p.shape == briccraft_parts::Shape::Propeller);
                wings += usize::from(p.shape == briccraft_parts::Shape::Wing);
                hulls += usize::from(p.name.to_ascii_lowercase().contains("hull"));
                let pos = instance.world_position();
                let size = instance.size();
                for axis in 0..3 {
                    min[axis] = min[axis].min(pos[axis]);
                    max[axis] = max[axis]
                        .max(pos[axis] + f32::from(size[axis]) * if axis == 1 { 0.4 } else { 1.0 });
                }
            }
        }
        if ids.len() >= 256 || (max[0] - min[0]) > 32.0 || (max[2] - min[2]) > 32.0 {
            self.status="This assembly is connected to the landscape. Build the vehicle as a separate construction or insert a vehicle blueprint.".into();
            return;
        }
        let kind = match kind {
            "car" => VehicleKind::Car,
            "plane" => VehicleKind::Plane,
            "train" => VehicleKind::Train,
            "boat" => VehicleKind::Boat,
            _ if wings > 0 && propellers > 0 => VehicleKind::Plane,
            _ if hulls > 0 => VehicleKind::Boat,
            _ => VehicleKind::Car,
        };
        let ready = match kind {
            VehicleKind::Car | VehicleKind::Train => wheels >= 2,
            VehicleKind::Plane => wings >= 1 && propellers >= 1,
            VehicleKind::Boat => hulls >= 1,
        };
        if !ready {
            self.status="This construction is missing running gear. Cars and trains need wheels; planes need wings and a propeller; boats need a hull.".into();
            return;
        }
        let pivot = [(min[0] + max[0]) * 0.5, min[1], (min[2] + max[2]) * 0.5];
        let vehicle = Vehicle::new(kind, pivot);
        self.active_vehicle = Some(ActiveVehicle {
            instance_ids: ids,
            pivot,
            vehicle,
        });
        self.input.clear();
        self.preview = None;
        self.status =
            "Vehicle active · W / S throttle · A / D steer · E / Q lift · B brake · Escape parks."
                .into();
    }
    pub fn park_vehicle(&mut self) {
        let Some(active) = &self.active_vehicle else {
            return;
        };
        let quarter = (active.vehicle.yaw / std::f32::consts::FRAC_PI_2).round() as i32;
        let rotation = quarter.rem_euclid(4) as u8;
        let origin = active.vehicle.position;
        let pivot = active.pivot;
        let (sin, cos) = (f32::from(rotation) * std::f32::consts::FRAC_PI_2).sin_cos();
        let mut operations: Vec<_> = active
            .instance_ids
            .iter()
            .map(|id| BuildOp::Remove(*id))
            .collect();
        for id in &active.instance_ids {
            let Some(instance) = self.world.instance(*id) else {
                continue;
            };
            let size = instance.size();
            let x = instance.position[0] as f32 + f32::from(size[0]) * 0.5 - pivot[0];
            let z = instance.position[2] as f32 + f32::from(size[2]) * 0.5 - pivot[2];
            let r = (instance.rotation + rotation) % 4;
            let next_size = briccraft_parts::part(instance.part_id)
                .unwrap()
                .rotated_size(r);
            let p = [
                (origin[0] + x * cos + z * sin - f32::from(next_size[0]) * 0.5).round() as i32,
                (instance.position[1] as f32 + (origin[1] - pivot[1]) / 0.4).round() as i32,
                (origin[2] - x * sin + z * cos - f32::from(next_size[2]) * 0.5).round() as i32,
            ];
            operations.push(BuildOp::Place(Placement::new(
                instance.part_id,
                instance.color,
                p,
                r,
            )));
        }
        match self.history.execute(&mut self.world, &operations) {
            Ok(ids) => {
                self.last_blueprint = ids;
                self.active_vehicle = None;
                self.input.clear();
                self.status = "Vehicle parked. Its parts can be edited again.".into();
                self.camera.set_mode(crate::CameraMode::Orbit);
            }
            Err(e) => {
                self.status = format!("Cannot park here: {e}. Move to a clear, level surface.")
            }
        }
    }
}
