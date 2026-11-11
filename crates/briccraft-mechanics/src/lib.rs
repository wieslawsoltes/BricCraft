//! Engineering primitives independent of presentation and platform.
#![forbid(unsafe_code)]
mod assembly;
mod gears;
mod joints;
mod rail;
mod vehicle;

pub use assembly::{AssemblyAssessment, assess_assembly};
pub use gears::{GearMesh, GearTrain, MechanicalError, ShaftId, ShaftState, Transmission};
pub use joints::{
    Joint, JointKind, Motor, SpringDamper, ackermann_steering, differential, rack_displacement,
};
pub use rail::{RailPath, RailSample};
pub use vehicle::{Vehicle, VehicleControl, VehicleKind};
