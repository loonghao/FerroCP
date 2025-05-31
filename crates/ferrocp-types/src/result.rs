//! Result type alias for FerroCP operations

use crate::Error;

/// Result type alias for FerroCP operations
pub type Result<T> = std::result::Result<T, Error>;
