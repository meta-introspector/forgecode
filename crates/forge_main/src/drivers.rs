//! Standalone CLI drivers for forge functionality
//! 
//! This module provides clean, reusable interfaces for executing forge commands
//! programmatically without the full interactive UI layer.

use anyhow::Result;
use clap::Parser;
use forge_domain::{ConversationId, Effort, ModelId, ProviderId};
use forge_main::{Cli, TopLevelCommand, UI};
use forge_api::ForgeAPI;
use forge_config::ForgeConfig;

/// Standalone driver for executing forge commands
/// 
/// This provides a clean interface for programmatic execution of forge commands
/// without the full interactive UI layer.
pub struct ForgeDriver {
    /// The underlying CLI configuration
    cli: Cli,
    /// Configuration for the forge instance
    config: ForgeConfig,
    /// API instance for backend operations
    api: ForgeAPI,
}

impl ForgeDriver {
    /// Create a new forge driver with default configuration
    pub async fn new() -> Result<Self> {
        let cli = Cli::parse_from(["forge"]);
        let config = ForgeConfig::read().await?;
        let api = ForgeAPI::init(std::env::current_dir()?, config.clone()).await?;
        
        Ok(Self { cli, config, api })
    }
    
    /// Create a new forge driver with custom CLI arguments
    pub async fn with_args<I, T>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<std::ffi::OsStr>,
    {
        let cli = Cli::parse_from(args);
        let config = ForgeConfig::read().await?;
        let api = ForgeAPI::init(std::env::current_dir()?, config.clone()).await?;
        
        Ok(Self { cli, config, api })
    }
    
    /// Execute a specific command programmatically
    pub async fn execute_command(&mut self, command: TopLevelCommand) -> Result<()> {
        // Create a temporary UI instance for this command execution
        let mut ui = UI::init(self.cli.clone(), self.config.clone(), |config| {
            ForgeAPI::init(std::env::current_dir()?, config)
        })?;
        
        // Handle the command directly
        ui.handle_subcommands(command).await
    }
    
    /// Execute a commit command with custom parameters
    pub async fn commit(
        &mut self,
        preview: bool,
        max_diff_size: Option<usize>,
        diff: Option<String>,
        text: Vec<String>,
    ) -> Result<()> {
        use forge_main::cli::CommitCommandGroup;
        
        let commit_group = CommitCommandGroup {
            preview,
            max_diff_size,
            diff,
            text,
        };
        
        let command = TopLevelCommand::Commit(commit_group);
        self.execute_command(command).await
    }
    
    /// Execute a config set command
    pub async fn config_set_model(
        &mut self,
        provider: ProviderId,
        model: ModelId,
    ) -> Result<()> {
        use forge_main::cli::{ConfigCommand, ConfigCommandGroup, ConfigSetField};
        
        let config_group = ConfigCommandGroup {
            command: ConfigCommand::Set(forge_main::cli::ConfigSetArgs {
                field: ConfigSetField::Model { provider, model },
            }),
            porcelain: false,
        };
        
        let command = TopLevelCommand::Config(config_group);
        self.execute_command(command).await
    }
    
    /// Execute a workspace sync command
    pub async fn workspace_sync(
        &mut self,
        path: std::path::PathBuf,
        init: bool,
    ) -> Result<()> {
        use forge_main::cli::{WorkspaceCommand, WorkspaceCommandGroup};
        
        let workspace_group = WorkspaceCommandGroup {
            command: WorkspaceCommand::Sync { path, init },
        };
        
        let command = TopLevelCommand::Workspace(workspace_group);
        self.execute_command(command).await
    }
}

/// Convenience function to run forge commands from external programs
/// 
/// This is the main entry point for standalone usage.
pub async fn run_cli<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: AsRef<std::ffi::OsStr>,
{
    let mut driver = ForgeDriver::with_args(args).await?;
    
    // Parse the command from args
    let cli = Cli::parse();
    
    if let Some(command) = cli.subcommands {
        driver.execute_command(command).await
    } else {
        // Default to interactive mode or help
        Ok(())
    }
}

/// Execute a specific forge command programmatically
/// 
/// Example usage:
/// ```rust
/// let mut driver = ForgeDriver::new().await?;
/// driver.commit(true, None, None, vec!["fix".to_string(), "bug".to_string()]).await?;
/// ```
pub async fn execute_command(command: TopLevelCommand) -> Result<()> {
    let mut driver = ForgeDriver::new().await?;
    driver.execute_command(command).await
}