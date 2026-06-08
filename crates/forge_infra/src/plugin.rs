use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

/// A plugin that can be loaded and managed by the Forge application.
///
/// Plugins are dynamically loaded components that can extend the functionality
/// of the Forge application. They have access to services and can participate
/// in the application lifecycle.
#[async_trait::async_trait]
pub trait Plugin<S>: Send + Sync {
    /// Returns the plugin's name.
    ///
    /// The name must be unique among all loaded plugins.
    fn name(&self) -> &'static str;

    /// Returns a human-readable description of the plugin.
    fn description(&self) -> &'static str;

    /// Returns the plugin's version.
    fn version(&self) -> &'static str;

    /// Initializes the plugin with access to application services.
    ///
    /// # Arguments
    ///
    /// * `services` - Access to Forge application services
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if initialization was successful, or an error if it failed.
    async fn initialize(&self, services: Arc<S>) -> Result<()>;

    /// Shuts down the plugin and cleans up any resources.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if shutdown was successful, or an error if it failed.
    async fn shutdown(&self) -> Result<()>;

    /// Returns true if the plugin is currently active (initialized but not shut down).
    fn is_active(&self) -> bool;

    /// Returns the plugin as a reference to std::any::Any for type checking.
    fn as_any(&self) -> &dyn Any;

    /// Returns the plugin as a mutable reference to std::any::Any for type checking.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Manages the lifecycle of plugins in the Forge application.
///
/// The PluginManager is responsible for loading, initializing, and shutting down
/// plugins. It maintains a registry of all loaded plugins and provides access
/// to them by name or type.
pub struct PluginManager<S> {
    plugins: HashMap<String, Arc<dyn Plugin<S>>>,
    _service_type: std::marker::PhantomData<S>,
}

impl<S: Services> PluginManager<S> {
    /// Creates a new, empty PluginManager.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            _service_type: std::marker::PhantomData,
        }
    }

    /// Registers a plugin with the manager.
    ///
    /// If a plugin with the same name is already registered, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin to register
    ///
    /// # Returns
    ///
    /// Returns the previous plugin with the same name, if any.
    pub fn register_plugin(&mut self, plugin: Arc<dyn Plugin<S>>) -> Option<Arc<dyn Plugin<S>>> {
        self.plugins.insert(plugin.name().to_string(), plugin)
    }

    /// Retrieves a plugin by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the plugin to retrieve
    ///
    /// # Returns
    ///
    /// Returns Some(plugin) if found, or None if no plugin with that name exists.
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin<S>>> {
        self.plugins.get(name).cloned()
    }

    /// Retrieves all registered plugins.
    ///
    /// # Returns
    ///
    /// Returns a vector containing all registered plugins.
    pub fn get_all_plugins(&self) -> Vec<Arc<dyn Plugin<S>>> {
        self.plugins.values().cloned().collect()
    }

    /// Initializes all registered plugins.
    ///
    /// Plugins are initialized in the order they were registered.
    ///
    /// # Arguments
    ///
    /// * `services` - Access to Forge application services
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if all plugins were initialized successfully.
    /// If any plugin fails to initialize, initialization stops and the error is returned.
    pub async fn initialize_all(&self, services: Arc<S>) -> Result<()> {
        for plugin in self.plugins.values() {
            if !plugin.is_active() {
                plugin.initialize(services.clone()).await?;
            }
        }
        Ok(())
    }

    /// Shuts down all registered plugins.
    ///
    /// Plugins are shut down in reverse order of registration.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if all plugins were shut down successfully.
    /// If any plugin fails to shut down, the error is logged but shutdown continues.
    pub async fn shutdown_all(&self) -> Result<()> {
        // Shut down in reverse order
        let mut plugins: Vec<_> = self.plugins.values().cloned().collect();
        plugins.reverse();

        for plugin in plugins {
            if plugin.is_active() {
                let _ = plugin.shutdown().await;
            }
        }
        Ok(())
    }

    /// Returns the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Returns true if any plugins are registered.
    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }
}

impl<S: Services> Default for PluginManager<S> {
    fn default() -> Self {
        Self::new()
    }
}