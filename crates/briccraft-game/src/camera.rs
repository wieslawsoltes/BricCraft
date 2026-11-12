//! A continuous camera with separate orbit, walking and free-flight state.
use crate::Input;
use briccraft_character::{CharacterController, CharacterInput, CollisionGrid};
use briccraft_world::World;
use glam::{IVec3, Vec2, Vec3};
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
    character: CharacterController,
    #[serde(skip)]
    walking: bool,
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
            character: CharacterController::default(),
            walking: false,
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
        // Walk translations must go through collision; editor panning remains
        // independent in Orbit and Fly modes.
        if self.mode == CameraMode::Walk {
            return;
        }
        let scale = self.distance.max(5.0) * 0.0016;
        let offset = self.right() * (-dx * scale) + self.up() * (dy * scale);
        self.target += offset;
        self.position += offset;
    }
    pub fn zoom(&mut self, delta: f32) {
        if self.mode == CameraMode::Orbit {
            self.distance = (self.distance * (delta * 0.0015).exp()).clamp(2.0, 160.0);
            self.update_orbit();
        } else if self.mode == CameraMode::Fly {
            self.position += self.forward() * (-delta * 0.025);
        } else {
            // A walking zoom changes the lens, never moves the body through a wall.
            self.vertical_fov = (self.vertical_fov * (delta * 0.0005).exp())
                .clamp(40.0_f32.to_radians(), 90.0_f32.to_radians());
        }
    }
    pub fn set_mode(&mut self, mode: CameraMode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.velocity = Vec3::ZERO;
        self.character.reset();
        self.walking = false;
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
        self.walking = false;
        self.character.reset();
        self.velocity = Vec3::ZERO;
        self.target = Vec3::from_array(target);
        self.distance = distance.clamp(8.0, 160.0);
        self.pitch = -0.40;
        self.update_orbit();
    }
    pub fn grounded(&self) -> bool {
        self.mode == CameraMode::Walk && self.walking && self.character.grounded()
    }

    /// Walk against the actual occupied volume, including ceilings and bridges.
    /// Position is the authoritative eye location, so streamed-origin rebases and
    /// restored cameras need no extra controller translation or persistent fields.
    pub fn tick_world(&mut self, dt: f32, input: &Input, world: &World) {
        if !dt.is_finite() {
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        if self.mode == CameraMode::Walk {
            if !self.walking {
                self.character.reset();
                self.velocity = Vec3::ZERO;
                self.walking = true;
            }
            let [x, _, z] = input.axes();
            let forward = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
            let direction = self.right() * x + forward * z;
            let eye_height = self.character.config().eye_height;
            let motion = self.character.advance(
                self.position - Vec3::Y * eye_height,
                CharacterInput {
                    direction: Vec2::new(direction.x, direction.z),
                    speed: if input.fast { 18.0 } else { 7.0 },
                    jump: input.up,
                },
                dt,
                &WorldCollision(world),
            );
            self.position = motion.position + Vec3::Y * eye_height;
            return;
        }
        if self.walking {
            self.character.reset();
            self.velocity = Vec3::ZERO;
            self.walking = false;
        }
        self.tick_free(dt, input);
    }

    fn tick_free(&mut self, dt: f32, input: &Input) {
        let [x, y, z] = input.axes();
        let speed = if input.fast { 18.0 } else { 7.0 };
        let flat = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let forward = if self.mode == CameraMode::Fly {
            self.forward()
        } else {
            flat
        };
        let desired = (self.right() * x + forward * z + Vec3::Y * y).normalize_or_zero() * speed;
        self.velocity = self.velocity.lerp(desired, 1.0 - (-12.0 * dt).exp());
        self.position += self.velocity * dt;
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

pub(crate) struct WorldCollision<'a>(pub(crate) &'a World);

impl CollisionGrid for WorldCollision<'_> {
    fn dimensions(&self) -> IVec3 {
        let [x, y, z] = self.0.dimensions();
        IVec3::new(i32::from(x), i32::from(y), i32::from(z))
    }

    fn cell_size(&self) -> Vec3 {
        Vec3::new(1.0, 0.4, 1.0)
    }

    fn is_solid(&self, cell: IVec3) -> bool {
        self.0
            .index(cell.to_array())
            .is_some_and(|index| self.0.occupancy()[index] != 0)
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
        let world = World::empty(1);
        for _ in 0..60 {
            a.tick_world(1.0 / 60.0, &input, &world);
        }
        input.right = true;
        for _ in 0..60 {
            b.tick_world(1.0 / 60.0, &input, &world);
        }
        assert!(((a.position - origin).length() - (b.position - origin).length()).abs() < 0.001);
    }

    #[test]
    fn walk_recovers_inside_bricks_but_preserves_an_open_room_under_a_roof() {
        use briccraft_world::Placement;
        let mut world = World::empty(1);
        world.place(Placement::new(0, 1, [10, 0, 10], 0)).unwrap();
        let mut camera = CameraRig::default();
        camera.set_mode(CameraMode::Walk);
        camera.position = Vec3::new(10.5, 2.3, 10.5);
        camera.tick_world(1.0 / 60.0, &Input::default(), &world);
        assert!((camera.position.y - 3.4).abs() < 0.001);
        assert!(camera.grounded());
        world.place(Placement::new(0, 1, [10, 12, 10], 0)).unwrap();
        let before = camera.position;
        camera.tick_world(1.0 / 60.0, &Input::default(), &world);
        assert!((camera.position - before).length() < 0.001);
    }

    #[test]
    fn saved_camera_schema_is_unchanged_and_runtime_state_is_rebuilt() {
        let legacy = r#"{"mode":"Walk","position":[10.5,2.2,10.5],"target":[0,0,0],"yaw":0,"pitch":0,"distance":12,"vertical_fov":1}"#;
        let mut camera: CameraRig = serde_json::from_str(legacy).unwrap();
        let value = serde_json::to_value(&camera).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 7);
        camera.tick_world(1.0 / 60.0, &Input::default(), &World::empty(1));
        assert!(camera.grounded());
        assert!(camera.position.is_finite());
    }

    #[test]
    fn walking_pan_and_zoom_cannot_teleport_the_body() {
        let mut camera = CameraRig::default();
        camera.set_mode(CameraMode::Walk);
        let position = camera.position;
        camera.pan(500.0, -500.0);
        camera.zoom(-600.0);
        assert_eq!(camera.position, position);
        assert!(camera.vertical_fov >= 40.0_f32.to_radians());
    }
}
