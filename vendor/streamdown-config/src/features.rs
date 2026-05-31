//! Feature flags configuration.
//!
//! This module contains the `FeaturesConfig` struct which holds
//! all boolean feature flags and related settings.

use serde::{Deserialize, Serialize};

/// Feature flags configuration.
///
/// Controls which features are enabled in streamdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FeaturesConfig {
    /// Enable space-indented code blocks (4 spaces = code).
    /// Default: false (only fenced code blocks are recognized)
    #[serde(default)]
    pub code_spaces: bool,

    /// Enable clipboard integration for code blocks.
    /// Default: true
    #[serde(default = "default_true")]
    pub clipboard: bool,

    /// Enable debug logging.
    /// Default: false
    #[serde(default)]
    pub logging: bool,

    /// Timeout in seconds for streaming operations.
    /// Default: 0.1
    #[serde(default = "default_timeout")]
    pub timeout: f64,

    /// Save brace matching state.
    /// Default: true
    #[serde(default = "default_true")]
    pub savebrace: bool,

    /// Enable image rendering (where supported).
    /// Default: true
    #[serde(default = "default_true")]
    pub images: bool,

    /// Enable link rendering with OSC 8 hyperlinks.
    /// Default: true
    #[serde(default = "default_true")]
    pub links: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            code_spaces: false,
            clipboard: true,
            logging: false,
            timeout: 0.1,
            savebrace: true,
            images: true,
            links: true,
        }
    }
}

impl FeaturesConfig {
    /// Merge another FeaturesConfig into this one.
    ///
    /// All fields are copied from `other` since they're all
    /// simple values with no "unset" state in TOML.
    pub fn merge(&mut self, other: &FeaturesConfig) {
        // For a proper merge, we'd need Option<T> fields.
        // Since TOML doesn't distinguish "not set" from "set to default",
        // we just copy all values from other.
        // In practice, this means the override file needs only the
        // values the user wants to change, and we parse a partial
        // config for the override.
        self.code_spaces = other.code_spaces;
        self.clipboard = other.clipboard;
        self.logging = other.logging;
        self.timeout = other.timeout;
        self.savebrace = other.savebrace;
        self.images = other.images;
        self.links = other.links;
    }

    /// Create a new FeaturesConfig with all features enabled.
    pub fn all_enabled() -> Self {
        Self {
            code_spaces: true,
            clipboard: true,
            logging: true,
            timeout: 0.1,
            savebrace: true,
            images: true,
            links: true,
        }
    }

    /// Create a new FeaturesConfig with all features disabled.
    pub fn all_disabled() -> Self {
        Self {
            code_spaces: false,
            clipboard: false,
            logging: false,
            timeout: 0.0,
            savebrace: false,
            images: false,
            links: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> f64 {
    0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let features = FeaturesConfig::default();
        assert!(!features.code_spaces);
        assert!(features.clipboard);
        assert!(!features.logging);
        assert!((features.timeout - 0.1).abs() < f64::EPSILON);
        assert!(features.savebrace);
        assert!(features.images);
        assert!(features.links);
    }

    #[test]
    fn test_serde_pascal_case() {
        let toml_str = r#"
            CodeSpaces = true
            Clipboard = false
            Logging = true
            Timeout = 0.5
            Savebrace = false
            Images = false
            Links = false
        "#;

        let features: FeaturesConfig = toml::from_str(toml_str).unwrap();
        assert!(features.code_spaces);
        assert!(!features.clipboard);
        assert!(features.logging);
        assert!((features.timeout - 0.5).abs() < f64::EPSILON);
        assert!(!features.savebrace);
        assert!(!features.images);
        assert!(!features.links);
    }

    #[test]
    fn test_all_enabled() {
        let features = FeaturesConfig::all_enabled();
        assert!(features.code_spaces);
        assert!(features.clipboard);
        assert!(features.logging);
    }

    #[test]
    fn test_all_disabled() {
        let features = FeaturesConfig::all_disabled();
        assert!(!features.code_spaces);
        assert!(!features.clipboard);
        assert!(!features.logging);
    }
}
