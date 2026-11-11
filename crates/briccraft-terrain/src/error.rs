use std::fmt;

pub type TerrainResult<T> = Result<T, TerrainError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainError {
    InvalidInput(String),
    MissingElevation(String),
    BudgetExceeded {
        resource: &'static str,
        limit: usize,
    },
    Cancelled,
}
impl fmt::Display for TerrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "Invalid terrain: {message}"),
            Self::MissingElevation(message) => write!(f, "Missing elevation: {message}"),
            Self::BudgetExceeded { resource, limit } => write!(f, "Terrain exceeds the {resource} budget ({limit}); use a coarser scale or smaller area"),
            Self::Cancelled => f.write_str("Terrain generation cancelled"),
        }
    }
}
impl std::error::Error for TerrainError {}

pub(crate) fn invalid(message: impl Into<String>) -> TerrainError {
    TerrainError::InvalidInput(message.into())
}
pub(crate) fn limit(resource: &'static str, actual: usize, maximum: usize) -> TerrainResult<()> {
    if actual > maximum {
        Err(TerrainError::BudgetExceeded {
            resource,
            limit: maximum,
        })
    } else {
        Ok(())
    }
}
