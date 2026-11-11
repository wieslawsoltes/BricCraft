use std::fmt;

pub type ImportResult<T> = Result<T, ImportError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    InvalidInput(String),
    Unsupported(String),
    BudgetExceeded {
        resource: &'static str,
        limit: usize,
    },
    Cancelled,
    EmptyGeometry,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "Invalid import: {message}"),
            Self::Unsupported(message) => write!(f, "Unsupported import: {message}"),
            Self::BudgetExceeded { resource, limit } => write!(f, "Import exceeds the {resource} budget ({limit}); use a smaller area or coarser scale"),
            Self::Cancelled => f.write_str("Import cancelled"),
            Self::EmptyGeometry => f.write_str("No supported geometry was found in the input"),
        }
    }
}
impl std::error::Error for ImportError {}
impl From<serde_json::Error> for ImportError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidInput(value.to_string())
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> ImportError {
    ImportError::InvalidInput(message.into())
}

impl From<briccraft_terrain::TerrainError> for ImportError {
    fn from(value: briccraft_terrain::TerrainError) -> Self {
        match value {
            briccraft_terrain::TerrainError::BudgetExceeded { resource, limit } => {
                Self::BudgetExceeded { resource, limit }
            }
            briccraft_terrain::TerrainError::Cancelled => Self::Cancelled,
            other => Self::InvalidInput(other.to_string()),
        }
    }
}
