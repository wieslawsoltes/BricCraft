//! Deterministic expedition rules without a renderer, UI or world implementation.
//! Hosts approve a resource change, commit their world transaction, then consume
//! its single-use permit. Failed world edits never consume that permit or resources.
#![forbid(unsafe_code)]
mod economy;
mod objectives;
mod state;

pub use economy::{part_cost, parts_cost, BuildQuote, ChangeValue, ResourcePermit};
pub use objectives::{Activity, ObjectiveView, Reward, TravelKind};
pub use state::{GameMode, Gameplay, PlayError, STARTER_STUDS};

pub(crate) const MAX_STUDS: u64 = 1_000_000_000_000;
pub(crate) const MAX_COUNTER: u64 = 1_000_000_000_000;
