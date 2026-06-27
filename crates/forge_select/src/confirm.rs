use anyhow::Result;
use colored::Colorize;

use crate::input::InputBuilder;

/// Builder for confirm (yes/no) prompts with enhanced features.
pub struct ConfirmBuilder {
    pub(crate) message: String,
    pub(crate) default: Option<bool>,
    pub(crate) view_more: Option<String>,
    pub(crate) actions: Vec<String>,
    /// Preview content to show before approval (e.g., file diffs, permission changes)
    pub(crate) preview: Option<String>,
    /// Permission details for "Review Permissions" action
    pub(crate) permissions: Option<String>,
    /// Pattern details for "Edit Patterns" action
    pub(crate) patterns: Option<String>,
    /// Track if we're in dry-run mode for bench testing
    pub(crate) dry_run: bool,
    /// Log of actions taken during dry run
    pub(crate) action_log: Vec<String>,
}

impl ConfirmBuilder {
    /// Set the default value for the confirm prompt.
    pub fn with_default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    /// Add more context that can be viewed with 'view' command.
    pub fn view_more(mut self, more_text: String) -> Self {
        self.view_more = Some(more_text);
        self
    }

    /// Add preview content to show before approval
    pub fn preview(mut self, preview_text: String) -> Self {
        self.preview = Some(preview_text);
        self
    }

    /// Add permission details for "Review Permissions" action
    pub fn permissions(mut self, permissions_text: String) -> Self {
        self.permissions = Some(permissions_text);
        self
    }

    /// Add pattern details for "Edit Patterns" action
    pub fn patterns(mut self, patterns_text: String) -> Self {
        self.patterns = Some(patterns_text);
        self
    }

    /// Add numbered action options to the prompt.
    pub fn actions(mut self, actions: Vec<String>) -> Self {
        self.actions = actions;
        self
    }

    /// Enable dry-run mode where actions are logged instead of executed.
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Execute the confirm prompt with enhanced features.
    pub fn prompt(self) -> Result<Option<bool>> {
        let mut action_log = Vec::new();
        let mut shown_actions = false;
        let mut shown_view_more = false;
        let mut preview_offset = 0;
        let mut preview_exhausted = false;
        const PREVIEW_PAGE_SIZE: usize = 15;
        
        // Add action descriptions to help user understand options
        let mut action_help = String::new();
        for (i, action) in self.actions.iter().enumerate() {
            action_help.push_str(&format!("  {}. {} ", i + 1, action));
        }

        loop {
            // Show preview content if available (before asking for actions)
            // Paginate preview output (default 15 lines per page)
            let preview_lines: Vec<&str> = self.preview.as_ref().map(|p| p.lines().collect()).unwrap_or_default();
            if !preview_exhausted && preview_offset < preview_lines.len() {
                let end = std::cmp::min(preview_offset + PREVIEW_PAGE_SIZE, preview_lines.len());
                for line in &preview_lines[preview_offset..end] {
                    println!("{}", line);
                }
                preview_offset = end;
                if preview_offset < preview_lines.len() {
                    println!("(type 'more' to see more preview)");
                } else {
                    preview_exhausted = true;
                }
            }
            
            let input_builder = InputBuilder {
                message: format!("{} {}", 
                    self.message, 
                    if self.default == Some(true) { "(Y/n)" } 
                    else if self.default == Some(false) { "(y/N)" } 
                    else { "(y/n)" }.to_string()),
                allow_empty: true,
                default: None,
                default_display: None,
            };
            
            let result = input_builder.prompt()?;

            // User cancelled (Ctrl+C or EOF)
            if result.is_none() {
                return Ok(None);
            }

            let input = result.unwrap().trim().to_lowercase();

            // Handle 'view' command
            if input == "view" {
                if let Some(more_text) = &self.view_more {
                    println!("{}", more_text);
                    shown_view_more = true;
                }
                continue;
            }
            
            // Handle 'more' command for paginated preview
            if input == "more" && !preview_exhausted {
                // The preview will automatically show the next page on the next loop iteration
                // because preview_offset has already been updated
                continue;
            }

            // Handle numbered actions
            if let Ok(action_idx) = input.parse::<usize>() {
                if action_idx > 0 && action_idx <= self.actions.len() {
                    let action = &self.actions[action_idx - 1];
                    
                    // Record action in dry-run mode or execute
                    if self.dry_run {
                        action_log.push(format!("Executed action: {}", action));
                        println!("{}", format!("DRY RUN: {}", action).bright_blue());
                    } else {
                        match action.as_str() {
                            "Approve" | "Approve Always" => return Ok(Some(true)),
                            "Review Permissions" => {
                                // Show permission details
                                if let Some(perms) = &self.permissions {
                                    println!("\n=== Permission Details ===");
                                    println!("{}", perms);
                                    println!("========================\n");
                                }
                                continue;
                            }
                            "Edit Patterns" => {
                                // Show pattern details
                                if let Some(patterns) = &self.patterns {
                                    println!("\n=== Pattern Details ===");
                                    println!("{}", patterns);
                                    println!("======================\n");
                                }
                                continue;
                            }
                            _ => println!("Executing: {}", action),
                        }
                    }
                    
                    shown_actions = true;
                    continue;
                }
            }

            // Handle yes/no response
            if input == "y" || input == "yes" {
                return Ok(Some(true));
            }
            if input == "n" || input == "no" {
                return Ok(Some(false));
            }

            // Handle empty input
            if input.is_empty() {
                return Ok(Some(self.default.unwrap_or(false)));
            }

            // Show help if no actions shown yet
            if !shown_actions {
                println!("{}", action_help.bright_green());
                shown_actions = true;
            }
            
            // Prompt for view command if view_more available
            if self.view_more.is_some() && !shown_view_more {
                println!("(type 'view' to see more context)");
            }
        }
    }

    /// Retrieve the action log from a dry run.
    pub fn get_log(&self) -> Vec<String> {
        self.action_log.clone()
    }
}