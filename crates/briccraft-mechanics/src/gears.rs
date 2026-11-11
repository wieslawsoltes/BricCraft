use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaftId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct GearMesh {
    pub driver: ShaftId,
    pub driven: ShaftId,
    pub driver_teeth: u16,
    pub driven_teeth: u16,
    pub efficiency: f32,
    /// External gears reverse direction; a belt, chain, or internal ring preserves direction.
    pub reverses: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaftState {
    pub angular_velocity: f32,
    pub torque: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MechanicalError {
    ZeroTeeth,
    InvalidEfficiency,
    InvalidInput,
    LockedLoop(ShaftId),
}

#[derive(Debug, Default)]
pub struct GearTrain {
    meshes: Vec<GearMesh>,
}

impl GearTrain {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn connect(&mut self, mesh: GearMesh) -> Result<(), MechanicalError> {
        if mesh.driver_teeth == 0 || mesh.driven_teeth == 0 {
            return Err(MechanicalError::ZeroTeeth);
        }
        if !mesh.efficiency.is_finite() || !(0.0..=1.0).contains(&mesh.efficiency) {
            return Err(MechanicalError::InvalidEfficiency);
        }
        self.meshes.push(mesh);
        Ok(())
    }
    /// Propagate ideal shaft speeds. Conflicting closed gear loops are mechanically locked.
    /// Torque is the available no-branch output torque, not a load-sharing network solver.
    pub fn solve(
        &self,
        input: ShaftId,
        state: ShaftState,
    ) -> Result<BTreeMap<ShaftId, ShaftState>, MechanicalError> {
        if !state.angular_velocity.is_finite() || !state.torque.is_finite() {
            return Err(MechanicalError::InvalidInput);
        }
        let mut states = BTreeMap::from([(input, state)]);
        let mut ratios = BTreeMap::from([(input, 1.0f32)]);
        let mut queue = VecDeque::from([input]);
        while let Some(current) = queue.pop_front() {
            for mesh in &self.meshes {
                let (next, ratio) = if mesh.driver == current {
                    (
                        mesh.driven,
                        mesh.driver_teeth as f32 / mesh.driven_teeth as f32,
                    )
                } else if mesh.driven == current {
                    (
                        mesh.driver,
                        mesh.driven_teeth as f32 / mesh.driver_teeth as f32,
                    )
                } else {
                    continue;
                };
                let signed = if mesh.reverses { -ratio } else { ratio };
                let total = ratios[&current] * signed;
                if let Some(existing) = ratios.get(&next) {
                    if (existing - total).abs() > 1e-4 * existing.abs().max(total.abs()).max(1.0) {
                        return Err(MechanicalError::LockedLoop(next));
                    }
                    continue;
                }
                let parent = states[&current];
                ratios.insert(next, total);
                states.insert(
                    next,
                    ShaftState {
                        angular_velocity: parent.angular_velocity * signed,
                        torque: parent.torque / signed * mesh.efficiency,
                    },
                );
                queue.push_back(next);
            }
        }
        Ok(states)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transmission {
    pub gear_ratio: f32,
    pub final_drive: f32,
    pub efficiency: f32,
    pub wheel_radius: f32,
}
impl Default for Transmission {
    fn default() -> Self {
        Self {
            gear_ratio: 3.0,
            final_drive: 2.5,
            efficiency: 0.88,
            wheel_radius: 1.25,
        }
    }
}
impl Transmission {
    pub fn wheel_force(self, motor_torque: f32) -> f32 {
        if !self.wheel_radius.is_finite() || self.wheel_radius <= 0.0 {
            return 0.0;
        }
        motor_torque * self.gear_ratio * self.final_drive * self.efficiency.clamp(0.0, 1.0)
            / self.wheel_radius
    }
    pub fn motor_rpm(self, speed: f32) -> f32 {
        if !self.wheel_radius.is_finite() || self.wheel_radius <= 0.0 {
            return 0.0;
        }
        speed / self.wheel_radius * self.gear_ratio * self.final_drive * 60.0
            / std::f32::consts::TAU
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mesh(a: u32, b: u32, ta: u16, tb: u16) -> GearMesh {
        GearMesh {
            driver: ShaftId(a),
            driven: ShaftId(b),
            driver_teeth: ta,
            driven_teeth: tb,
            efficiency: 1.0,
            reverses: true,
        }
    }
    #[test]
    fn gear_reduction_preserves_power() {
        let mut train = GearTrain::new();
        train.connect(mesh(0, 1, 8, 24)).unwrap();
        let state = train
            .solve(
                ShaftId(0),
                ShaftState {
                    angular_velocity: 30.0,
                    torque: 2.0,
                },
            )
            .unwrap();
        assert!((state[&ShaftId(1)].angular_velocity + 10.0).abs() < 1e-5);
        assert!((state[&ShaftId(1)].torque + 6.0).abs() < 1e-5);
        assert!(
            (state[&ShaftId(1)].torque * state[&ShaftId(1)].angular_velocity - 60.0).abs() < 1e-4
        );
    }
    #[test]
    fn odd_external_gear_loop_is_locked_even_at_rest() {
        let mut train = GearTrain::new();
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            train.connect(mesh(a, b, 16, 16)).unwrap();
        }
        assert!(matches!(
            train.solve(
                ShaftId(0),
                ShaftState {
                    angular_velocity: 0.0,
                    torque: 1.0
                }
            ),
            Err(MechanicalError::LockedLoop(_))
        ));
    }
}
