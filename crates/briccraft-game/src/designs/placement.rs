use crate::{CameraMode, Session};
use briccraft_world::BuildOp;

impl Session {
    /// Place at the picked face. Without a hit, use the build focus or a point eight studs ahead.
    /// No alternate location is searched: a collision rejects the entire copy at this anchor.
    pub fn place_design(&mut self, id: u32) -> Result<Vec<u32>, String> {
        self.design_edit_allowed()?;
        let size = self
            .design_library
            .get(id)
            .ok_or_else(|| format!("Saved design {id} no longer exists."))?
            .rotated_dimensions(self.rotation)
            .map_err(|e| e.to_string())?;
        let origin = self.design_anchor(size)?;
        self.place_design_at(id, origin, self.rotation)
    }

    /// Exact local cells make scripted building, editor tooling and tests use the same atomic path.
    pub fn place_design_at(
        &mut self,
        id: u32,
        origin: [i32; 3],
        rotation: u8,
    ) -> Result<Vec<u32>, String> {
        self.design_edit_allowed()?;
        let design = self
            .design_library
            .get(id)
            .ok_or_else(|| format!("Saved design {id} no longer exists."))?;
        let name = design.name.clone();
        let operations: Vec<_> = design
            .placements(origin, rotation)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(BuildOp::Place)
            .collect();
        let (ids, reward) = self.execute_construction(&operations)?;
        self.last_blueprint = ids.clone();
        if let Some(state) = &mut self.streaming {
            state.last_blueprint_keys = ids
                .iter()
                .filter_map(|&id| state.snapshot.part_key(id))
                .collect();
        }
        self.status = format!("Placed {name} · {} parts at [{}, {}, {}], rotated {}°. Ctrl Z undoes the whole copy.{reward}",
            ids.len(), origin[0], origin[1], origin[2], u16::from(rotation) * 90);
        self.update_pick();
        Ok(ids)
    }

    fn design_anchor(&self, size: [u8; 3]) -> Result<[i32; 3], String> {
        if let Some(hit) = self.hit {
            if let Some(instance) = self.world.instance(hit.id) {
                let mut origin = hit.cell;
                for axis in 0..3 {
                    if hit.normal[axis] > 0 {
                        origin[axis] = instance.position[axis] + i32::from(instance.size()[axis]);
                    } else if hit.normal[axis] < 0 {
                        origin[axis] = instance.position[axis] - i32::from(size[axis]);
                    }
                }
                return Ok(origin);
            }
        }
        let anchor = if self.camera.mode == CameraMode::Orbit {
            self.camera.target
        } else {
            self.camera.position
                + glam::Vec3::new(
                    self.camera.yaw.sin() * 8.0,
                    0.0,
                    self.camera.yaw.cos() * 8.0,
                )
        };
        if !anchor.is_finite() {
            return Err("Choose a valid construction anchor.".into());
        }
        let x = anchor.x.floor() as i32;
        let z = anchor.z.floor() as i32;
        let width = i32::from(size[0]);
        let depth = i32::from(size[2]);
        let dimensions = self.world.dimensions();
        if x < 0
            || z < 0
            || x.checked_add(width)
                .is_none_or(|end| end > i32::from(dimensions[0]))
            || z.checked_add(depth)
                .is_none_or(|end| end > i32::from(dimensions[2]))
        {
            return Err("The design does not fit at this anchor. Move the build focus into clear loaded space.".into());
        }
        let mut top = 0.0_f32;
        for px in x..x + width {
            for pz in z..z + depth {
                top = top.max(self.world.surface_height(px as f32 + 0.5, pz as f32 + 0.5));
            }
        }
        Ok([x, (top / 0.4).round() as i32, z])
    }
}
