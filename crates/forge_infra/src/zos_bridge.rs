use std::collections::HashMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use dirs::home_dir;
use libloading::{Library, Symbol};

use crate::plugin::Plugin;

/// Type definitions for ZOS plugin functions.
type ZosPluginNameFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;
type ZosPluginInitFn = unsafe extern "C" fn() -> i32;
type ZosPluginDestroyFn = unsafe extern "C" fn() -> i32;

/// Wrapper for a loaded ZOS plugin that implements the Forge Plugin trait.
pub(crate) struct ZosPluginWrapper<S> {
    _library: Arc<Library>, // Keep the library loaded.
    name: String,
    init_fn: ZosPluginInitFn,
    destroy_fn: ZosPluginDestroyFn,
    initialized: AtomicBool,
    _service_type: std::marker::PhantomData<S>,
}

impl<S: Send + Sync + 'static> ZosPluginWrapper<S> {
    /// Create a new ZosPluginWrapper from a loaded library.
    ///
    /// # Arguments
    ///
    /// * `library` - The loaded shared library
    /// * `name` - The plugin name
    ///
    /// # Returns
    ///
    /// Result containing the wrapper or an error if required symbols are missing.
    fn new(library: Library, name: String) -> Result<Self> {
        unsafe {
            let init_fn_symbol: Symbol<ZosPluginInitFn> = library
                .get(b"zos_plugin_init")
                .context("Failed to find zos_plugin_init symbol")?;
            let destroy_fn_symbol: Symbol<ZosPluginDestroyFn> = library
                .get(b"zos_plugin_destroy")
                .context("Failed to find zos_plugin_destroy symbol")?;
            let init_fn = *init_fn_symbol;
            let destroy_fn = *destroy_fn_symbol;

            Ok(Self {
                _library: Arc::new(library),
                name,
                init_fn,
                destroy_fn,
                initialized: AtomicBool::new(false),
                _service_type: std::marker::PhantomData,
            })
        }
    }
}

#[async_trait::async_trait]
impl<S: Send + Sync + 'static> Plugin<S> for ZosPluginWrapper<S> {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A ZOS plugin loaded via the ZOS plugin bridge"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn initialize(&self, _services: Arc<S>) -> Result<()> {
        if self
            .initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }

        let result = unsafe { (self.init_fn)() };
        if result != 0 {
            self.initialized.store(false, Ordering::SeqCst);
            anyhow::bail!(
                "Plugin {} initialization failed with exit code {}",
                self.name,
                result
            );
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if !self
            .initialized
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Ok(());
        }

        let result = unsafe { (self.destroy_fn)() };
        if result != 0 {
            tracing::warn!(
                plugin = self.name.as_str(),
                exit_code = result,
                "ZOS plugin destroy function returned non-zero exit code"
            );
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Bridge for loading and managing ZOS plugins from the zos-server/plugins directory.
pub struct ZosPluginBridge<S> {
    plugins_dir: PathBuf,
    loaded_plugins: HashMap<String, ZosPluginWrapper<S>>,
}

impl<S: Send + Sync + 'static> ZosPluginBridge<S> {
    /// Create a new ZosPluginBridge.
    ///
    /// # Arguments
    ///
    /// * `plugins_dir` - Directory containing ZOS plugins (default: ~/zos-server/plugins/)
    ///
    /// # Returns
    ///
    /// New ZosPluginBridge instance.
    pub fn new<P: Into<PathBuf>>(plugins_dir: P) -> Self {
        Self {
            plugins_dir: plugins_dir.into(),
            loaded_plugins: HashMap::new(),
        }
    }

    /// Returns the configured plugin directory.
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Discovers and loads all ZOS plugins in the plugins directory.
    ///
    /// Missing plugin directories are treated as an empty plugin set so Forge can
    /// start even when the optional ZOS plugin tree has not been installed.
    ///
    /// # Returns
    ///
    /// Result indicating success or failure.
    pub async fn load_all_plugins(&mut self) -> Result<()> {
        if !self.plugins_dir.exists() {
            tracing::debug!(
                plugins_dir = %self.plugins_dir.display(),
                "ZOS plugin directory does not exist"
            );
            return Ok(());
        }

        if !self.plugins_dir.is_dir() {
            anyhow::bail!(
                "ZOS plugins path is not a directory: {}",
                self.plugins_dir.display()
            );
        }

        let entries = std::fs::read_dir(&self.plugins_dir).with_context(|| {
            format!(
                "Failed to read ZOS plugins directory: {}",
                self.plugins_dir.display()
            )
        })?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() && is_dynamic_library(&path) {
                self.load_plugin_from_path(&path).await?;
                continue;
            }

            if !path.is_dir() {
                continue;
            }

            if let Some(plugin_so_path) = find_plugin_library(&path) {
                self.load_plugin_from_path(&plugin_so_path).await?;
            }
        }

        Ok(())
    }

    /// Loads a single ZOS plugin from a dynamic library path.
    ///
    /// # Arguments
    ///
    /// * `so_path` - Path to the plugin dynamic library
    ///
    /// # Returns
    ///
    /// Result indicating success or failure.
    pub async fn load_plugin_from_path(&mut self, so_path: &Path) -> Result<()> {
        let library = unsafe { Library::new(so_path) }
            .with_context(|| format!("Failed to load plugin library: {}", so_path.display()))?;

        unsafe {
            let name_fn: Symbol<ZosPluginNameFn> =
                library.get(b"zos_plugin_name").with_context(|| {
                    format!(
                        "Failed to find zos_plugin_name symbol in {}",
                        so_path.display()
                    )
                })?;

            let name_ptr = name_fn();
            if name_ptr.is_null() {
                anyhow::bail!(
                    "zos_plugin_name returned a null pointer in {}",
                    so_path.display()
                );
            }

            let name = CStr::from_ptr(name_ptr)
                .to_str()
                .with_context(|| {
                    format!("Plugin name in {} is not valid UTF-8", so_path.display())
                })?
                .to_string();

            let wrapper = ZosPluginWrapper::<S>::new(library, name)?;
            self.loaded_plugins
                .insert(wrapper.name().to_string(), wrapper);
        }

        Ok(())
    }

    /// Returns the number of loaded plugins.
    pub fn plugin_count(&self) -> usize {
        self.loaded_plugins.len()
    }

    /// Returns true if any plugins have been loaded.
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

impl<S: Send + Sync + 'static> Default for ZosPluginBridge<S> {
    fn default() -> Self {
        let mut plugins_dir = home_dir().expect("Could not find home directory");
        plugins_dir.push("zos-server");
        plugins_dir.push("plugins");
        Self::new(plugins_dir)
    }
}

#[async_trait::async_trait]
impl<S: Send + Sync + 'static> Plugin<S> for ZosPluginBridge<S> {
    fn name(&self) -> &str {
        "zos-plugin-bridge"
    }

    fn description(&self) -> &str {
        "Loads and manages ZOS dynamic plugins"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn initialize(&self, services: Arc<S>) -> Result<()> {
        for plugin in self.loaded_plugins.values() {
            plugin.initialize(services.clone()).await?;
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let mut plugins: Vec<_> = self.loaded_plugins.values().collect();
        plugins.reverse();

        for plugin in plugins {
            plugin.shutdown().await?;
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.loaded_plugins
            .values()
            .any(|plugin| plugin.is_active())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn is_dynamic_library(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("so" | "dylib" | "dll")
    )
}

fn find_plugin_library(plugin_dir: &Path) -> Option<PathBuf> {
    find_library_file(plugin_dir)
        .or_else(|| find_library_file(&plugin_dir.join("target").join("debug")))
        .or_else(|| find_library_file(&plugin_dir.join("target").join("release")))
}

fn find_library_file(directory: &Path) -> Option<PathBuf> {
    if directory.is_file() && is_dynamic_library(directory) {
        return Some(directory.to_path_buf());
    }

    if !directory.is_dir() {
        return None;
    }

    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_file() && is_dynamic_library(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginManager;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_load_all_plugins_missing_directory_is_ok() {
        let plugins_dir = tempfile::tempdir().unwrap().path().join("missing");
        let mut bridge = ZosPluginBridge::<()>::new(plugins_dir);

        let actual = bridge.load_all_plugins().await;

        assert!(actual.is_ok());
        assert_eq!(bridge.plugin_count(), 0);
    }

    #[tokio::test]
    async fn test_load_all_plugins_empty_directory_is_ok() {
        let plugins_dir = tempfile::tempdir().unwrap();
        let mut bridge = ZosPluginBridge::<()>::new(plugins_dir.path());

        let actual = bridge.load_all_plugins().await;

        assert!(actual.is_ok());
        assert_eq!(bridge.plugin_count(), 0);
    }

    #[tokio::test]
    async fn test_zos_plugin_bridge_lifecycle_without_plugins() {
        let bridge = ZosPluginBridge::<()>::new(tempfile::tempdir().unwrap().path());

        let setup = Arc::new(());
        let actual = bridge.initialize(setup.clone()).await;
        assert!(actual.is_ok());
        assert!(!bridge.is_active());

        let actual = bridge.shutdown().await;
        assert!(actual.is_ok());
        assert!(!bridge.is_active());
    }

    #[test]
    fn test_plugin_manager_clone_preserves_plugins() {
        let bridge = ZosPluginBridge::<()>::new(tempfile::tempdir().unwrap().path());
        let mut manager = PluginManager::new();

        let setup = manager.plugin_count();
        manager.register_plugin(Arc::new(bridge));
        let cloned = manager.clone();

        let actual = cloned.plugin_count();
        let expected = setup + 1;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_dynamic_library_recognizes_common_extensions() {
        let setup = [
            PathBuf::from("libzos_plugin_example.so"),
            PathBuf::from("zos_plugin_example.dylib"),
            PathBuf::from("zos_plugin_example.dll"),
            PathBuf::from("zos_plugin_example.txt"),
        ];

        let actual: Vec<_> = setup.iter().map(|path| is_dynamic_library(path)).collect();
        let expected = vec![true, true, true, false];

        assert_eq!(actual, expected);
    }
}
