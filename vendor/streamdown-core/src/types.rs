//! Core types for streamdown

use serde::{Deserialize, Serialize};

/// Represents a position in the input stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Byte offset from start
    pub offset: usize,
}

/// Represents a span in the input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

impl Span {
    /// Create a new span from start and end positions
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}
