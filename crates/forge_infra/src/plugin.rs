use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

/// A plugin that can be loaded and managed by the Forge application.
///
/// Plugins are dynamically loaded components that can extend the functionality
/// of the Forge application. They have access to services and can participate
/// in the application lifecycle.
#[async_trait::async_trait]
pub trait Plugin<S>: Send + Sync
where
    S: Send + Sync + 'static,
{
    /// Returns the plugin's name.
    ///
    /// The name must be unique among all loaded plugins.
    fn name(&self) -> &str;

    /// Returns a human-readable description of the plugin.
    fn description(&self) -> &str;

    /// Returns the plugin's version.
    fn version(&self) -> &str;

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

impl<S: Send + Sync + 'static> PluginManager<S> {
    /// Creates a new, empty PluginManager.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            _service_type: std::marker::PhantomData::<S>,
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

    /// Lists registered plugins as lightweight metadata.
    ///
    /// # Returns
    ///
    /// Returns plugin metadata sorted by name for stable command output.
    pub fn list_plugins(&self) -> Vec<forge_domain::PluginInfo> {
        let mut plugins: Vec<_> = self
            .plugins
            .values()
            .map(|plugin| forge_domain::PluginInfo {
                name: plugin.name().to_string(),
                description: plugin.description().to_string(),
                version: plugin.version().to_string(),
                active: plugin.is_active(),
            })
            .collect();
        plugins.sort_by(|left, right| left.name.cmp(&right.name));
        plugins
    }

    /// Replaces the current plugin registry with the provided plugins.
    ///
    /// Existing plugins are not shut down by this method; callers should invoke
    /// `shutdown_all` before replacing the registry when a reload is intended.
    ///
    /// # Arguments
    ///
    /// * `plugins` - New plugins to register.
    pub fn replace_plugins<I>(&mut self, plugins: I)
    where
        I: IntoIterator<Item = Arc<dyn Plugin<S>>>,
    {
        self.plugins.clear();
        for plugin in plugins {
            self.register_plugin(plugin);
        }
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

impl<S: Send + Sync + 'static> Default for PluginManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Send + Sync + 'static> Clone for PluginManager<S> {
    fn clone(&self) -> Self {
        Self {
            plugins: self.plugins.clone(),
            _service_type: std::marker::PhantomData::<S>,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pretty_assertions::assert_eq;

    use super::*;

    struct CountingPlugin {
        initializes: AtomicUsize,
        shutdowns: AtomicUsize,
        active: AtomicUsize,
    }

    impl CountingPlugin {
        fn new() -> Self {
            Self {
                initializes: AtomicUsize::new(0),
                shutdowns: AtomicUsize::new(0),
                active: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl<S: Send + Sync + 'static> Plugin<S> for CountingPlugin {
        fn name(&self) -> &str {
            "counting-plugin"
        }

        fn description(&self) -> &str {
            "Counts lifecycle calls"
        }

        fn version(&self) -> &str {
            "0.1.0"
        }

        async fn initialize(&self, _services: Arc<S>) -> Result<()> {
            self.initializes.fetch_add(1, Ordering::SeqCst);
            self.active.store(1, Ordering::SeqCst);
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            self.active.store(0, Ordering::SeqCst);
            Ok(())
        }

        fn is_active(&self) -> bool {
            self.active.load(Ordering::SeqCst) == 1
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_initializes_and_shutdowns_plugins() {
        let mut manager = PluginManager::<()>::new();
        let plugin = Arc::new(CountingPlugin::new());

        let setup = manager.plugin_count();
        manager.register_plugin(plugin.clone() as Arc<dyn Plugin<()>>);

        let actual = manager.plugin_count();
        let expected = setup + 1;
        assert_eq!(actual, expected);

        let services = Arc::new(());
        manager.initialize_all(services.clone()).await.unwrap();
        manager.initialize_all(services).await.unwrap();

        let actual_initializes = plugin.initializes.load(Ordering::SeqCst);
        let actual_active = <CountingPlugin as Plugin<()>>::is_active(&plugin);
        let expected_initializes = 1;
        assert_eq!(actual_initializes, expected_initializes);
        assert!(actual_active);

        let actual = manager.shutdown_all().await;
        assert!(actual.is_ok());

        let actual_shutdowns = plugin.shutdowns.load(Ordering::SeqCst);
        let actual_active = <CountingPlugin as Plugin<()>>::is_active(&plugin);
        let expected_shutdowns = 1;
        assert_eq!(actual_shutdowns, expected_shutdowns);
        assert!(!actual_active);
    }
}
