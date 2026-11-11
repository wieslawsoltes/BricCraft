//! Camera coordinates are world units: one stud horizontally, 0.4 per plate.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: [f32; 3],
    /// Radians around the up axis. Zero looks towards positive Z.
    pub yaw: f32,
    /// Radians above the horizontal plane.
    pub pitch: f32,
    pub vertical_fov: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::looking_at([58.0, 17.0, 80.0], [44.0, 7.0, 42.0])
    }
}

impl Camera {
    pub fn looking_at(position: [f32; 3], target: [f32; 3]) -> Self {
        let d = normalize(sub(target, position));
        Self {
            position,
            yaw: d[0].atan2(d[2]),
            pitch: d[1].asin(),
            vertical_fov: 58_f32.to_radians(),
        }
    }

    pub fn forward(self) -> [f32; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        [sy * cp, sp, cy * cp]
    }

    pub fn right(self) -> [f32; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        [cy, 0.0, -sy]
    }

    pub fn up(self) -> [f32; 3] {
        cross(self.forward(), self.right())
    }

    /// `normalized` is the pointer position in 0..1 from the top left.
    pub fn ray(self, normalized: [f32; 2], aspect: f32) -> [f32; 3] {
        let scale = (self.vertical_fov.clamp(0.25, 2.5) * 0.5).tan();
        let sx = (normalized[0] * 2.0 - 1.0) * aspect * scale;
        let sy = (1.0 - normalized[1] * 2.0) * scale;
        let forward = self.forward();
        let right = self.right();
        let up = self.up();
        normalize(std::array::from_fn(|i| {
            forward[i] + right[i] * sx + up[i] * sy
        }))
    }
}

pub(crate) fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    v.map(|x| x / length)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_ray_and_camera_basis_agree() {
        let c = Camera::default();
        let ray = c.ray([0.5, 0.5], 2.0);
        for (a, b) in ray.into_iter().zip(c.forward()) {
            assert!((a - b).abs() < 1e-6);
        }
        let right = c.ray([1.0, 0.5], 1.0);
        assert!(
            right
                .into_iter()
                .zip(c.right())
                .map(|(a, b)| a * b)
                .sum::<f32>()
                > 0.0
        );
        assert!(c.ray([0.5, 0.0], 1.0)[1] > c.forward()[1]);
    }
}

pub(crate) fn rotation_axes(yaw: f32, pitch: f32, roll: f32) -> [[f32; 3]; 3] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = (-pitch).sin_cos();
    let (sr, cr) = roll.sin_cos();
    [
        [cy * cr + sy * sp * sr, cp * sr, -sy * cr + cy * sp * sr],
        [-cy * sr + sy * sp * cr, cp * cr, sy * sr + cy * sp * cr],
        [sy * cp, -sp, cy * cp],
    ]
}

#[cfg(test)]
mod vehicle_tests {
    use super::*;

    #[test]
    fn positive_pitch_raises_the_nose_and_rotation_preserves_lengths() {
        let axes = rotation_axes(0.7, 0.35, -0.2);
        assert!(axes[2][1] > 0.0);
        for axis in axes {
            assert!((axis.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1e-6);
        }
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            assert!(
                axes[a]
                    .iter()
                    .zip(axes[b])
                    .map(|(x, y)| x * y)
                    .sum::<f32>()
                    .abs()
                    < 1e-6
            );
        }
    }
}
