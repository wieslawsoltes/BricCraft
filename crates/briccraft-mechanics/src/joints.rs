#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    Fixed,
    Hinge,
    Axle,
    Slider,
    Ball,
}

#[derive(Debug, Clone, Copy)]
pub struct Joint {
    pub kind: JointKind,
    pub coordinate: f32,
    pub velocity: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub damping: f32,
}
impl Joint {
    /// One generalized degree of freedom for a hinge, axle, or slider.
    pub fn step(&mut self, dt: f32, force: f32, inertia: f32) {
        if !dt.is_finite()
            || dt <= 0.0
            || !force.is_finite()
            || !inertia.is_finite()
            || inertia <= 0.0
        {
            return;
        }
        if self.kind == JointKind::Fixed {
            self.velocity = 0.0;
            return;
        }
        let dt = dt.min(0.05);
        self.velocity += (force - self.damping.max(0.0) * self.velocity) / inertia * dt;
        self.coordinate += self.velocity * dt;
        if self.coordinate < self.minimum {
            self.coordinate = self.minimum;
            self.velocity = self.velocity.max(0.0);
        }
        if self.coordinate > self.maximum {
            self.coordinate = self.maximum;
            self.velocity = self.velocity.min(0.0);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Motor {
    pub stall_torque: f32,
    pub free_speed: f32,
    pub target: f32,
}
impl Motor {
    pub fn torque(self, speed: f32) -> f32 {
        if self.free_speed <= 0.0 || !self.free_speed.is_finite() || !speed.is_finite() {
            return 0.0;
        }
        (self.target.clamp(-1.0, 1.0) - speed / self.free_speed).clamp(-1.0, 1.0)
            * self.stall_torque.max(0.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpringDamper {
    pub stiffness: f32,
    pub damping: f32,
    pub rest_length: f32,
    pub travel: f32,
}
impl SpringDamper {
    pub fn force(self, length: f32, velocity: f32) -> f32 {
        let compression = (self.rest_length - length).clamp(0.0, self.travel.max(0.0));
        (self.stiffness.max(0.0) * compression - self.damping.max(0.0) * velocity).max(0.0)
    }
}

/// Ideal open differential: carrier angular speed is the mean of the two axle speeds.
pub fn differential(carrier_speed: f32, left_speed: f32, input_torque: f32) -> (f32, f32, f32) {
    (
        2.0 * carrier_speed - left_speed,
        input_torque * 0.5,
        input_torque * 0.5,
    )
}
pub fn rack_displacement(pinion_angle: f32, pitch_radius: f32) -> f32 {
    pinion_angle * pitch_radius
}

/// Inner/outer steering angles (radians), with sign preserved for left and right turns.
pub fn ackermann_steering(central_angle: f32, wheelbase: f32, track: f32) -> [f32; 2] {
    if central_angle.abs() < 1e-5 || wheelbase <= 0.0 {
        return [0.0; 2];
    }
    let radius = wheelbase / central_angle.abs().min(1.2).tan();
    let inner =
        (wheelbase / (radius - track.max(0.0) * 0.5).max(0.01)).atan() * central_angle.signum();
    let outer = (wheelbase / (radius + track.max(0.0) * 0.5)).atan() * central_angle.signum();
    if central_angle > 0.0 {
        [inner, outer]
    } else {
        [outer, inner]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn differential_and_steering_preserve_geometry() {
        let (right, left_torque, right_torque) = differential(10.0, 8.0, 4.0);
        assert_eq!(right, 12.0);
        assert_eq!(left_torque + right_torque, 4.0);
        let [inside, outside] = ackermann_steering(0.4, 6.0, 4.0);
        assert!(inside > outside && outside > 0.0);
        assert_eq!(rack_displacement(2.0, 3.0), 6.0);
    }
}
