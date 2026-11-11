//! A continuous camera with separate orbit, walking and free-flight state.
use crate::Input;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum CameraMode {
    #[default]
    Orbit,
    Walk,
    Fly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraRig {
    pub mode: CameraMode,
    pub position: Vec3,
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub vertical_fov: f32,
    #[serde(skip)]
    velocity: Vec3,
    #[serde(skip)]
    vertical_velocity: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        let mut rig = Self {
            mode: CameraMode::Orbit,
            position: Vec3::ZERO,
            target: Vec3::new(48.0, 5.5, 46.0),
            yaw: 0.72,
            pitch: -0.45,
            distance: 68.0,
            vertical_fov: 58.0_f32.to_radians(),
            velocity: Vec3::ZERO,
            vertical_velocity: 0.0,
        };
        rig.update_orbit();
        rig
    }
}

impl CameraRig {
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }
    pub fn up(&self) -> Vec3 {
        self.forward().cross(self.right()).normalize()
    }
    pub fn ray(&self, normalized: [f32; 2], aspect: f32) -> [f32; 3] {
        let height = (self.vertical_fov * 0.5).tan();
        (self.forward()
            + self.right() * normalized[0] * height * aspect
            + self.up() * normalized[1] * height)
            .normalize()
            .to_array()
    }
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw + dx * 0.005).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch - dy * 0.004).clamp(-1.48, 1.48);
        if self.mode == CameraMode::Orbit {
            self.update_orbit();
        }
    }
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let scale = self.distance.max(5.0) * 0.0016;
        let offset = self.right() * (-dx * scale) + self.up() * (dy * scale);
        self.target += offset;
        self.position += offset;
    }
    pub fn zoom(&mut self, delta: f32) {
        if self.mode == CameraMode::Orbit {
            self.distance = (self.distance * (delta * 0.0015).exp()).clamp(2.0, 160.0);
            self.update_orbit();
        } else {
            self.position += self.forward() * (-delta * 0.025);
        }
    }
    pub fn set_mode(&mut self, mode: CameraMode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.velocity = Vec3::ZERO;
        self.vertical_velocity = 0.0;
        if mode == CameraMode::Orbit {
            self.target = self.position + self.forward() * self.distance.min(30.0);
            self.update_orbit();
        }
    }
    fn update_orbit(&mut self) {
        self.position = self.target - self.forward() * self.distance;
    }
    pub fn focus(&mut self, target: [f32; 3], distance: f32) {
        self.mode = CameraMode::Orbit;
        self.target = Vec3::from_array(target);
        self.distance = distance.clamp(8.0, 160.0);
        self.pitch = -0.40;
        self.update_orbit();
    }
    pub fn tick(&mut self, dt: f32, input: &Input, floor: impl Fn(f32, f32) -> f32) {
        let dt = dt.clamp(0.0, 0.05);
        let [x, y, z] = input.axes();
        let speed = if input.fast { 18.0 } else { 7.0 };
        let flat = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let forward = if self.mode == CameraMode::Fly {
            self.forward()
        } else {
            flat
        };
        let desired = (self.right() * x
            + forward * z
            + Vec3::Y
                * if self.mode == CameraMode::Walk {
                    0.0
                } else {
                    y
                })
        .normalize_or_zero()
            * speed;
        self.velocity = self.velocity.lerp(desired, 1.0 - (-12.0 * dt).exp());
        let mut next = self.position + self.velocity * dt;
        if self.mode == CameraMode::Walk {
            let old_floor = floor(self.position.x, self.position.z) + 2.2;
            let next_floor = floor(next.x, next.z) + 2.2;
            if next_floor - self.position.y > 0.65 {
                next.x = self.position.x;
                next.z = self.position.z;
            }
            if self.position.y <= old_floor + 0.05 && input.up {
                self.vertical_velocity = 7.0;
            }
            self.vertical_velocity -= 18.0 * dt;
            next.y =
                (self.position.y + self.vertical_velocity * dt).max(floor(next.x, next.z) + 2.2);
            if next.y <= floor(next.x, next.z) + 2.21 {
                self.vertical_velocity = self.vertical_velocity.max(0.0);
            }
        }
        self.position = next;
        if self.mode == CameraMode::Orbit {
            self.target += self.velocity * dt;
            self.update_orbit();
        }
    }
    pub fn follow(&mut self, position: [f32; 3], yaw: f32, dt: f32) {
        let target = Vec3::from_array(position) + Vec3::Y * 2.0;
        let desired = target - Vec3::new(yaw.sin(), 0.0, yaw.cos()) * 13.0 + Vec3::Y * 6.0;
        self.position = self.position.lerp(desired, 1.0 - (-5.0 * dt).exp());
        let direction = (target - self.position).normalize();
        self.yaw = direction.x.atan2(direction.z);
        self.pitch = direction.y.asin();
        self.target = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ray_center_matches_forward_and_axes_are_orthogonal() {
        let camera = CameraRig::default();
        assert!((Vec3::from_array(camera.ray([0.0, 0.0], 1.8)) - camera.forward()).length() < 1e-5);
        assert!(camera.forward().dot(camera.right()).abs() < 1e-5);
        assert!(camera.forward().dot(camera.up()).abs() < 1e-5);
    }
    #[test]
    fn zoom_preserves_the_orbit_target() {
        let mut camera = CameraRig::default();
        let target = camera.target;
        camera.zoom(-120.0);
        camera.orbit(80.0, -30.0);
        assert!((camera.position + camera.forward() * camera.distance - target).length() < 1e-4);
    }
    #[test]
    fn input_has_no_diagonal_speed_bonus() {
        let mut a = CameraRig::default();
        let mut b = a.clone();
        a.mode = CameraMode::Fly;
        b.mode = CameraMode::Fly;
        let origin = a.position;
        let mut input = Input {
            forward: true,
            ..Default::default()
        };
        for _ in 0..60 {
            a.tick(1.0 / 60.0, &input, |_, _| 0.0);
        }
        input.right = true;
        for _ in 0..60 {
            b.tick(1.0 / 60.0, &input, |_, _| 0.0);
        }
        assert!(((a.position - origin).length() - (b.position - origin).length()).abs() < 0.001);
    }
}
