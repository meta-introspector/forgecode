/// Forge Infrastructure Plugin System
///
/// This module provides the foundational plugin infrastructure for the Forge application.
/// Plugins are dynamically loaded components that can extend Forge's functionality
/// while maintaining loose coupling through well-defined interfaces.
///
/// # Plugin System Overview
///
/// The Forge plugin system consists of several key components:
///
/// 1. **Plugin Trait** (`plugin.rs`): Defines the interface that all plugins must implement
/// 2. **PluginManager** (`plugin.rs`): Handles plugin lifecycle (registration, initialization, shutdown)
/// 3. **ZosPluginBridge** (`zos_bridge.rs`): Loads ZOS plugins from the zos-server/plugins directory
///
/// # Creating a Plugin
///
/// To create a Forge plugin, implement the `Plugin<S>` trait where `S` is a type that
/// implements the `Services` trait (providing access to Forge services):
///
/// ```
/// use forge_infra::Plugin;
/// use forge_services::Services;
///
/// struct MyPlugin;
///
/// #[async_trait::async_trait]
/// impl<S: Services> Plugin<S> for MyPlugin {
///     fn name(&self) -> &'static str {
///         "my-plugin"
///     }
///     
///     fn description(&self) -> &'static str {
///         "A sample Forge plugin"
///     }
///     
///     fn version(&self) -> &'static str {
///         "0.1.0"
///     }
///     
///     async fn initialize(&self, services: Arc<S>) -> Result<()> {
///         // Initialize plugin with access to Forge services
///         Ok(())
///     }
///     
///     async fn shutdown(&self) -> Result<()> {
///         // Clean up resources
///         Ok(())
///     }
///     
///     fn is_active(&self) -> bool {
///         // Return true if plugin is initialized
///         true
///     }
///     
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
///     
///     fn as_any_mut(&mut self) -> &mut dyn Any {
///         self
///     }
/// }
/// ```
///
/// # ZOS Plugin Bridge
///
/// The `ZosPluginBridge` automatically discovers and loads plugins from the
/// `~/zos-server/plugins/` directory. Each ZOS plugin must export three functions:
///
/// * `zos_plugin_name()` - Returns the plugin name as a null-terminated C string
/// * `zos_plugin_init()` - Returns 0 on success, non-zero on failure
/// * `zos_plugin_destroy()` - Returns 0 on success, non-zero on failure
///
/// These plugins are written in Rust and compiled as cdylib dynamic libraries.
///
/// # Using the Plugin System
///
/// Plugins are typically managed through the `ForgeServices` struct, which
/// contains a `PluginManager` and automatically loads ZOS plugins during initialization:
///
/// ```
/// let services = ForgeServices::new(infra);
/// services.initialize_plugins().await?; // Loads and initializes all plugins
/// ```
/// 
/// The plugin system enables Forge to be extended with new functionality
/// without modifying the core application, supporting both first-party plugins
/// (like those in zos-server/plugins) and third-party extensions.

mod auth;
mod console;
mod env;
mod error;
mod executor;
mod forge_infra;
mod fs_create_dirs;
mod fs_meta;
mod fs_read;
mod fs_read_dir;
mod fs_remove;
mod fs_write;
mod grpc;
mod http;
mod inquire;
mod kv_storage;
mod mcp_client;
mod mcp_server;
mod plugin;
mod walker;
mod zos_bridge;

pub use console::StdConsoleWriter;
pub use env::ForgeEnvironmentInfra;
pub use executor::ForgeCommandExecutorService;
pub use forge_infra::*;
pub use http::sanitize_headers;
pub use kv_storage::CacacheStorage;
pub use mcp_client::*;
pub use plugin::{Plugin, PluginManager};
pub use zos_bridge::ZosPluginBridge;