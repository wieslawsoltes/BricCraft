use crate::{GameMode, Gameplay, PlayError, MAX_STUDS};
use briccraft_parts::{part, Category, Shape};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildQuote {
    pub studs: u64,
    pub affordable: bool,
    pub charged: bool,
}

/// Stable catalog economics: envelope material plus functional component cost.
/// Rotation, color and placement cannot change the purchase/recycling value.
pub fn part_cost(id: u16) -> Option<u64> {
    let part = part(id)?;
    let volume = part.size.into_iter().map(u64::from).product::<u64>();
    let material = volume.div_ceil(3).max(1);
    let function = match part.category {
        Category::Power => match id {
            140 | 141 | 144 => 90,
            142 | 160 => 70,
            _ => 18,
        },
        _ => match part.shape {
            Shape::Wheel => 10,
            Shape::Gear => 12,
            Shape::Axle => 3,
            Shape::Beam => 4,
            Shape::Propeller => 30,
            Shape::Window | Shape::Door => 4,
            _ => 0,
        },
    };
    Some(material + function)
}

pub fn parts_cost(parts: impl IntoIterator<Item = u16>) -> Result<u64, PlayError> {
    parts.into_iter().try_fold(0u64, |sum, id| {
        let value = part_cost(id).ok_or(PlayError::UnknownPart(id))?;
        sum.checked_add(value)
            .filter(|v| *v <= MAX_STUDS)
            .ok_or(PlayError::ResourceLimit)
    })
}

/// Actual catalog value introduced/removed by one atomic world change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeValue {
    pub added: u64,
    pub removed: u64,
}
impl ChangeValue {
    pub const fn inverse(self) -> Self {
        Self {
            added: self.removed,
            removed: self.added,
        }
    }
}

/// Not Clone/Copy/Deserialize: one authorization can be consumed only once.
#[must_use = "Consume this permit only after the corresponding world edit succeeds"]
pub struct ResourcePermit {
    before: u64,
    after: u64,
    revision: u64,
    mode: GameMode,
}

impl Gameplay {
    pub fn quote(&self, studs: u64) -> BuildQuote {
        BuildQuote {
            studs,
            affordable: !self.is_expedition() || studs <= self.studs,
            charged: self.is_expedition(),
        }
    }
    pub fn approve(&self, change: ChangeValue) -> Result<ResourcePermit, PlayError> {
        let after = if self.is_expedition() {
            let available = self
                .studs
                .checked_add(change.removed)
                .ok_or(PlayError::ResourceLimit)?;
            let value =
                available
                    .checked_sub(change.added)
                    .ok_or(PlayError::InsufficientStuds {
                        needed: change.added,
                        available,
                    })?;
            if value > MAX_STUDS {
                return Err(PlayError::ResourceLimit);
            }
            value
        } else {
            self.studs
        };
        Ok(ResourcePermit {
            before: self.studs,
            after,
            revision: self.revision,
            mode: self.mode,
        })
    }
    pub fn commit(&mut self, permit: ResourcePermit) -> Result<(), PlayError> {
        if self.studs != permit.before
            || self.revision != permit.revision
            || self.mode != permit.mode
        {
            return Err(PlayError::StalePermit);
        }
        self.studs = permit.after;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}
