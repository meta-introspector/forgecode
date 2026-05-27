use std::cmp;
use std::io::{self, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEventKind,
};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode};
use crossterm::{execute, queue};

/// Result of the permission pager interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPagerResult {
    /// User accepted the operation.
    Accept,
    /// User accepted and wants to remember this choice (create a policy rule).
    AcceptAndRemember,
    /// User rejected the operation.
    Reject,
}

/// Runs an interactive permission pager that displays content and lets the
/// user Accept, AcceptAndRemember, or Reject.
///
/// The pager renders the given panel content in a scrollable view and always
/// shows a footer bar with keybindings at the bottom of the terminal.
///
/// # Arguments
/// * `panel` - The formatted panel text to display (e.g. from
///   `PermissionCase::format_panel()`)
///
/// # Returns
/// * `PermissionPagerResult::Accept` — Enter pressed
/// * `PermissionPagerResult::AcceptAndRemember` — 'A' pressed
/// * `PermissionPagerResult::Reject` — 'R', Esc, or Ctrl+C pressed
///
/// # Errors
/// Returns an error if terminal setup, event handling, or rendering fails.
pub fn show_permission_pager(panel: &str) -> anyhow::Result<PermissionPagerResult> {
    let mut stderr = io::BufWriter::new(io::stderr());

    // Enter raw mode + hide cursor + enable mouse
    let raw_mode_was_enabled = terminal::is_raw_mode_enabled()?;
    enable_raw_mode()?;
    execute!(stderr, EnableMouseCapture, Hide)?;

    let result = run_pager(&mut stderr, panel);

    // Restore terminal state
    let _ = execute!(stderr, Show, DisableMouseCapture);
    let _ = stderr.flush();
    if !raw_mode_was_enabled {
        let _ = disable_raw_mode();
    }

    result
}

fn run_pager(
    stderr: &mut impl Write,
    panel: &str,
) -> anyhow::Result<PermissionPagerResult> {
    let lines: Vec<&str> = panel.lines().collect();
    let total_lines = lines.len();
    let mut scroll_offset = 0usize;
    let mut content_height;

    loop {
        // ═══════════════════════════════════════════════════════════════
        // Redraw on every cycle (no dirty flag) to handle interference
        // from background threads (e.g. the spinner) that write to stderr
        // while the pager is active.
        // ═══════════════════════════════════════════════════════════════
        let (width, height) = terminal::size()?;
        let footer_height = 2u16; // 2 rows for footer: separator bar + keybindings
        content_height = height.saturating_sub(footer_height).max(1) as usize;

        // Clamp scroll offset
        if total_lines > content_height {
            let max_offset = total_lines - content_height;
            if scroll_offset > max_offset {
                scroll_offset = max_offset;
            }
        } else {
            scroll_offset = 0;
            content_height = total_lines;
        }

        // Clear entire screen first to wipe any leftover content
        // (status messages, tool output, etc.) that was written before the
        // pager entered raw mode.
        queue!(stderr, Clear(ClearType::All))?;

        // Clear content area only (not footer)
        for row in 0..content_height {
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, row as u16),
                Clear(ClearType::CurrentLine)
            )?;
        }

        // Draw content lines
        let visible_end = cmp::min(scroll_offset + content_height, total_lines);
        for (i, line_idx) in (scroll_offset..visible_end).enumerate() {
            let line = lines[line_idx];
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, i as u16),
                Print(truncate_line(line, width as usize))
            )?;
        }

        // Scroll indicator in top-right (if scrolled)
        if total_lines > content_height {
            let indicator =
                format!("{}/{}", scroll_offset.saturating_add(1), total_lines);
            if indicator.len() + 2 < width as usize {
                queue!(
                    stderr,
                    crossterm::cursor::MoveTo(
                        width.saturating_sub(indicator.len() as u16 + 2),
                        0,
                    ),
                    SetForegroundColor(Color::DarkYellow),
                    Print(&indicator),
                    ResetColor
                )?;
            }
        }

        // Clear and redraw footer each time
        let footer_y = height.saturating_sub(footer_height);
        for row in footer_y..height {
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, row),
                Clear(ClearType::CurrentLine)
            )?;
        }
        // Separator line
        let separator = "─".repeat(width as usize);
        queue!(
            stderr,
            crossterm::cursor::MoveTo(0, footer_y),
            SetForegroundColor(Color::DarkGrey),
            Print(separator),
            ResetColor
        )?;
        // Keybindings bar
        let keybindings = format!(
            " [Enter] Accept  [A] Accept & Remember  [R] Reject  ↑↓u/d PgUp/PgDn Scroll"
        );
        queue!(
            stderr,
            crossterm::cursor::MoveTo(0, footer_y + 1),
            SetForegroundColor(Color::Cyan),
            Print(truncate_line(&keybindings, width as usize)),
            ResetColor
        )?;

        stderr.flush()?;

        // Wait for event
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    let old_offset = scroll_offset;
                    match handle_pager_key(
                        key,
                        total_lines,
                        content_height,
                        &mut scroll_offset,
                    ) {
                        PagerAction::Accept => return Ok(PermissionPagerResult::Accept),
                        PagerAction::AcceptAndRemember => {
                            return Ok(PermissionPagerResult::AcceptAndRemember)
                        }
                        PagerAction::Reject => return Ok(PermissionPagerResult::Reject),
                        PagerAction::Continue => {
                            // scroll_offset updates will be reflected on next draw
                            let _ = old_offset;
                        }
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if total_lines > content_height {
                            scroll_offset = scroll_offset.saturating_sub(3);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if total_lines > content_height {
                            scroll_offset = cmp::min(
                                scroll_offset.saturating_add(3),
                                total_lines.saturating_sub(content_height),
                            );
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    // Handled naturally — we recalculate on every iteration
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PagerAction {
    Accept,
    AcceptAndRemember,
    Reject,
    Continue,
}

fn handle_pager_key(
    key: KeyEvent,
    total_lines: usize,
    content_height: usize,
    scroll_offset: &mut usize,
) -> PagerAction {
    match key {
        // Accept
        KeyEvent { code: KeyCode::Enter, .. } => PagerAction::Accept,
        // Accept and Remember
        KeyEvent { code: KeyCode::Char('a'), modifiers, .. }
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            PagerAction::AcceptAndRemember
        }
        KeyEvent { code: KeyCode::Char('A'), .. } => PagerAction::AcceptAndRemember,
        // Reject
        KeyEvent { code: KeyCode::Char('r'), modifiers, .. }
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            PagerAction::Reject
        }
        KeyEvent { code: KeyCode::Char('R'), .. } => PagerAction::Reject,
        KeyEvent { code: KeyCode::Esc, .. } => PagerAction::Reject,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => PagerAction::Reject,
        // Scroll up (also 'u' for vi-style)
        KeyEvent { code: KeyCode::Up, .. } => {
            *scroll_offset = scroll_offset.saturating_sub(1);
            PagerAction::Continue
        }
        KeyEvent { code: KeyCode::Char('u'), .. } => {
            let page = content_height.saturating_sub(1).max(1);
            *scroll_offset = scroll_offset.saturating_sub(page);
            PagerAction::Continue
        }
        KeyEvent { code: KeyCode::PageUp, .. } => {
            let page = content_height.saturating_sub(1).max(1);
            *scroll_offset = scroll_offset.saturating_sub(page);
            PagerAction::Continue
        }
        // Scroll down (also 'd' for vi-style)
        KeyEvent { code: KeyCode::Down, .. } => {
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(1), max_offset);
            PagerAction::Continue
        }
        KeyEvent { code: KeyCode::Char('d'), .. } => {
            let page = content_height.saturating_sub(1).max(1);
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(page), max_offset);
            PagerAction::Continue
        }
        KeyEvent { code: KeyCode::PageDown, .. } => {
            let page = content_height.saturating_sub(1).max(1);
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(page), max_offset);
            PagerAction::Continue
        }
        _ => PagerAction::Continue,
    }
}

fn truncate_line(value: &str, max_width: usize) -> String {
    let mut rendered = String::new();
    let mut visible_width = 0usize;
    let mut chars = value.chars().peekable();
    let mut truncated = false;
    let mut has_ansi = false;

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            has_ansi = true;
            rendered.push(ch);
            for ansi_ch in chars.by_ref() {
                rendered.push(ansi_ch);
                if ansi_ch.is_ascii_alphabetic() || ansi_ch == '~' {
                    break;
                }
            }
            continue;
        }

        if visible_width >= max_width {
            truncated = true;
            break;
        }

        rendered.push(ch);
        visible_width = visible_width.saturating_add(1);
    }

    if truncated && has_ansi {
        rendered.push_str("\u{1b}[0m");
    }

    rendered
}

/// Interactive reasoning viewer with a histogram overview and section navigation.
///
/// Shows the reasoning/thought text in a scrollable pager with:
/// - A header histogram showing proportional section sizes with numbered labels
/// - The full reasoning text below, scrollable with arrow/page keys
/// - `1`-`9` number keys to jump to the corresponding section
///
/// # Arguments
/// * `reasoning` - The full reasoning text to display
///
/// # Errors
/// Returns an error if terminal setup, event handling, or rendering fails.
pub fn show_reasoning_pager(reasoning: &str) -> anyhow::Result<()> {
    let mut stderr = io::BufWriter::new(io::stderr());

    let raw_mode_was_enabled = terminal::is_raw_mode_enabled()?;
    enable_raw_mode()?;
    execute!(stderr, EnableMouseCapture, Hide)?;

    run_reasoning_pager(&mut stderr, reasoning)?;

    let _ = execute!(stderr, Show, DisableMouseCapture);
    let _ = stderr.flush();
    if !raw_mode_was_enabled {
        let _ = disable_raw_mode();
    }
    Ok(())
}

/// Metadata for a reasoning section used by the histogram.
struct ReasoningSection {
    /// Display number (1-based)
    number: usize,
    /// Short label (first ~25 chars)
    label: String,
    /// Line index in the full text where this section starts
    start_line: usize,
    /// Character count of the section (for proportional bar width)
    char_len: usize,
}

/// Splits reasoning text into sections by double-newline paragraphs and builds
/// metadata for each section.
fn build_section_metadata(lines: &[&str]) -> Vec<ReasoningSection> {
    // Split by blank lines to find section boundaries
    let mut sections: Vec<ReasoningSection> = Vec::new();
    let mut current_start = 0usize;
    let mut current_char_len = 0usize;
    let mut section_idx = 1usize;

    for (line_idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() && current_char_len > 0 {
            // End of a section
            let label_raw = lines[current_start].trim();
            let label = if label_raw.len() > 25 {
                format!("{}...", &label_raw[..22])
            } else {
                label_raw.to_string()
            };
            sections.push(ReasoningSection {
                number: section_idx,
                label,
                start_line: current_start,
                char_len: current_char_len,
            });
            section_idx += 1;
            current_start = line_idx + 1;
            current_char_len = 0;
            continue;
        }
        current_char_len = current_char_len.saturating_add(line.len()).saturating_add(1);
    }

    // Don't forget the last section if there's trailing content
    if current_char_len > 0 && current_start < lines.len() {
        let label_raw = lines[current_start].trim();
        let label = if label_raw.len() > 25 {
            format!("{}...", &label_raw[..22])
        } else {
            label_raw.to_string()
        };
        sections.push(ReasoningSection {
            number: section_idx,
            label,
            start_line: current_start,
            char_len: current_char_len,
        });
    }

    sections
}

/// Determine the foreground color for a histogram bar based on section index.
fn section_color(index: usize) -> Color {
    const COLORS: &[Color] = &[
        Color::Green,
        Color::Cyan,
        Color::Yellow,
        Color::Magenta,
        Color::Blue,
        Color::Red,
        Color::DarkCyan,
        Color::DarkYellow,
        Color::DarkMagenta,
    ];
    COLORS.get(index % COLORS.len()).copied().unwrap_or(Color::White)
}

fn run_reasoning_pager(
    stderr: &mut impl Write,
    reasoning: &str,
) -> anyhow::Result<()> {
    let lines: Vec<&str> = reasoning.lines().collect();
    let total_lines = lines.len();
    let sections = build_section_metadata(&lines);
    let total_sections = sections.len();
    let total_chars: usize = sections.iter().map(|s| s.char_len).sum();

    let mut scroll_offset = 0usize;

    loop {
        let (width, height) = terminal::size()?;

        // Header: histogram bar + labels
        let header_lines = if total_sections > 0 { 3u16 } else { 1u16 }; // title + maybe histogram + separator
        let footer_height = 1u16; // keybinding row
        let header_height = header_lines as usize;
        let content_area = height.saturating_sub(footer_height).max(1);
        let content_height = content_area.saturating_sub(header_lines).max(1) as usize;

        // Clamp scroll offset
        if total_lines > content_height {
            let max_offset = total_lines.saturating_sub(content_height);
            if scroll_offset > max_offset {
                scroll_offset = max_offset;
            }
        } else {
            scroll_offset = 0;
        }

        // Clear screen
        queue!(stderr, Clear(ClearType::All))?;

        // ── Draw header: histogram bar ──────────────────────────────────
        if total_sections > 0 {
            // Title line
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, 0),
                SetForegroundColor(Color::White),
                Print(format!(
                    " Reasoning Overview  ({total_sections} section{})",
                    if total_sections == 1 { "" } else { "s" }
                )),
                ResetColor
            )?;

            // Histogram bar — proportional colored blocks with numbers
            let bar_max_width = width.saturating_sub(4) as usize;
            let bar_y = 1u16;

            // How many characters of bar each section gets
            let mut remaining_width = bar_max_width;
            let sections_to_render = sections.len().min(9);

            // Calculate if we have room — if the labels alone would be cramped, hide labels
            let _estimated_label_width: usize = sections
                .iter()
                .take(sections_to_render)
                .map(|s| 3usize.saturating_add(s.label.len())) // " [N] label"
                .sum::<usize>()
                .saturating_add(4); // for " …" suffix if truncated

            // First render the block bar, then labels on the next line
            // --- Histogram blocks ---
            queue!(stderr, crossterm::cursor::MoveTo(1, bar_y))?;
            let _drawn_blocks = 0usize;
            for sec in &sections[..sections_to_render] {
                if remaining_width < 2 {
                    break;
                }
                let block_w = if total_chars > 0 {
                    cmp::max(1usize, (sec.char_len * remaining_width) / total_chars)
                } else {
                    1
                };
                let block_w = cmp::min(block_w, remaining_width);
                let block = "█".repeat(block_w);
                queue!(
                    stderr,
                    SetForegroundColor(section_color(sec.number)),
                    Print(block),
                    ResetColor
                )?;
                remaining_width = remaining_width.saturating_sub(block_w);
            }

            if total_sections > sections_to_render {
                // Truncation indicator
                let rem = total_sections.saturating_sub(sections_to_render);
                queue!(
                    stderr,
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("…+{rem}")),
                    ResetColor
                )?;
            }

            // --- Section labels line ---
            let label_y = 2u16;
            let mut label_cursor = 1u16;
            queue!(stderr, crossterm::cursor::MoveTo(label_cursor, label_y))?;

            for sec in &sections[..sections_to_render] {
                let label_text = format!(" [{}] {}", sec.number, sec.label);
                let label_w = visible_width_for(&label_text);
                if label_cursor.saturating_add(label_w as u16) > width.saturating_sub(2) {
                    // Not enough room for this label, show "…"
                    queue!(
                        stderr,
                        SetForegroundColor(Color::DarkGrey),
                        Print(" …"),
                        ResetColor
                    )?;
                    break;
                }
                queue!(
                    stderr,
                    SetForegroundColor(section_color(sec.number)),
                    Print(format!("[{}]", sec.number)),
                    ResetColor,
                    Print(format!(" {} ", sec.label)),
                )?;
                label_cursor = label_cursor.saturating_add(label_w as u16);
            }
        } else {
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, 0),
                SetForegroundColor(Color::DarkGrey),
                Print(" (no reasoning content)"),
                ResetColor
            )?;
        }

        // ── Separator line ─────────────────────────────────────────────
        let sep_y = header_height.saturating_sub(1) as u16;
        let sep = "─".repeat(width as usize);
        queue!(
            stderr,
            crossterm::cursor::MoveTo(0, sep_y),
            SetForegroundColor(Color::DarkGrey),
            Print(sep),
            ResetColor
        )?;

        // ── Content area ───────────────────────────────────────────────
        let content_start_y = header_lines;
        for row in content_start_y..content_start_y.saturating_add(content_height as u16) {
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, row),
                Clear(ClearType::CurrentLine)
            )?;
        }

        let visible_end = cmp::min(scroll_offset + content_height, total_lines);
        for (i, line_idx) in (scroll_offset..visible_end).enumerate() {
            let line = lines[line_idx];
            queue!(
                stderr,
                crossterm::cursor::MoveTo(0, content_start_y + i as u16),
                Print(truncate_line(line, width as usize))
            )?;
        }

        // Scroll indicator
        if total_lines > content_height {
            let indicator = format!(
                "{}/{}",
                scroll_offset.saturating_add(1),
                total_lines
            );
            if indicator.len() + 2 < width as usize {
                queue!(
                    stderr,
                    crossterm::cursor::MoveTo(
                        width.saturating_sub(indicator.len() as u16 + 2),
                        0,
                    ),
                    SetForegroundColor(Color::DarkYellow),
                    Print(&indicator),
                    ResetColor
                )?;
            }
        }

        // ── Clear footer area ──────────────────────────────────────────
        let footer_y = height.saturating_sub(footer_height);
        queue!(
            stderr,
            crossterm::cursor::MoveTo(0, footer_y),
            Clear(ClearType::CurrentLine)
        )?;

        // Keybindings
        let kb = "[Esc/q] Close  [1-9] Jump to section  ↑↓→← PageUp/Dn Scroll";
        queue!(
            stderr,
            crossterm::cursor::MoveTo(0, footer_y),
            SetForegroundColor(Color::Cyan),
            Print(truncate_line(kb, width as usize)),
            ResetColor
        )?;

        stderr.flush()?;

        // ── Wait for event ─────────────────────────────────────────────
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    let old = scroll_offset;
                    match handle_reasoning_key(
                        key,
                        total_lines,
                        content_height,
                        &mut scroll_offset,
                        &sections,
                    ) {
                        ReasoningAction::Close => break,
                        ReasoningAction::Continue => {
                            let _ = old;
                        }
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        scroll_offset = scroll_offset.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        if total_lines > content_height {
                            scroll_offset = cmp::min(
                                scroll_offset.saturating_add(3),
                                total_lines.saturating_sub(content_height),
                            );
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    // Recalculated on next iteration
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Actions from reasoning pager key events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReasoningAction {
    Close,
    Continue,
}

fn handle_reasoning_key(
    key: KeyEvent,
    total_lines: usize,
    content_height: usize,
    scroll_offset: &mut usize,
    sections: &[ReasoningSection],
) -> ReasoningAction {
    match key {
        // Close
        KeyEvent { code: KeyCode::Esc, .. } => ReasoningAction::Close,
        KeyEvent { code: KeyCode::Char('q'), .. } => ReasoningAction::Close,
        KeyEvent { code: KeyCode::Char('Q'), .. } => ReasoningAction::Close,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => ReasoningAction::Close,

        // Jump to section by number
        KeyEvent {
            code: KeyCode::Char(ch @ '1'..='9'),
            ..
        } => {
            let target = ch.to_digit(10).unwrap_or(1) as usize;
            if let Some(section) = sections.iter().find(|s| s.number == target) {
                *scroll_offset = section.start_line;
                // Clamp
                let max_offset = total_lines.saturating_sub(content_height);
                *scroll_offset = cmp::min(*scroll_offset, max_offset);
            }
            ReasoningAction::Continue
        }

        // Scroll
        KeyEvent { code: KeyCode::Up, .. } => {
            *scroll_offset = scroll_offset.saturating_sub(1);
            ReasoningAction::Continue
        }
        KeyEvent { code: KeyCode::Down, .. } => {
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(1), max_offset);
            ReasoningAction::Continue
        }
        // u / d for half-page vi-style
        KeyEvent { code: KeyCode::Char('u'), .. } => {
            let page = content_height.saturating_sub(1).max(1);
            *scroll_offset = scroll_offset.saturating_sub(page);
            ReasoningAction::Continue
        }
        KeyEvent { code: KeyCode::Char('d'), .. } => {
            let page = content_height.saturating_sub(1).max(1);
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(page), max_offset);
            ReasoningAction::Continue
        }
        KeyEvent { code: KeyCode::PageUp, .. } => {
            let page = content_height.saturating_sub(1).max(1);
            *scroll_offset = scroll_offset.saturating_sub(page);
            ReasoningAction::Continue
        }
        KeyEvent { code: KeyCode::PageDown, .. } => {
            let page = content_height.saturating_sub(1).max(1);
            let max_offset = total_lines.saturating_sub(content_height);
            *scroll_offset = cmp::min(scroll_offset.saturating_add(page), max_offset);
            ReasoningAction::Continue
        }
        _ => ReasoningAction::Continue,
    }
}

/// Measure visible width of a string (ignoring ANSI codes).
fn visible_width_for(value: &str) -> usize {
    console::measure_text_width(value)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_truncate_line_simple() {
        let fixture = "Hello, World!";
        let actual = truncate_line(fixture, 5);
        let expected = "Hello";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_truncate_line_no_truncation() {
        let fixture = "Hi";
        let actual = truncate_line(fixture, 80);
        let expected = "Hi";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_truncate_line_ansi() {
        let fixture = "\u{1b}[31mRed\u{1b}[0m text";
        let actual = truncate_line(fixture, 10);
        // Should keep ANSI sequences
        assert!(actual.starts_with("\u{1b}[31m"));
        assert!(actual.len() <= 30);
    }

    #[test]
    fn test_handle_pager_key_accept() {
        let mut offset = 0usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Accept);
    }

    #[test]
    fn test_handle_pager_key_accept_and_remember() {
        let mut offset = 0usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::AcceptAndRemember);
    }

    #[test]
    fn test_handle_pager_key_accept_and_remember_uppercase() {
        let mut offset = 0usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::AcceptAndRemember);
    }

    #[test]
    fn test_handle_pager_key_reject() {
        let mut offset = 0usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Reject);
    }

    #[test]
    fn test_handle_pager_key_reject_escape() {
        let mut offset = 0usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Reject);
    }

    #[test]
    fn test_handle_pager_key_scroll_up() {
        let mut offset = 10usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Continue);
        assert_eq!(offset, 9);
    }

    #[test]
    fn test_handle_pager_key_scroll_down() {
        let mut offset = 10usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Continue);
        assert_eq!(offset, 11);
    }

    #[test]
    fn test_handle_pager_key_scroll_down_clamped() {
        let mut offset = 90usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Continue);
        assert_eq!(offset, 80);
    }

    #[test]
    fn test_handle_pager_key_page_down() {
        let mut offset = 10usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Continue);
        assert_eq!(offset, 29);
    }

    #[test]
    fn test_handle_pager_key_page_up() {
        let mut offset = 50usize;
        let result = handle_pager_key(
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
            100,
            20,
            &mut offset,
        );
        assert_eq!(result, PagerAction::Continue);
        assert_eq!(offset, 31);
    }
}
