use crate::World;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub id: u32,
    pub position: [f32; 3],
    pub cell: [i32; 3],
    pub normal: [i32; 3],
    pub distance: f32,
}

impl World {
    /// Amanatides-Woo traversal. Ray distances remain in world units despite plate-scaled Y cells.
    pub fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    ) -> Option<RayHit> {
        if origin.into_iter().chain(direction).any(|v| !v.is_finite())
            || !max_distance.is_finite()
            || max_distance <= 0.0
        {
            return None;
        }
        let length = direction.into_iter().map(|v| v * v).sum::<f32>().sqrt();
        if length < 1e-7 {
            return None;
        }
        let dir = direction.map(|v| v / length);
        let cell_size = [1.0, 0.4, 1.0];
        let extent = [
            self.size[0] as f32,
            self.size[1] as f32 * 0.4,
            self.size[2] as f32,
        ];
        let mut enter = 0.0f32;
        let mut exit = max_distance;
        let mut normal = [0; 3];
        for axis in 0..3 {
            if dir[axis].abs() < 1e-8 {
                if origin[axis] < 0.0 || origin[axis] >= extent[axis] {
                    return None;
                }
            } else {
                let a = -origin[axis] / dir[axis];
                let b = (extent[axis] - origin[axis]) / dir[axis];
                let near = a.min(b);
                if near > enter {
                    enter = near;
                    normal = [0; 3];
                    normal[axis] = if dir[axis] > 0.0 { -1 } else { 1 };
                }
                exit = exit.min(a.max(b));
            }
        }
        if enter > exit || exit < 0.0 {
            return None;
        }
        let t = enter.max(0.0) + 1e-4;
        let mut cell = std::array::from_fn(|axis| {
            ((origin[axis] + dir[axis] * t) / cell_size[axis]).floor() as i32
        });
        let step: [i32; 3] = dir.map(|v| {
            if v > 0.0 {
                1
            } else if v < 0.0 {
                -1
            } else {
                0
            }
        });
        let delta: [f32; 3] = std::array::from_fn(|axis| {
            if step[axis] == 0 {
                f32::INFINITY
            } else {
                cell_size[axis] / dir[axis].abs()
            }
        });
        let mut next: [f32; 3] = std::array::from_fn(|axis| {
            if step[axis] == 0 {
                f32::INFINITY
            } else {
                let boundary = (cell[axis] + i32::from(step[axis] > 0)) as f32 * cell_size[axis];
                (boundary - origin[axis]) / dir[axis]
            }
        });
        let mut distance = enter.max(0.0);
        loop {
            self.index(cell)?;
            if let Some(instance) = self.cell(cell) {
                return Some(RayHit {
                    id: instance.id,
                    cell,
                    normal,
                    distance,
                    position: std::array::from_fn(|axis| origin[axis] + dir[axis] * distance),
                });
            }
            let axis = if next[0] < next[1] && next[0] < next[2] {
                0
            } else if next[1] < next[2] {
                1
            } else {
                2
            };
            distance = next[axis];
            if distance > exit {
                return None;
            }
            cell[axis] += step[axis];
            next[axis] += delta[axis];
            normal = [0; 3];
            normal[axis] = -step[axis];
        }
    }
}
