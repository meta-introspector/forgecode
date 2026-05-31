//! Streamdown ANSI
//!
//! This crate provides ANSI escape code utilities and terminal
//! handling for streamdown's rendering output.
//!
//! # Overview
//!
//! - [`codes`] - ANSI escape code constants
//! - [`style`] - Style pairs for toggleable formatting
//! - [`color`] - HSV/RGB color manipulation
//! - [`utils`] - Text processing utilities (visible length, ANSI stripping, etc.)
//! - [`sanitize`] - Security utilities for safe terminal output
//!
//! # Example
//!
//! ```
//! use streamdown_ansi::{codes, style, utils};
//!
//! // Apply bold formatting
//! let text = format!("{}bold text{}", style::BOLD.0, style::BOLD.1);
//!
//! // Get visible length (ignoring ANSI codes)
//! let visible_len = utils::visible_length(&text);
//! assert_eq!(visible_len, 9); // "bold text"
//! ```

pub mod codes;
pub mod color;
pub mod sanitize;
pub mod style;
pub mod utils;

pub use codes::*;
pub use color::*;
pub use sanitize::*;
pub use style::*;
pub use utils::*;
