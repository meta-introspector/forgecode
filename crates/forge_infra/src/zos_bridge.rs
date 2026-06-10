use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use dirs::home_dir;
use libloading::{Library, Symbol};

use crate::plugin::{Plugin, PluginManager};
use forge_config::ForgeConfig;
use forge_domain::{AnyProvider, Conversation, ConversationId, FileInfo};
use forge_app::{Services, ProviderAuthService};

/// Type definitions for ZOS plugin functions
type ZosPluginNameFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;
type ZosPluginInitFn = unsafe extern "C" fn() -> i32;
type ZosPluginDestroyFn = unsafe extern "C" fn() -> i32;

/// Wrapper for a loaded ZOS plugin that implements the Forge Plugin trait
struct ZosPluginWrapper<S: Services> {
    _library: Arc<Library>, // Keep the library loaded
    name: String,
    init_fn: ZosPluginInitFn,
    destroy_fn: ZosPluginDestroyFn,
    initialized: bool,
    _service_type: std::marker::PhantomData<S>,
}

impl<S: Services> ZosPluginWrapper<S> {
    /// Create a new ZosPluginWrapper from a loaded library
    ///
    /// # Arguments
    ///
    /// * `library` - The loaded shared library
    /// * `name` - The plugin name
    ///
    /// # Returns
    ///
    /// Result containing the wrapper or an error if required symbols are missing
    fn new(library: Library, name: String) -> Result<Self> {
        unsafe {
            // Get the required plugin functions
            let init_fn_symbol: Symbol<ZosPluginInitFn> = library.get(b"zos_plugin_init")
                .context("Failed to find zos_plugin_init symbol")?;
            let destroy_fn_symbol: Symbol<ZosPluginDestroyFn> = library.get(b"zos_plugin_destroy")
                .context("Failed to find zos_plugin_destroy symbol")?;
            let init_fn = *init_fn_symbol;
            let destroy_fn = *destroy_fn_symbol;
            Ok(Self {
                _library: Arc::new(library),
                name,
                init_fn,
                destroy_fn,
                initialized: false,
                _service_type: std::marker::PhantomData,
            })
        }
    }
}

#[async_trait::async_trait]
impl<S: Services> Plugin<S> for ZosPluginWrapper<S> {
    fn name(&self) -> &'static str {
        // Leak the string to get a 'static lifetime
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        "A ZOS plugin loaded via the ZOS plugin bridge"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    async fn initialize(&self, services: Arc<S>) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Call the plugin's init function
        let result = unsafe { (self.init_fn)() };
        if result != 0 {
            anyhow::bail!("Plugin {} initialization failed with exit code {}", self.name, result);
        }

        // TODO: Actually use the services parameter if needed by the plugin
        // For now, we just call the init function and assume the plugin
        // will use any global state it needs

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Call the plugin's destroy function
        let result = unsafe { (self.destroy_fn)() };
        if result != 0 {
            eprintln!("Warning: Plugin {} destroy function returned non-zero exit code: {}", self.name, result);
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.initialized
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Bridge for loading and managing ZOS plugins from the zos-server/plugins directory
pub struct ZosPluginBridge<S: Services> {
    plugins_dir: PathBuf,
    loaded_plugins: HashMap<String, ZosPluginWrapper<S>>,
}

impl<S: Services> ZosPluginBridge<S> {
    /// Create a new ZosPluginBridge
    ///
    /// # Arguments
    ///
    /// * `plugins_dir` - Directory containing ZOS plugins (default: ~/zos-server/plugins/)
    ///
    /// # Returns
    ///
    /// New ZosPluginBridge instance
    pub fn new<P: Into<PathBuf>>(plugins_dir: P) -> Self {
        Self {
            plugins_dir: plugins_dir.into(),
            loaded_plugins: HashMap::new(),
        }
    }

    /// Discovers and loads all ZOS plugins in the plugins directory
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn load_all_plugins(&mut self) -> Result<()> {
        // Read the plugins directory
        let entries = std::fs::read_dir(&self.plugins_dir)
            .with_context(|| format!("Failed to read plugins directory: {}", self.plugins_dir.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            // Check if this looks like a ZOS plugin (has Cargo.toml and lib.rs)
            let cargo_toml = path.join("Cargo.toml");
            let lib_rs = path.join("src").join("lib.rs");

            // For now, we'll look for pre-built .so files in a standard location
            // In a real implementation, we might want to build the plugins first
            let mut plugin_so_path = None;

            // Check if there's a target directory with .so files
            let target_dir = path.join("target");
            if target_dir.is_dir() {
                // Look for .so files in debug and release directories
                for subdir in ["debug", "release"] {
                    let plugin_dir = target_dir.join(subdir);
                    if plugin_dir.is_dir() {
                        if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
                            for entry in entries.flatten() {
                                let file_name = entry.file_name();
                                let file_name_str = file_name.to_string_lossy();
                                if file_name_str.starts_with("libzos_plugin_") && file_name_str.ends_with(".so") {
                                    plugin_so_path = Some(entry.path());
                                    break;
                                }
                            }
                        }
                    }
                    if plugin_so_path.is_some() {
                        break;
                    }
                }
            }

            // If we found a .so file, try to load it
            if let Some(so_path) = plugin_so_path {
                self.load_plugin_from_path(&so_path).await?;
            }
        }

        Ok(())
    }

    /// Loads a single ZOS plugin from a .so file path
    ///
    /// # Arguments
    ///
    /// * `so_path` - Path to the plugin .so file
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    async fn load_plugin_from_path(&mut self, so_path: &Path) -> Result<()> {
        // Load the library
        let library = unsafe { Library::new(so_path) }
            .with_context(|| format!("Failed to load plugin library: {}", so_path.display()))?;

        // Get the plugin name
        unsafe {
            let name_fn: Symbol<ZosPluginNameFn> = library.get(b"zos_plugin_name")
                .with_context(|| format!("Failed to find zos_plugin_name symbol in {}", so_path.display()))?;
            let c_str = CString::from_raw(name_fn() as *mut i8);
            let name = c_str.to_string_lossy().into_owned();

            // Create the wrapper
            let wrapper = ZosPluginWrapper::<S>::new(library, name)?;

            // Store the plugin
            self.loaded_plugins.insert(wrapper.name().to_string(), wrapper);
        }

        Ok(())
    }

    /// Gets a reference to a loaded plugin by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the plugin
    ///
    /// # Returns
    ///
    /// Optional reference to the plugin if found
    pub fn get_plugin(&self, name: &str) -> Option<&ZosPluginWrapper<S>> {
        self.loaded_plugins.get(name)
    }

    /// Gets a mutable reference to a loaded plugin by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the plugin
    ///
    /// # Returns
    ///
    /// Optional mutable reference to the plugin if found
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut ZosPluginWrapper<S>> {
        self.loaded_plugins.get_mut(name)
    }

    /// Returns the number of loaded plugins
    pub fn plugin_count(&self) -> usize {
        self.loaded_plugins.len()
    }

    /// Returns true if any plugins have been loaded
    pub fn has_plugins(&self) -> bool {
        !self.loaded_plugins.is_empty()
    }

    /// Consumes the bridge and returns all loaded plugins as plugin trait objects.
    pub fn into_loaded_plugins(self) -> Vec<Arc<dyn Plugin<S>>> {
        self.loaded_plugins
            .into_values()
            .map(|plugin| -> Arc<dyn Plugin<S>> { Arc::new(plugin) })
            .collect()
    }
}

impl<S: Services> Default for ZosPluginBridge<S> {
    fn default() -> Self {
        // Default to ~/zos-server/plugins/
        let mut plugins_dir = home_dir()
            .expect("Could not find home directory");
        plugins_dir.push("zos-server");
        plugins_dir.push("plugins");
        Self::new(plugins_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock Services trait for testing
    struct MockServices;
    #[async_trait::async_trait]
    impl Services for MockServices {
        async fn find_conversation(&self, _conversation_id: &ConversationId) -> Result<Option<Conversation>> {
            Ok(None)
        }
        async fn upsert_conversation(&self, _conversation: Conversation) -> Result<()> {
            Ok(())
        }
        fn get_config(&self) -> Result<ForgeConfig> {
            Ok(ForgeConfig::default())
        }
        fn get_environment(&self) -> forge_domain::Environment {
            forge_domain::Environment {
                os: "test".to_string(),
                cwd: std::path::PathBuf::from("/test"),
                home: std::path::PathBuf::from("/home/test"),
                shell: "/bin/bash".to_string(),
                base_path: ".forge".to_string(),
            }
        }
        async fn list_current_directory(&self) -> Result<Vec<FileInfo>> {
            Ok(vec![])
        }
        async fn get_custom_instructions(&self) -> Result<Option<String>> {
            Ok(None)
        }
        async fn get_agent(&self, _agent_id: &forge_domain::AgentId) -> Result<Option<forge_domain::Agent>> {
            Ok(None)
        }
        async fn get_all_providers(&self) -> Result<Vec<AnyProvider>> {
            Ok(vec![])
        }
        async fn provider_auth_service(&self) -> Arc<dyn ProviderAuthService> {
            Arc::new(MockProviderAuthService)
        }
        async fn initialize_plugins(&self) -> Result<()> {
            Ok(())
        }
    }

    struct MockProviderAuthService;
    #[async_trait::async_trait]
    impl ProviderAuthService for MockProviderAuthService {
        async fn refresh_provider_credential(&self, _provider: AnyProvider) -> Result<AnyProvider> {
            Ok(_provider)
        }
    }

    #[tokio::test]
    async fn test_load_all_plugins_from_actual_directory() {
        // Use the actual zos-server/plugins directory
        let plugins_dir = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("zos-server").join("plugins");
        
        // Skip test if plugins directory doesn't exist
        if !plugins_dir.exists() {
            println!("Skipping test: plugins directory does not exist at {}", plugins_dir.display());
            return;
        }

        let mut bridge = ZosPluginBridge::<MockServices>::new(plugins_dir);
        let result = bridge.load_all_plugins().await;
        
        // Should succeed in loading plugins
        assert!(result.is_ok(), "Failed to load plugins: {:?}", result.err());
        
        // Should have loaded at least our test plugins
        let count = bridge.plugin_count();
        println!("Loaded {} plugins", count);
        
        // Check that we can get specific plugins
        let generators_plugin = bridge.get_plugin("generators");
        assert!(generators_plugin.is_some(), "Should be able to get generators plugin");
        
        let git_tools_plugin = bridge.get_plugin("git-tools");
        assert!(git_tools_plugin.is_some(), "Should be able to get git-tools plugin");
        
        // Test that plugins are initialized
        if let Some(plugin) = generators_plugin {
            assert!(!plugin.is_active(), "Plugin should not be active until initialized");
            
            // Initialize the plugin
            let services = Arc::new(MockServices);
            let init_result = plugin.initialize(services.clone()).await;
            assert!(init_result.is_ok(), "Failed to initialize generators plugin: {:?}", init_result.err());
            assert!(plugin.is_active(), "Plugin should be active after initialization");
            
            // Test shutdown
            let shutdown_result = plugin.shutdown().await;
            assert!(shutdown_result.is_ok(), "Failed to shutdown generators plugin: {:?}", shutdown_result.err());
            assert!(!plugin.is_active(), "Plugin should not be active after shutdown");
        }
    }

    #[tokio::test]
    async fn test_load_specific_plugin() {
        // Test loading a specific plugin by path
        let generators_so_path = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("zos-server")
            .join("plugins")
            .join("generators")
            .join("target")
            .join("debug")
            .join("libzos_plugin_generators.so");
            
        // Skip test if plugin doesn't exist
        if !generators_so_path.exists() {
            println!("Skipping test: generators plugin not found at {}", generators_so_path.display());
            return;
        }

        let mut bridge = ZosPluginBridge::<MockServices>::new(std::path::PathBuf::new());
        let result = bridge.load_plugin_from_path(&generators_so_path).await;
        
        assert!(result.is_ok(), "Failed to load generators plugin: {:?}", result.err());
        assert_eq!(bridge.plugin_count(), 1, "Should have loaded exactly one plugin");
        
        let plugin = bridge.get_plugin("generators");
        assert!(plugin.is_some(), "Should be able to get generators plugin");
        
        if let Some(plugin) = plugin {
            // Test that we can initialize it
            let services = Arc::new(MockServices);
            let init_result = plugin.initialize(services.clone()).await;
            assert!(init_result.is_ok(), "Failed to initialize generators plugin: {:?}", init_result.err());
        }
    }
}