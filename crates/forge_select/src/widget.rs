use crate::confirm::ConfirmBuilder;
use crate::input::InputBuilder;
use crate::multi::MultiSelectBuilder;
use crate::preview::{SelectRow, SelectUiOptions};
use crate::select::SelectBuilder;

/// Centralized fuzzy select functionality with consistent error handling.
///
/// All interactive selection is handled by the shared nucleo-backed selector
/// UI.
pub struct ForgeWidget;

impl ForgeWidget {
    /// Entry point for select operations with fuzzy search.
    pub fn select<T>(message: impl Into<String>, options: Vec<T>) -> SelectBuilder<T> {
        SelectBuilder {
            message: message.into(),
            options,
            starting_cursor: None,
            default: None,
            help_message: None,
            initial_text: None,
            header_lines: 0,
            preview: None,
            preview_window: None,
        }
    }

    /// Convenience method for confirm (yes/no) prompts.
    pub fn confirm(
        message: impl Into<String>,
        preview: Option<String>,
        permissions: Option<String>,
        patterns: Option<String>,
    ) -> ConfirmBuilder {
        ConfirmBuilder {
            message: message.into(),
            preview,
            permissions,
            patterns,
            default: None,
            view_more: None,
            actions: Vec::new(),
            dry_run: false,
            action_log: Vec::new(),
        }
    }

    /// Prompt a question and get text input.
    pub fn input(message: impl Into<String>) -> InputBuilder {
        InputBuilder {
            message: message.into(),
            allow_empty: false,
            default: None,
            default_display: None,
        }
    }

    /// Multi-select prompt.
    pub fn multi_select<T>(message: impl Into<String>, options: Vec<T>) -> MultiSelectBuilder<T> {
        MultiSelectBuilder {
            message: message.into(),
            options,
        }
    }

    /// Entry point for row-based select operations.
    pub fn select_rows(message: impl Into<String>, rows: Vec<SelectRow>) -> SelectUiOptions {
        SelectUiOptions::new(message, rows)
    }
}