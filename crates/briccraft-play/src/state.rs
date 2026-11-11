use crate::{objectives::Statistics, MAX_COUNTER, MAX_STUDS};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const STARTER_STUDS: u64 = 1200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    #[default]
    Creative,
    Expedition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gameplay {
    version: u8,
    pub(crate) mode: GameMode,
    started: bool,
    pub(crate) studs: u64,
    pub(crate) statistics: Statistics,
    pub(crate) claimed: u16,
    #[serde(skip)]
    pub(crate) revision: u64,
}
impl Default for Gameplay {
    fn default() -> Self {
        Self {
            version: 1,
            mode: GameMode::Creative,
            started: false,
            studs: STARTER_STUDS,
            statistics: Statistics::default(),
            claimed: 0,
            revision: 0,
        }
    }
}
impl Gameplay {
    pub fn mode(&self) -> GameMode {
        self.mode
    }
    pub fn is_expedition(&self) -> bool {
        self.mode == GameMode::Expedition
    }
    pub fn started(&self) -> bool {
        self.started
    }
    pub fn studs(&self) -> u64 {
        self.studs
    }
    /// Preserves the wallet and completed objectives. Hosts clear construction
    /// history at this boundary so free and paid transactions are never replayed
    /// under the opposite mode's accounting policy.
    pub fn set_expedition(&mut self, enabled: bool) {
        self.mode = if enabled {
            GameMode::Expedition
        } else {
            GameMode::Creative
        };
        self.started |= enabled;
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn validate(&self) -> Result<(), PlayError> {
        if self.version != 1
            || self.studs > MAX_STUDS
            || self.claimed & !0xff != 0
            || self.statistics.values().iter().any(|v| *v > MAX_COUNTER)
            || (self.is_expedition() && !self.started)
        {
            return Err(PlayError::InvalidState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayError {
    UnknownPart(u16),
    InsufficientStuds { needed: u64, available: u64 },
    ResourceLimit,
    StalePermit,
    InvalidState,
}
impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPart(id)=>write!(f,"Unknown catalog part {id}."),
            Self::InsufficientStuds{needed,available}=>write!(f,"Need {} more studs ({needed} required, {available} available). Recycle parts or complete an expedition objective.",needed-available),
            Self::ResourceLimit=>f.write_str("This change exceeds the construction resource limit."),
            Self::StalePermit=>f.write_str("The construction wallet changed after this operation was approved."),
            Self::InvalidState=>f.write_str("Invalid or unsupported expedition state."),
        }
    }
}
impl std::error::Error for PlayError {}
