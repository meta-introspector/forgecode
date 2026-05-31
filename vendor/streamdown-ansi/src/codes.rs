//! ANSI escape code constants.
//!
//! This module provides all the raw ANSI escape sequences used
//! for terminal formatting and colors.

/// Escape sequence prefix for 24-bit foreground color.
/// Usage: `format!("{}r;g;bm", FG)` where r, g, b are 0-255.
pub const FG: &str = "\x1b[38;2;";

/// Escape sequence prefix for 24-bit background color.
/// Usage: `format!("{}r;g;bm", BG)` where r, g, b are 0-255.
pub const BG: &str = "\x1b[48;2;";

/// Reset all attributes (colors and formatting).
pub const RESET: &str = "\x1b[0m";

/// Reset foreground color to default.
pub const FGRESET: &str = "\x1b[39m";

/// Reset all formatting (underline, italic, bold) but keep colors.
pub const FORMATRESET: &str = "\x1b[24;23;22m";

/// Reset background color to default.
pub const BGRESET: &str = "\x1b[49m";

/// Bold on.
pub const BOLD_ON: &str = "\x1b[1m";

/// Bold off (normal intensity).
pub const BOLD_OFF: &str = "\x1b[22m";

/// Underline on.
pub const UNDERLINE_ON: &str = "\x1b[4m";

/// Underline off.
pub const UNDERLINE_OFF: &str = "\x1b[24m";

/// Italic on.
pub const ITALIC_ON: &str = "\x1b[3m";

/// Italic off.
pub const ITALIC_OFF: &str = "\x1b[23m";

/// Strikeout/strikethrough on.
pub const STRIKEOUT_ON: &str = "\x1b[9m";

/// Strikeout/strikethrough off.
pub const STRIKEOUT_OFF: &str = "\x1b[29m";

/// Dim/faint text on.
pub const DIM_ON: &str = "\x1b[2m";

/// Dim/faint text off.
pub const DIM_OFF: &str = "\x1b[22m";

/// Reverse video on.
pub const REVERSE_ON: &str = "\x1b[7m";

/// Reverse video off.
pub const REVERSE_OFF: &str = "\x1b[27m";

/// OSC 8 hyperlink start. Usage: `format!("{}url\x1b\\", LINK_START)`
pub const LINK_START: &str = "\x1b]8;;";

/// OSC 8 hyperlink end.
pub const LINK_END: &str = "\x1b]8;;\x1b\\";

/// Clear from cursor to end of line.
pub const CLEAR_LINE: &str = "\x1b[K";

/// Move cursor up one line.
pub const CURSOR_UP: &str = "\x1b[A";

/// Move cursor down one line.
pub const CURSOR_DOWN: &str = "\x1b[B";

/// Move cursor to beginning of line.
pub const CURSOR_HOME: &str = "\x1b[G";

/// Save cursor position.
pub const CURSOR_SAVE: &str = "\x1b[s";

/// Restore cursor position.
pub const CURSOR_RESTORE: &str = "\x1b[u";

/// Hide cursor.
pub const CURSOR_HIDE: &str = "\x1b[?25l";

/// Show cursor.
pub const CURSOR_SHOW: &str = "\x1b[?25h";

/// Clear entire screen.
pub const CLEAR_SCREEN: &str = "\x1b[2J";

/// Move cursor to position. Usage: `format!("\x1b[{};{}H", row, col)`
pub const CURSOR_POSITION: &str = "\x1b[H";

/// Superscript digit Unicode code points.
/// Index corresponds to digit (0-9).
pub const SUPER: [u32; 10] = [
    0x2070, // ⁰
    0x00B9, // ¹
    0x00B2, // ²
    0x00B3, // ³
    0x2074, // ⁴
    0x2075, // ⁵
    0x2076, // ⁶
    0x2077, // ⁷
    0x2078, // ⁸
    0x2079, // ⁹
];

/// Convert a digit (0-9) to its superscript Unicode character.
///
/// # Arguments
///
/// * `digit` - A digit from 0 to 9
///
/// # Returns
///
/// The superscript character, or the original digit character if invalid.
///
/// # Example
///
/// ```
/// use streamdown_ansi::codes::digit_to_superscript;
/// assert_eq!(digit_to_superscript(2), '²');
/// assert_eq!(digit_to_superscript(0), '⁰');
/// ```
pub fn digit_to_superscript(digit: u8) -> char {
    if digit <= 9 {
        char::from_u32(SUPER[digit as usize]).unwrap_or((b'0' + digit) as char)
    } else {
        (b'0' + digit) as char
    }
}

/// Convert a number to superscript string.
///
/// # Arguments
///
/// * `num` - The number to convert
///
/// # Returns
///
/// A string of superscript characters.
///
/// # Example
///
/// ```
/// use streamdown_ansi::codes::number_to_superscript;
/// assert_eq!(number_to_superscript(42), "⁴²");
/// assert_eq!(number_to_superscript(123), "¹²³");
/// ```
pub fn number_to_superscript(num: usize) -> String {
    num.to_string()
        .chars()
        .map(|c| {
            let digit = c.to_digit(10).unwrap_or(0) as u8;
            digit_to_superscript(digit)
        })
        .collect()
}

/// Create a foreground color escape sequence.
///
/// # Arguments
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Example
///
/// ```
/// use streamdown_ansi::codes::fg_color;
/// let red = fg_color(255, 0, 0);
/// assert_eq!(red, "\x1b[38;2;255;0;0m");
/// ```
pub fn fg_color(r: u8, g: u8, b: u8) -> String {
    format!("{}{}m", FG, rgb_string(r, g, b))
}

/// Create a background color escape sequence.
///
/// # Arguments
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Example
///
/// ```
/// use streamdown_ansi::codes::bg_color;
/// let blue_bg = bg_color(0, 0, 255);
/// assert_eq!(blue_bg, "\x1b[48;2;0;0;255m");
/// ```
pub fn bg_color(r: u8, g: u8, b: u8) -> String {
    format!("{}{}m", BG, rgb_string(r, g, b))
}

/// Format RGB values as semicolon-separated string.
fn rgb_string(r: u8, g: u8, b: u8) -> String {
    format!("{};{};{}", r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fg_color() {
        assert_eq!(fg_color(255, 128, 0), "\x1b[38;2;255;128;0m");
    }

    #[test]
    fn test_bg_color() {
        assert_eq!(bg_color(0, 128, 255), "\x1b[48;2;0;128;255m");
    }

    #[test]
    fn test_digit_to_superscript() {
        assert_eq!(digit_to_superscript(0), '⁰');
        assert_eq!(digit_to_superscript(1), '¹');
        assert_eq!(digit_to_superscript(2), '²');
        assert_eq!(digit_to_superscript(3), '³');
        assert_eq!(digit_to_superscript(9), '⁹');
    }

    #[test]
    fn test_number_to_superscript() {
        assert_eq!(number_to_superscript(0), "⁰");
        assert_eq!(number_to_superscript(42), "⁴²");
        assert_eq!(number_to_superscript(123), "¹²³");
    }
}
