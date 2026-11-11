use crate::Session;
use briccraft_mechanics::{assess_assembly, Vehicle, VehicleKind};
use briccraft_stream::{GlobalPlacement, PartKey};
use briccraft_world::Instance;
use std::rc::Rc;

pub struct ActiveVehicle {
    pub instance_ids: Vec<u32>,
    /// Stable construction-space geometry independent of local World IDs and window origins.
    pub parts: Rc<[Instance]>,
    pub part_keys: Vec<PartKey>,
    pub pivot: [f32; 3],
    pub vehicle: Vehicle,
    /// World heading of the construction before the continuous driving transform is applied.
    pub base_yaw: f32,
    pub half_extents_xz: [f32; 2],
}

impl ActiveVehicle {
    pub fn render_yaw(&self) -> f32 {
        self.vehicle.yaw - self.base_yaw
    }
}

impl Session {
    pub fn activate_vehicle(&mut self, kind: &str) {
        if self.active_vehicle.is_some() {
            return;
        }
        if let Err(error) = self.capture_streamed_edits() {
            self.status = error.to_string();
            return;
        }
        let pointed = self
            .hit
            .as_ref()
            .map(|h| self.world.connected_assembly(h.id, 256))
            .unwrap_or_default();
        let pointed_parts: Vec<_> = pointed
            .iter()
            .filter_map(|id| self.world.instance(*id).cloned())
            .collect();
        let pointed_ready = pointed.len() < 256 && assess_assembly(&pointed_parts).drive_ready;
        let ids = if pointed_ready {
            pointed
        } else {
            self.last_blueprint
                .iter()
                .copied()
                .filter(|id| self.world.instance(*id).is_some())
                .collect::<Vec<_>>()
        };
        if ids.is_empty() {
            self.status =
                "Insert a vehicle blueprint, or point at your connected vehicle construction."
                    .into();
            return;
        }
        let instances: Vec<_> = ids
            .iter()
            .filter_map(|id| self.world.instance(*id).cloned())
            .collect();
        let assessment = assess_assembly(&instances);
        if !assessment.drive_ready {
            self.status = format!(
                "Construction needs attention: {}.",
                assessment.reasons.join("; ")
            );
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
            _ => assessment.suggested_vehicle,
        };
        let ready = match kind {
            VehicleKind::Car | VehicleKind::Train => wheels >= 2,
            VehicleKind::Plane => wings >= 1 && propellers >= 1,
            VehicleKind::Boat => hulls >= 1,
        };
        if self.is_streamed() && kind == VehicleKind::Train {
            self.status = "Powered trains use the coastal railway in Sunlit Archipelago. This open world has no guided railway network yet; the locomotive remains editable here.".into();
            return;
        }
        if !ready {
            self.status="This construction is missing running gear. Cars and trains need wheels; planes need wings and a propeller; boats need a hull.".into();
            return;
        }
        let pivot = [(min[0] + max[0]) * 0.5, min[1], (min[2] + max[2]) * 0.5];
        let base_yaw = assessment.forward_yaw;
        let mut vehicle = Vehicle::new(kind, pivot);
        vehicle.configure(&assessment);
        vehicle.yaw = base_yaw;
        self.active_vehicle = Some(ActiveVehicle {
            part_keys: self.streaming.as_ref().map_or_else(Vec::new, |state| {
                ids.iter()
                    .filter_map(|&id| state.snapshot.part_key(id))
                    .collect()
            }),
            instance_ids: ids,
            parts: instances.into(),
            pivot,
            vehicle,
            base_yaw,
            half_extents_xz: [(max[0] - min[0]) * 0.5, (max[2] - min[2]) * 0.5],
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
        let quarter = (active.render_yaw() / std::f32::consts::FRAC_PI_2).round() as i32;
        let rotation = quarter.rem_euclid(4) as u8;
        let origin = active.vehicle.position;
        let pivot = active.pivot;
        let (sin, cos) = (f32::from(rotation) * std::f32::consts::FRAC_PI_2).sin_cos();
        let mut transformed = Vec::with_capacity(active.parts.len());
        for instance in active.parts.iter() {
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
            transformed.push(Instance {
                id: instance.id,
                part_id: instance.part_id,
                color: instance.color,
                position: p,
                rotation: r,
            });
        }
        if self.streaming.is_some() {
            let keys = active.part_keys.clone();
            let result = (|| {
                let state = self.streaming.as_mut().expect("streamed parking");
                let placements = transformed
                    .iter()
                    .map(|instance| {
                        GlobalPlacement::from_local(state.snapshot.origin, instance.placement())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                state.stream.move_parts(&keys, &placements)?;
                state.last_blueprint_keys = keys;
                Ok::<(), briccraft_stream::StreamError>(())
            })();
            match result {
                Ok(()) => {
                    self.active_vehicle = None;
                    self.input.clear();
                    match self.refresh_streamed_world() {
                        Ok(()) => {
                            self.status = "Vehicle parked. Its original parts are saved at this global location.".into();
                            self.camera.set_mode(crate::CameraMode::Orbit);
                        }
                        Err(error) => {
                            self.status =
                                format!("Vehicle saved; refreshing the local view failed: {error}")
                        }
                    }
                }
                Err(error) => {
                    self.status =
                        format!("Cannot park here: {error}. Move to a clear, level surface.")
                }
            }
            return;
        }
        match self.history.transform(&mut self.world, &transformed) {
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
