use crate::grid::{Grid, EPSILON, SKIN};
use crate::recovery::recover;
use crate::sweep::{horizontal, vertical, HorizontalMove};
use crate::{CharacterConfig, CollisionGrid, ConfigError};
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterInput {
    /// World-space X/Z direction; length is clamped to one before applying speed.
    pub direction: Vec2,
    pub speed: f32,
    /// Held state. Only a new press while grounded starts a jump.
    pub jump: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterMotion {
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub hit_ceiling: bool,
    pub stepped: bool,
    pub recovered: bool,
}

/// Runtime-only movement state. Position remains authoritative in the caller.
#[derive(Clone, Debug)]
pub struct CharacterController {
    config: CharacterConfig,
    velocity: Vec3,
    grounded: bool,
    jump_held: bool,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self::new(CharacterConfig::default()).expect("valid default character")
    }
}

impl CharacterController {
    pub fn new(config: CharacterConfig) -> Result<Self, ConfigError> {
        Ok(Self {
            config: config.validate()?,
            velocity: Vec3::ZERO,
            grounded: false,
            jump_held: false,
        })
    }

    pub fn config(&self) -> &CharacterConfig {
        &self.config
    }

    pub fn grounded(&self) -> bool {
        self.grounded
    }

    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    pub fn reset(&mut self) {
        self.velocity = Vec3::ZERO;
        self.grounded = false;
        self.jump_held = false;
    }

    /// Advances at most 100 ms, split into at most twelve 120 Hz integration
    /// steps. Each translation is also swept continuously against occupancy.
    /// A caller's fixed simulation loop should pass its real step duration.
    pub fn advance<G: CollisionGrid + ?Sized>(
        &mut self,
        position: Vec3,
        input: CharacterInput,
        dt: f32,
        world: &G,
    ) -> CharacterMotion {
        let mut result = CharacterMotion {
            position,
            velocity: self.velocity,
            grounded: self.grounded,
            ..CharacterMotion::default()
        };
        let Some(grid) = Grid::new(world) else {
            return result;
        };
        let (mut feet, recovered) = recover(&grid, position, &self.config);
        result.recovered = recovered;
        if recovered {
            self.reset();
        }
        if !dt.is_finite() || dt <= 0.0 {
            result.position = feet;
            result.velocity = self.velocity;
            result.grounded = self.grounded;
            return result;
        }
        let dt = dt.min(0.1);
        let count = (dt * 120.0).ceil().clamp(1.0, 12.0) as usize;
        let step = dt / count as f32;
        let direction = if input.direction.is_finite() {
            input.direction.clamp_length_max(1.0)
        } else {
            Vec2::ZERO
        };
        let speed = if input.speed.is_finite() {
            input.speed.clamp(0.0, self.config.max_speed)
        } else {
            0.0
        };
        let desired = direction * speed;
        let mut jump = input.jump && !self.jump_held;
        self.jump_held = input.jump;
        for _ in 0..count {
            let (snap, contact) = vertical(
                &grid,
                &self.config,
                feet,
                -self.config.ground_snap.max(SKIN * 2.0),
            );
            self.grounded =
                self.velocity.y <= 0.0 && contact && (snap.abs() <= SKIN * 3.0 || self.grounded);
            if self.grounded {
                feet.y += snap;
            }
            if jump && self.grounded {
                self.velocity.y = self.config.jump_speed;
                self.grounded = false;
            }
            jump = false;

            let acceleration = if self.grounded {
                self.config.ground_acceleration
            } else {
                self.config.air_acceleration
            };
            let mut planar = Vec2::new(self.velocity.x, self.velocity.z);
            planar = planar.lerp(desired, 1.0 - (-acceleration * step).exp());
            let delta = planar * step;
            let mut movement = horizontal(&grid, &self.config, feet, delta);
            if movement.contacts > 0 && self.grounded && self.config.step_height > 0.0 {
                if let Some(stepped) = self.try_step(&grid, feet, delta, &movement) {
                    movement = stepped;
                    result.stepped = true;
                }
            }
            feet = movement.feet;
            for normal in &movement.normals[..movement.contacts] {
                planar -= *normal * planar.dot(*normal).min(0.0);
            }
            self.velocity.x = planar.x;
            self.velocity.z = planar.y;
            self.velocity.y =
                (self.velocity.y - self.config.gravity * step).max(-self.config.terminal_speed);
            let (dy, hit) = vertical(&grid, &self.config, feet, self.velocity.y * step);
            feet.y += dy;
            if hit {
                result.hit_ceiling |= self.velocity.y > 0.0;
                self.grounded = self.velocity.y < 0.0;
                self.velocity.y = 0.0;
            } else {
                self.grounded = false;
            }
        }
        result.position = feet;
        result.velocity = self.velocity;
        result.grounded = self.grounded;
        result
    }

    fn try_step<G: CollisionGrid + ?Sized>(
        &self,
        grid: &Grid<'_, G>,
        feet: Vec3,
        delta: Vec2,
        ordinary: &HorizontalMove,
    ) -> Option<HorizontalMove> {
        let (rise, _) = vertical(grid, &self.config, feet, self.config.step_height);
        if rise <= SKIN {
            return None;
        }
        let mut raised = horizontal(grid, &self.config, feet + Vec3::Y * rise, delta);
        let (drop, ground) = vertical(
            grid,
            &self.config,
            raised.feet,
            -(rise + self.config.ground_snap),
        );
        if !ground {
            return None;
        }
        raised.feet.y += drop;
        let height = raised.feet.y - feet.y;
        let gained = Vec2::new(
            raised.feet.x - ordinary.feet.x,
            raised.feet.z - ordinary.feet.z,
        )
        .dot(delta);
        if height > EPSILON
            && height <= self.config.step_height + SKIN * 2.0
            && gained > EPSILON * delta.length().max(EPSILON)
            && grid.clear(raised.feet, &self.config)
        {
            Some(raised)
        } else {
            None
        }
    }
}
