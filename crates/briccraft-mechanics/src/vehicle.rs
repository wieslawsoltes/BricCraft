use crate::{AssemblyAssessment, Motor, RailPath, Transmission};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleKind {
    Car,
    Plane,
    Train,
    Boat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VehicleControl {
    pub throttle: f32,
    pub steering: f32,
    pub lift: f32,
    pub roll: f32,
    pub brake: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub kind: VehicleKind,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub speed: f32,
    pub wheel_angle: f32,
    pub engine_rpm: f32,
    pub gear: i8,
    pub mass_kg: f32,
    pub wheel_radius: f32,
    pub wheelbase: f32,
    pub track: f32,
    pub drive_ratio: f32,
    pub motor_count: usize,
    rail_distance: f32,
}

fn approach(value: f32, target: f32, amount: f32) -> f32 {
    value + (target - value).clamp(-amount, amount)
}
fn finite_input(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

impl Vehicle {
    pub fn new(kind: VehicleKind, position: [f32; 3]) -> Self {
        let rail_distance = RailPath::coastal().closest_distance(position);
        Self {
            kind,
            position,
            velocity: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            speed: 0.0,
            wheel_angle: 0.0,
            engine_rpm: 0.0,
            gear: 1,
            mass_kg: 0.08,
            wheel_radius: 1.25,
            wheelbase: 6.0,
            track: 4.0,
            drive_ratio: 1.0,
            motor_count: 1,
            rail_distance,
        }
    }

    /// Configure dynamics from the actual construction, rather than its chosen vehicle label.
    /// Shape-derived shell mass is an approximation; motor torque uses the game's stud-unit scale.
    pub fn configure(&mut self, assembly: &AssemblyAssessment) {
        self.mass_kg = assembly.mass_kg.clamp(0.01, 50.0);
        self.wheel_radius = assembly.wheel_radius.clamp(0.2, 8.0);
        self.wheelbase = assembly.wheelbase.clamp(1.0, 50.0);
        self.track = assembly.track.clamp(1.0, 50.0);
        self.drive_ratio = assembly.drive_ratio.clamp(0.05, 20.0);
        self.motor_count = assembly.motors;
    }

    /// Semi-fixed 120 Hz integration avoids frame-rate-dependent suspension and steering.
    pub fn step(&mut self, dt: f32, control: VehicleControl, terrain: impl Fn(f32, f32) -> f32) {
        if !dt.is_finite()
            || dt <= 0.0
            || self
                .position
                .iter()
                .chain(&self.velocity)
                .any(|v| !v.is_finite())
        {
            return;
        }
        let dt = dt.min(0.2);
        let steps = (dt * 120.0).ceil().max(1.0) as usize;
        let control = VehicleControl {
            throttle: finite_input(control.throttle),
            steering: finite_input(control.steering),
            lift: finite_input(control.lift),
            roll: finite_input(control.roll),
            brake: control.brake,
        };
        for _ in 0..steps {
            self.integrate(dt / steps as f32, control, &terrain);
        }
    }

    fn integrate(&mut self, dt: f32, input: VehicleControl, terrain: &impl Fn(f32, f32) -> f32) {
        let (acceleration, drag, max_speed) = match self.kind {
            VehicleKind::Car => (8.0, 0.22, 20.0),
            VehicleKind::Plane => (5.0, 0.07, 34.0),
            VehicleKind::Train => (4.0, 0.10, 17.0),
            VehicleKind::Boat => (3.5, 0.17, 11.0),
        };
        let ratio = match self.gear {
            2 => 1.8,
            3 => 1.1,
            _ => 3.0,
        };
        let transmission = Transmission {
            gear_ratio: ratio,
            final_drive: self.drive_ratio,
            efficiency: 0.88,
            wheel_radius: self.wheel_radius,
        };
        let shaft_speed = self.speed / self.wheel_radius * ratio * self.drive_ratio;
        let motor = Motor {
            stall_torque: self.motor_count as f32 * 0.6,
            free_speed: 45.0,
            target: input.throttle,
        };
        let drive_acceleration = if self.kind == VehicleKind::Car {
            transmission.wheel_force(motor.torque(shaft_speed)) / (self.mass_kg * 2.5)
        } else {
            let power = if self.kind == VehicleKind::Boat {
                1.0
            } else {
                self.motor_count as f32
            };
            input.throttle * acceleration * (0.08 * power / self.mass_kg).clamp(0.1, 3.0)
        };
        self.speed +=
            (drive_acceleration - drag * self.speed - self.speed * self.speed.abs() * 0.006) * dt;
        if input.brake {
            self.speed = approach(self.speed, 0.0, 24.0 * dt);
        }
        self.speed = self.speed.clamp(-max_speed * 0.35, max_speed);
        self.gear = if self.speed < -0.2 {
            -1
        } else if self.speed < 7.0 {
            1
        } else if self.speed < 13.0 {
            2
        } else {
            3
        };
        self.engine_rpm = transmission.motor_rpm(self.speed).abs();
        self.wheel_angle = (self.wheel_angle + self.speed / self.wheel_radius * dt)
            .rem_euclid(std::f32::consts::TAU);
        if self.kind == VehicleKind::Train {
            self.rail_distance += self.speed * dt;
            let rail = RailPath::sample_coastal(self.rail_distance);
            self.position = rail.position;
            self.yaw = rail.yaw;
            self.velocity = rail.tangent.map(|v| v * self.speed);
            self.pitch = 0.0;
            self.roll = 0.0;
            return;
        }
        let previous = self.position;
        let steer_scale = match self.kind {
            VehicleKind::Car => 0.10,
            VehicleKind::Plane => 0.028,
            VehicleKind::Boat => 0.065,
            _ => 0.0,
        };
        let yaw_rate = if self.kind == VehicleKind::Car {
            self.speed / self.wheelbase * (input.steering * 0.55).tan()
        } else {
            input.steering * self.speed * steer_scale
        };
        self.yaw = (self.yaw + yaw_rate * dt).rem_euclid(std::f32::consts::TAU);
        let direction = [self.yaw.sin(), self.yaw.cos()];
        self.position[0] += direction[0] * self.speed * dt;
        self.position[2] += direction[1] * self.speed * dt;
        let ground = terrain(self.position[0], self.position[2]);
        match self.kind {
            VehicleKind::Car => {
                if ground > previous[1] + 1.6 {
                    self.position[0] = previous[0];
                    self.position[2] = previous[2];
                    self.speed = 0.0;
                }
                let front = terrain(
                    self.position[0] + direction[0] * self.wheelbase * 0.5,
                    self.position[2] + direction[1] * self.wheelbase * 0.5,
                );
                let rear = terrain(
                    self.position[0] - direction[0] * self.wheelbase * 0.5,
                    self.position[2] - direction[1] * self.wheelbase * 0.5,
                );
                let left = terrain(
                    self.position[0] + direction[1] * self.track * 0.5,
                    self.position[2] - direction[0] * self.track * 0.5,
                );
                let right = terrain(
                    self.position[0] - direction[1] * self.track * 0.5,
                    self.position[2] + direction[0] * self.track * 0.5,
                );
                let floor = ground.max((front + rear + left + right) * 0.25);
                self.velocity[1] +=
                    (80.0 * (floor - self.position[1]) - 15.0 * self.velocity[1]) * dt;
                self.position[1] = (self.position[1] + self.velocity[1] * dt).max(ground);
                self.pitch = approach(
                    self.pitch,
                    ((front - rear) / self.wheelbase).atan().clamp(-0.5, 0.5),
                    2.0 * dt,
                );
                self.roll = approach(
                    self.roll,
                    ((right - left) / self.track).atan().clamp(-0.45, 0.45),
                    2.0 * dt,
                );
            }
            VehicleKind::Plane => {
                self.pitch = approach(self.pitch, input.lift * 0.42, 0.55 * dt);
                self.roll = approach(
                    self.roll,
                    -input.steering * 0.55 + input.roll * 0.45,
                    1.3 * dt,
                );
                let lift = self.speed.max(0.0).powi(2) * 0.026;
                self.velocity[1] +=
                    (lift - 3.6 + self.pitch * self.speed * 1.2 - self.velocity[1] * 0.6) * dt;
                self.velocity[1] = self.velocity[1].clamp(-12.0, 12.0);
                self.position[1] += self.velocity[1] * dt;
                if self.position[1] < ground {
                    self.position[1] = ground;
                    self.velocity[1] = 0.0;
                }
            }
            VehicleKind::Boat => {
                let target = briccraft_world::SEA_LEVEL - 0.35;
                if ground > briccraft_world::SEA_LEVEL + 0.5 {
                    self.position[0] = previous[0];
                    self.position[2] = previous[2];
                    self.speed *= 0.92;
                }
                self.velocity[1] +=
                    (35.0 * (target - self.position[1]) - 8.0 * self.velocity[1]) * dt;
                self.position[1] += self.velocity[1] * dt;
                self.roll = approach(self.roll, -input.steering * self.speed * 0.018, 0.5 * dt);
                self.pitch = approach(self.pitch, self.speed * 0.007, 0.5 * dt);
            }
            VehicleKind::Train => {}
        }
        self.velocity[0] = (self.position[0] - previous[0]) / dt;
        self.velocity[2] = (self.position[2] - previous[2]) / dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_mass_and_reduction_change_wheel_acceleration() {
        let mut world = briccraft_world::World::empty(1);
        world
            .spawn_blueprint(briccraft_world::Blueprint::Rover, [20, 0, 20], 0)
            .unwrap();
        let assessment = crate::assess_assembly(&world.instances().cloned().collect::<Vec<_>>());
        let mut normal = Vehicle::new(VehicleKind::Car, [20.0, 0.0, 20.0]);
        normal.configure(&assessment);
        let mut heavy = normal.clone();
        heavy.mass_kg *= 2.0;
        let mut reduced = normal.clone();
        reduced.drive_ratio *= 2.0;
        let input = VehicleControl {
            throttle: 1.0,
            ..Default::default()
        };
        for vehicle in [&mut normal, &mut heavy, &mut reduced] {
            vehicle.step(1.0 / 120.0, input, |_, _| 0.0);
        }
        assert!(heavy.speed < normal.speed * 0.55);
        assert!(reduced.speed > normal.speed * 1.9);
        assert_eq!(normal.wheel_radius, 1.6);
    }
    #[test]
    fn driving_is_frame_rate_independent_and_brakes() {
        let mut a = Vehicle::new(VehicleKind::Car, [20.0, 0.0, 20.0]);
        let mut b = a.clone();
        for _ in 0..120 {
            a.step(
                1.0 / 120.0,
                VehicleControl {
                    throttle: 1.0,
                    ..Default::default()
                },
                |_, _| 0.0,
            );
        }
        for _ in 0..30 {
            b.step(
                1.0 / 30.0,
                VehicleControl {
                    throttle: 1.0,
                    ..Default::default()
                },
                |_, _| 0.0,
            );
        }
        assert!((a.position[2] - b.position[2]).abs() < 0.01);
        for _ in 0..60 {
            a.step(
                1.0 / 60.0,
                VehicleControl {
                    brake: true,
                    ..Default::default()
                },
                |_, _| 0.0,
            );
        }
        assert!(a.speed.abs() < 0.01);
    }
    #[test]
    fn aircraft_can_take_off_and_boat_floats() {
        let mut plane = Vehicle::new(VehicleKind::Plane, [0.0, 0.0, 0.0]);
        for _ in 0..600 {
            plane.step(
                1.0 / 60.0,
                VehicleControl {
                    throttle: 1.0,
                    lift: 0.5,
                    ..Default::default()
                },
                |_, _| 0.0,
            );
        }
        assert!(plane.position[1] > 2.0);
        let mut boat = Vehicle::new(VehicleKind::Boat, [0.0, 0.0, 0.0]);
        for _ in 0..600 {
            boat.step(1.0 / 60.0, VehicleControl::default(), |_, _| 0.0);
        }
        assert!((boat.position[1] - (briccraft_world::SEA_LEVEL - 0.35)).abs() < 0.01);
    }
}
