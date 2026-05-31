//! Style configuration.
//!
//! This module contains the `StyleConfig` struct which holds
//! all visual styling settings including HSV color multipliers.

use serde::{Deserialize, Serialize};

/// HSV multiplier for color transformations.
///
/// These multipliers are applied to base HSV values to create
/// derived colors for different UI elements.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub struct HsvMultiplier {
    /// Hue multiplier (typically 1.0 to preserve hue)
    pub h: f64,
    /// Saturation multiplier
    pub s: f64,
    /// Value (brightness) multiplier
    pub v: f64,
}

impl Default for HsvMultiplier {
    fn default() -> Self {
        Self {
            h: 1.0,
            s: 1.0,
            v: 1.0,
        }
    }
}

impl HsvMultiplier {
    /// Create a new HSV multiplier.
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        Self { h, s, v }
    }

    /// Create the "Dark" style multiplier (default values).
    pub fn dark() -> Self {
        Self::new(1.00, 1.50, 0.25)
    }

    /// Create the "Mid" style multiplier (default values).
    pub fn mid() -> Self {
        Self::new(1.00, 1.00, 0.50)
    }

    /// Create the "Symbol" style multiplier (default values).
    pub fn symbol() -> Self {
        Self::new(1.00, 1.00, 1.50)
    }

    /// Create the "Head" style multiplier (default values).
    pub fn head() -> Self {
        Self::new(1.00, 1.00, 1.75)
    }

    /// Create the "Grey" style multiplier (default values).
    pub fn grey() -> Self {
        Self::new(1.00, 0.25, 1.37)
    }

    /// Create the "Bright" style multiplier (default values).
    pub fn bright() -> Self {
        Self::new(1.00, 0.60, 2.00)
    }
}

/// Style configuration.
///
/// Controls visual styling including margins, indentation,
/// colors, and code block appearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StyleConfig {
    /// Left margin in characters.
    /// Default: 2
    #[serde(default = "default_margin")]
    pub margin: usize,

    /// List item indentation in characters.
    /// Default: 2
    #[serde(default = "default_list_indent")]
    pub list_indent: usize,

    /// Enable pretty padding for code blocks.
    /// Default: true
    #[serde(default = "default_true")]
    pub pretty_pad: bool,

    /// Enable broken line indicators.
    /// Default: true
    #[serde(default = "default_true")]
    pub pretty_broken: bool,

    /// Terminal width override (0 = auto-detect).
    /// Default: 0
    #[serde(default)]
    pub width: usize,

    /// Base HSV color values [H, S, V].
    /// H is in 0.0..1.0 range (will be scaled to 360)
    /// S and V are in 0.0..1.0 range
    /// Default: [0.8, 0.5, 0.5]
    #[serde(default = "default_hsv", rename = "HSV")]
    pub hsv: [f64; 3],

    /// Dark color multiplier (for backgrounds).
    #[serde(default = "HsvMultiplier::dark")]
    pub dark: HsvMultiplier,

    /// Mid color multiplier (for secondary elements).
    #[serde(default = "HsvMultiplier::mid")]
    pub mid: HsvMultiplier,

    /// Symbol color multiplier (for special characters).
    #[serde(default = "HsvMultiplier::symbol")]
    pub symbol: HsvMultiplier,

    /// Head color multiplier (for headers).
    #[serde(default = "HsvMultiplier::head")]
    pub head: HsvMultiplier,

    /// Grey color multiplier (for muted text).
    #[serde(default = "HsvMultiplier::grey")]
    pub grey: HsvMultiplier,

    /// Bright color multiplier (for emphasis).
    #[serde(default = "HsvMultiplier::bright")]
    pub bright: HsvMultiplier,

    /// Syntax highlighting theme name.
    /// Default: "native"
    #[serde(default = "default_syntax")]
    pub syntax: String,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            margin: 2,
            list_indent: 2,
            pretty_pad: true,
            pretty_broken: true,
            width: 0,
            hsv: [0.8, 0.5, 0.5],
            dark: HsvMultiplier::dark(),
            mid: HsvMultiplier::mid(),
            symbol: HsvMultiplier::symbol(),
            head: HsvMultiplier::head(),
            grey: HsvMultiplier::grey(),
            bright: HsvMultiplier::bright(),
            syntax: "native".to_string(),
        }
    }
}

impl StyleConfig {
    /// Merge another StyleConfig into this one.
    pub fn merge(&mut self, other: &StyleConfig) {
        self.margin = other.margin;
        self.list_indent = other.list_indent;
        self.pretty_pad = other.pretty_pad;
        self.pretty_broken = other.pretty_broken;
        self.width = other.width;
        self.hsv = other.hsv;
        self.dark = other.dark;
        self.mid = other.mid;
        self.symbol = other.symbol;
        self.head = other.head;
        self.grey = other.grey;
        self.bright = other.bright;
        self.syntax.clone_from(&other.syntax);
    }

    /// Get the base HSV values as (H, S, V) tuple.
    ///
    /// H is scaled to 0..360 range for color calculations.
    pub fn base_hsv(&self) -> (f64, f64, f64) {
        // H in config is 0..1, convert to 0..360
        (self.hsv[0] * 360.0, self.hsv[1], self.hsv[2])
    }

    /// Get effective width (auto-detect if 0).
    pub fn effective_width(&self) -> usize {
        if self.width == 0 {
            // Try to get terminal width, fallback to 80
            crossterm::terminal::size()
                .map(|(w, _)| w as usize)
                .unwrap_or(80)
        } else {
            self.width
        }
    }
}

fn default_margin() -> usize {
    2
}

fn default_list_indent() -> usize {
    2
}

fn default_true() -> bool {
    true
}

fn default_hsv() -> [f64; 3] {
    [0.8, 0.5, 0.5]
}

fn default_syntax() -> String {
    "native".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = StyleConfig::default();
        assert_eq!(style.margin, 2);
        assert_eq!(style.list_indent, 2);
        assert!(style.pretty_pad);
        assert!(style.pretty_broken);
        assert_eq!(style.width, 0);
        assert_eq!(style.hsv, [0.8, 0.5, 0.5]);
        assert_eq!(style.syntax, "native");
    }

    #[test]
    fn test_hsv_multiplier_defaults() {
        let dark = HsvMultiplier::dark();
        assert!((dark.h - 1.0).abs() < f64::EPSILON);
        assert!((dark.s - 1.5).abs() < f64::EPSILON);
        assert!((dark.v - 0.25).abs() < f64::EPSILON);

        let bright = HsvMultiplier::bright();
        assert!((bright.v - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_serde_pascal_case() {
        let toml_str = r#"
            Margin = 4
            ListIndent = 3
            PrettyPad = false
            PrettyBroken = false
            Width = 100
            HSV = [0.5, 0.6, 0.7]
            Dark = { H = 1.0, S = 2.0, V = 0.5 }
            Syntax = "monokai"
        "#;

        let style: StyleConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(style.margin, 4);
        assert_eq!(style.list_indent, 3);
        assert!(!style.pretty_pad);
        assert_eq!(style.width, 100);
        assert_eq!(style.hsv, [0.5, 0.6, 0.7]);
        assert!((style.dark.s - 2.0).abs() < f64::EPSILON);
        assert_eq!(style.syntax, "monokai");
    }

    #[test]
    fn test_base_hsv() {
        let style = StyleConfig::default();
        let (h, s, v) = style.base_hsv();
        assert!((h - 288.0).abs() < f64::EPSILON); // 0.8 * 360
        assert!((s - 0.5).abs() < f64::EPSILON);
        assert!((v - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merge() {
        let mut base = StyleConfig::default();
        let other = StyleConfig {
            margin: 5,
            ..Default::default()
        };

        base.merge(&other);
        assert_eq!(base.margin, 5);
    }
}
