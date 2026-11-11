//! Engineering primitives independent of presentation and platform.
#![forbid(unsafe_code)]
mod assembly;
mod drive;
mod gears;
mod joints;
mod rail;
mod vehicle;

pub use assembly::{assess_assembly, AssemblyAssessment};
pub use gears::{GearMesh, GearTrain, MechanicalError, ShaftId, ShaftState, Transmission};
pub use joints::{
    ackermann_steering, differential, rack_displacement, Joint, JointKind, Motor, SpringDamper,
};
pub use rail::{RailPath, RailSample};
pub use vehicle::{Vehicle, VehicleControl, VehicleKind};
