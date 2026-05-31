//! Error types for streamdown

use thiserror::Error;

/// Main error type for streamdown operations
#[derive(Error, Debug)]
pub enum StreamdownError {
    /// IO error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Parse error during markdown processing
    #[error("Parse error: {0}")]
    Parse(String),

    /// Render error during output generation
    #[error("Render error: {0}")]
    Render(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),
}

/// Result type alias for streamdown operations
pub type Result<T> = std::result::Result<T, StreamdownError>;
