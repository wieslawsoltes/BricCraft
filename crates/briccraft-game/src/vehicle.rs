use crate::Session;
use briccraft_mechanics::Vehicle;
mod workshop;
use briccraft_stream::{GlobalPlacement, PartKey};
use briccraft_world::Instance;
use std::rc::Rc;
pub use workshop::WorkshopStatus;

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
        let prepared = match self.prepare_vehicle(kind) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let workshop::PreparedVehicle {
            ids,
            instances,
            assessment,
            min,
            max,
            kind,
        } = prepared;
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
        let reward =
            crate::gameplay::reward_text(self.gameplay.record(briccraft_play::Activity::Activated));
        self.status = format!("Vehicle active · W / S throttle · A / D steer · E / Q lift · B brake · Escape parks.{reward}");
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
