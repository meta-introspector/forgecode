use std::collections::HashMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use dirs::home_dir;
use forge_domain::PluginInfo;
use libloading::{Library, Symbol};
use serde::Deserialize;
use toml_edit::de::from_str as from_toml_str;

use crate::plugin::Plugin;

/// Type definitions for ZOS plugin functions.
type ZosPluginNameFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;
type ZosPluginInitFn = unsafe extern "C" fn() -> i32;
type ZosPluginShutdownFn = unsafe extern "C" fn() -> i32;
type ZosPluginMetadataFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

pub(crate) struct ZosPluginWrapper<S> {
    _library: Arc<Library>, // Keep the library loaded.
    name: String,
    description: String,
    version: String,
    init_fn: ZosPluginInitFn,
    shutdown_fn: ZosPluginShutdownFn,
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
            let init_fn = *library
                .get::<ZosPluginInitFn>(b"zos_plugin_init")
                .context("Failed to find zos_plugin_init symbol")?;
            let shutdown_fn = load_shutdown_fn(&library)?;
            let description = load_optional_c_string_symbol(&library, b"zos_plugin_description")
                .unwrap_or_else(|| "A ZOS plugin loaded via the ZOS plugin bridge".to_string());
            let version = load_optional_c_string_symbol(&library, b"zos_plugin_version")
                .unwrap_or_else(|| "0.1.0".to_string());

            Ok(Self {
                _library: Arc::new(library),
                name,
                description,
                version,
                init_fn,
                shutdown_fn,
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
        &self.description
    }

    fn version(&self) -> &str {
        &self.version
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
        if self
            .initialized
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }

        let result = unsafe { (self.shutdown_fn)() };
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
        let manifest_path = self.plugins_dir.join("plugins.toml");
        if manifest_path.exists() {
            self.load_plugins_from_manifest(&manifest_path).await?;
            return Ok(());
        }

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
                if is_forge_zos_plugin_library(&path)? {
                    self.load_plugin_from_path(&path).await?;
                } else {
                    tracing::debug!(
                        plugin = path.display().to_string(),
                        "Skipping non-Forge ZOS plugin library without zos_plugin_name"
                    );
                }
                continue;
            }

            if !path.is_dir() {
                continue;
            }

            if let Some(plugin_so_path) = find_plugin_library(&path) {
                if is_forge_zos_plugin_library(&plugin_so_path)? {
                    self.load_plugin_from_path(&plugin_so_path).await?;
                } else {
                    tracing::debug!(
                        plugin = plugin_so_path.display().to_string(),
                        "Skipping non-Forge ZOS plugin library without zos_plugin_name"
                    );
                }
            }
        }

        Ok(())
    }

    /// Loads the default plugin set from the TOML manifest.
    ///
    /// The manifest is intentionally the only runtime source of plugin paths when
    /// it exists. This lets ZOS use Nix store outputs as the loader input while
    /// keeping local discovery available for tests and older plugin layouts.
    ///
    /// # Arguments
    ///
    /// * `manifest_path` - Path to the ZOS plugin TOML manifest
    ///
    /// # Returns
    ///
    /// Result indicating success or failure.
    async fn load_plugins_from_manifest(&mut self, manifest_path: &Path) -> Result<()> {
        let manifest_text = std::fs::read_to_string(manifest_path).with_context(|| {
            format!(
                "Failed to read ZOS plugin manifest: {}",
                manifest_path.display()
            )
        })?;
        let manifest: PluginManifest = from_toml_str(&manifest_text).with_context(|| {
            format!(
                "Failed to parse ZOS plugin manifest: {}",
                manifest_path.display()
            )
        })?;

        for entry in manifest.plugins {
            let entry_name = entry.name.clone();
            self.load_nix_manifest_plugin(entry)
                .await
                .with_context(|| format!("Failed to load ZOS plugin `{entry_name}`"))?;
        }

        Ok(())
    }

    async fn load_nix_manifest_plugin(&mut self, entry: PluginManifestEntry) -> Result<()> {
        let shared_object = entry.shared_object.clone();
        let store_path = entry.store_path.clone().or_else(|| {
            shared_object
                .as_ref()
                .and_then(|path| path.parent().map(Path::to_path_buf))
        });
        let store_path = store_path.as_ref().with_context(|| {
            format!(
                "ZOS plugin `{}` manifest entry must include either store_path or shared_object",
                entry.name
            )
        })?;
        let plugin_so_path = match shared_object {
            Some(path) => path,
            None => find_plugin_library(store_path).with_context(|| {
                format!(
                    "ZOS plugin `{}` has no shared object in Nix store output {}",
                    entry.name,
                    store_path.display()
                )
            })?,
        };

        tracing::debug!(
            plugin = entry.name.as_str(),
            store_path = %store_path.display(),
            flake = entry.flake.as_ref().map(|path| path.display().to_string()),
            nora = entry.nora.as_deref(),
            shared_object = plugin_so_path.display().to_string(),
            "Loading ZOS plugin from Nix store manifest"
        );

        if !is_nix_store_path(store_path) {
            anyhow::bail!(
                "ZOS plugin `{}` store_path must point to a Nix store output, got {}",
                entry.name,
                store_path.display()
            );
        }

        if !is_nix_store_path(&plugin_so_path) {
            anyhow::bail!(
                "ZOS plugin `{}` shared_object {} must point to a Nix store output",
                entry.name,
                plugin_so_path.display()
            );
        }

        if !plugin_so_path.starts_with(store_path) {
            anyhow::bail!(
                "ZOS plugin `{}` shared_object {} must be inside store_path {}",
                entry.name,
                plugin_so_path.display(),
                store_path.display()
            );
        }

        if is_forge_zos_plugin_library(&plugin_so_path)? {
            self.load_plugin_from_path(&plugin_so_path).await?;
        } else {
            tracing::debug!(
                plugin = plugin_so_path.display().to_string(),
                "Skipping non-Forge ZOS plugin library without zos_plugin_name"
            );
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

    /// Lists loaded ZOS plugins as lightweight metadata sorted by name.
    pub fn list_loaded_plugins(&self) -> Vec<PluginInfo> {
        let mut plugins: Vec<_> = self
            .loaded_plugins
            .values()
            .map(|plugin| PluginInfo {
                name: plugin.name().to_string(),
                description: plugin.description().to_string(),
                version: plugin.version().to_string(),
                active: plugin.is_active(),
            })
            .collect();
        plugins.sort_by(|left, right| left.name.cmp(&right.name));
        plugins
    }

    /// Searches loaded ZOS plugins by name, description, or version.
    ///
    /// An empty query returns all loaded plugins.
    pub fn search_loaded_plugins(&self, query: &str) -> Vec<PluginInfo> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.list_loaded_plugins();
        }

        self.list_loaded_plugins()
            .into_iter()
            .filter(|plugin| {
                plugin.name.to_ascii_lowercase().contains(&query)
                    || plugin.description.to_ascii_lowercase().contains(&query)
                    || plugin.version.to_ascii_lowercase().contains(&query)
            })
            .collect()
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

#[derive(Debug, Deserialize)]
struct PluginManifest {
    plugins: Vec<PluginManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct PluginManifestEntry {
    name: String,
    store_path: Option<PathBuf>,
    flake: Option<PathBuf>,
    nora: Option<String>,
    shared_object: Option<PathBuf>,
}

#[cfg(not(test))]
fn is_nix_store_path(path: &Path) -> bool {
    path.starts_with(Path::new("/nix/store"))
}

#[cfg(test)]
fn is_nix_store_path(path: &Path) -> bool {
    path.starts_with(Path::new("/nix/store"))
        || path
            .components()
            .any(|component| component.as_os_str() == "nix-store")
}

fn is_forge_zos_plugin_library(path: &Path) -> Result<bool> {
    let library = unsafe { Library::new(path) }
        .with_context(|| format!("Failed to load plugin library: {}", path.display()))?;

    Ok(unsafe { library.get::<ZosPluginNameFn>(b"zos_plugin_name").is_ok() })
}

fn load_optional_c_string_symbol(library: &Library, name: &[u8]) -> Option<String> {
    unsafe {
        let Ok(symbol) = library.get::<ZosPluginMetadataFn>(name) else {
            return None;
        };
        let ptr = symbol();
        if ptr.is_null() {
            return None;
        }

        CStr::from_ptr(ptr).to_str().ok().map(str::to_string)
    }
}

fn load_shutdown_fn(library: &Library) -> Result<ZosPluginShutdownFn> {
    match unsafe { library.get::<ZosPluginShutdownFn>(b"zos_plugin_destroy") } {
        Ok(symbol) => Ok(*symbol),
        Err(destroy_error) => unsafe { library.get::<ZosPluginShutdownFn>(b"zos_plugin_shutdown") }
            .map(|symbol| *symbol)
            .with_context(|| {
                format!(
                    "Failed to find zos_plugin_destroy or zos_plugin_shutdown symbol: {destroy_error}"
                )
            }),
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
    use pretty_assertions::assert_eq;
    use std::path::{Path, PathBuf};
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

    #[tokio::test]
    async fn test_zos_plugin_bridge_lists_loaded_plugins() {
        let setup = tempfile::tempdir().unwrap();
        compile_zos_plugin_fixture_with_name(
            setup.path(),
            "zos_plugin_destroy",
            "beta-plugin",
            Some("Beta searchable plugin"),
            Some("0.2.0"),
        );
        compile_zos_plugin_fixture_with_name(
            setup.path(),
            "zos_plugin_destroy",
            "alpha-plugin",
            Some("Alpha searchable plugin"),
            Some("0.1.0"),
        );
        let mut bridge = ZosPluginBridge::<()>::new(setup.path());

        bridge.load_all_plugins().await.unwrap();
        let actual = bridge.list_loaded_plugins();
        let expected = vec![
            PluginInfo {
                name: "alpha-plugin".to_string(),
                description: "Alpha searchable plugin".to_string(),
                version: "0.1.0".to_string(),
                active: false,
            },
            PluginInfo {
                name: "beta-plugin".to_string(),
                description: "Beta searchable plugin".to_string(),
                version: "0.2.0".to_string(),
                active: false,
            },
        ];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_zos_plugin_bridge_searches_loaded_plugins() {
        let setup = tempfile::tempdir().unwrap();
        compile_zos_plugin_fixture_with_name(
            setup.path(),
            "zos_plugin_destroy",
            "alpha-plugin",
            Some("Alpha searchable plugin"),
            Some("0.1.0"),
        );
        compile_zos_plugin_fixture_with_name(
            setup.path(),
            "zos_plugin_destroy",
            "beta-plugin",
            Some("Beta searchable plugin"),
            Some("0.2.0"),
        );
        let mut bridge = ZosPluginBridge::<()>::new(setup.path());
        bridge.load_all_plugins().await.unwrap();

        let actual = bridge.search_loaded_plugins("beta");
        let expected = vec![PluginInfo {
            name: "beta-plugin".to_string(),
            description: "Beta searchable plugin".to_string(),
            version: "0.2.0".to_string(),
            active: false,
        }];
        assert_eq!(actual, expected);

        let actual = bridge.search_loaded_plugins("0.1.0");
        let expected = vec![PluginInfo {
            name: "alpha-plugin".to_string(),
            description: "Alpha searchable plugin".to_string(),
            version: "0.1.0".to_string(),
            active: false,
        }];
        assert_eq!(actual, expected);

        let actual = bridge.search_loaded_plugins("missing");
        let expected = Vec::<PluginInfo>::new();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_load_all_plugins_from_manifest_only() {
        let setup = tempfile::tempdir().unwrap();
        let plugins_dir = setup.path().join("plugins");
        let nix_store_dir = setup.path().join("nix-store");
        std::fs::create_dir_all(&plugins_dir).unwrap();
        std::fs::create_dir_all(&nix_store_dir).unwrap();
        compile_zos_plugin_fixture_with_name(
            &nix_store_dir,
            "zos_plugin_destroy",
            "manifest-plugin",
            Some("Manifest searchable plugin"),
            Some("0.3.0"),
        );
        std::fs::write(
            plugins_dir.join("plugins.toml"),
            format!(
                r#"
[[plugins]]
name = "manifest-plugin"
store_path = "{}"
flake = "{}"
nora = "zos-plugin-manifest-plugin"
"#,
                nix_store_dir.display(),
                nix_store_dir.join("flake.nix").display()
            ),
        )
        .unwrap();

        let mut bridge = ZosPluginBridge::<()>::new(&plugins_dir);
        bridge.load_all_plugins().await.unwrap();
        let actual = bridge.list_loaded_plugins();
        let expected = vec![PluginInfo {
            name: "manifest-plugin".to_string(),
            description: "Manifest searchable plugin".to_string(),
            version: "0.3.0".to_string(),
            active: false,
        }];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_load_all_plugins_from_manifest_with_shared_object_only() {
        let setup = tempfile::tempdir().unwrap();
        let plugins_dir = setup.path().join("plugins");
        let nix_store_dir = setup.path().join("nix-store");
        std::fs::create_dir_all(&plugins_dir).unwrap();
        std::fs::create_dir_all(&nix_store_dir).unwrap();
        let plugin_so = compile_zos_plugin_fixture_with_name(
            &nix_store_dir,
            "zos_plugin_destroy",
            "manifest-plugin",
            Some("Manifest searchable plugin"),
            Some("0.4.0"),
        );
        std::fs::write(
            plugins_dir.join("plugins.toml"),
            format!(
                r#"
[[plugins]]
name = "manifest-plugin"
flake = "{}"
nora = "zos-plugin-manifest-plugin"
shared_object = "{}"
"#,
                nix_store_dir.join("flake.nix").display(),
                plugin_so.display()
            ),
        )
        .unwrap();

        let mut bridge = ZosPluginBridge::<()>::new(&plugins_dir);
        bridge.load_all_plugins().await.unwrap();
        let actual = bridge.list_loaded_plugins();
        let expected = vec![PluginInfo {
            name: "manifest-plugin".to_string(),
            description: "Manifest searchable plugin".to_string(),
            version: "0.4.0".to_string(),
            active: false,
        }];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_load_all_plugins_rejects_non_nix_manifest_store_path() {
        let setup = tempfile::tempdir().unwrap();
        let plugins_dir = setup.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();
        std::fs::write(
            plugins_dir.join("plugins.toml"),
            r#"
[[plugins]]
name = "local-plugin"
store_path = "/tmp/local.so"
"#,
        )
        .unwrap();

        let mut bridge = ZosPluginBridge::<()>::new(&plugins_dir);
        let actual = bridge.load_all_plugins().await;

        assert!(actual.is_err());
        assert_eq!(bridge.plugin_count(), 0);
    }

    #[tokio::test]
    async fn test_zos_plugin_bridge_lifecycle_initializes_loaded_plugins() {
        let setup = tempfile::tempdir().unwrap();
        compile_zos_plugin_fixture_with_name(
            setup.path(),
            "zos_plugin_destroy",
            "test-plugin",
            None,
            None,
        );
        let mut bridge = ZosPluginBridge::<()>::new(setup.path());
        bridge.load_all_plugins().await.unwrap();

        let actual = bridge.initialize(Arc::new(())).await;
        assert!(actual.is_ok());
        assert!(bridge.is_active());

        let actual = bridge.shutdown().await;
        assert!(actual.is_ok());
        assert!(!bridge.is_active());
    }

    fn compile_zos_plugin_fixture(temp_dir: &Path, shutdown_symbol: &str) -> PathBuf {
        compile_zos_plugin_fixture_with_name(temp_dir, shutdown_symbol, "test-plugin", None, None)
    }

    fn compile_zos_plugin_fixture_with_name(
        temp_dir: &Path,
        shutdown_symbol: &str,
        plugin_name: &str,
        description: Option<&str>,
        version: Option<&str>,
    ) -> PathBuf {
        let safe_name = plugin_name
            .chars()
            .map(|character| {
                if character == '-' || character == '_' {
                    '_'
                } else {
                    character
                }
            })
            .collect::<String>();
        let source = temp_dir.join(format!("plugin_{safe_name}.rs"));
        let description_fn = description
            .map(|description| {
                format!(
                    r#"
#[no_mangle]
pub extern "C" fn zos_plugin_description() -> *const c_char {{
    b"{description}\0".as_ptr() as *const c_char
}}
"#
                )
            })
            .unwrap_or_default();
        let version_fn = version
            .map(|version| {
                format!(
                    r#"
#[no_mangle]
pub extern "C" fn zos_plugin_version() -> *const c_char {{
    b"{version}\0".as_ptr() as *const c_char
}}
"#
                )
            })
            .unwrap_or_default();
        std::fs::write(
            &source,
            format!(
                r#"
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn zos_plugin_name() -> *const c_char {{
    b"{plugin_name}\0".as_ptr() as *const c_char
}}

{description_fn}

{version_fn}

#[no_mangle]
pub extern "C" fn zos_plugin_init() -> i32 {{ 0 }}

#[no_mangle]
pub extern "C" fn {shutdown_symbol}() -> i32 {{ 0 }}
"#
            ),
        )
        .unwrap();

        let library_name = format!(
            "{}{safe_name}.{}",
            if cfg!(windows) { "" } else { "lib" },
            std::env::consts::DLL_EXTENSION
        );
        let output = temp_dir.join(library_name);

        let status = std::process::Command::new("rustc")
            .arg("--crate-type=cdylib")
            .arg("-o")
            .arg(&output)
            .arg(&source)
            .status()
            .unwrap();

        assert!(status.success());
        output
    }

    #[test]
    fn test_load_shutdown_fn_accepts_destroy_symbol() {
        let setup = tempfile::tempdir().unwrap();
        let library_path = compile_zos_plugin_fixture(setup.path(), "zos_plugin_destroy");
        let library = unsafe { Library::new(&library_path).unwrap() };

        let actual = load_shutdown_fn(&library)
            .map(|_| "loaded".to_string())
            .map_err(|error| error.to_string());
        let expected = Ok("loaded".to_string());

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_load_shutdown_fn_accepts_shutdown_symbol() {
        let setup = tempfile::tempdir().unwrap();
        let library_path = compile_zos_plugin_fixture(setup.path(), "zos_plugin_shutdown");
        let library = unsafe { Library::new(&library_path).unwrap() };

        let actual = load_shutdown_fn(&library)
            .map(|_| "loaded".to_string())
            .map_err(|error| error.to_string());
        let expected = Ok("loaded".to_string());

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_load_shutdown_fn_requires_shutdown_symbol() {
        let setup = tempfile::tempdir().unwrap();
        let source = setup.path().join("plugin.rs");
        std::fs::write(
            &source,
            r#"
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn zos_plugin_name() -> *const c_char {
    b"test-plugin\0".as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn zos_plugin_init() -> i32 { 0 }
"#,
        )
        .unwrap();

        let library_name = format!(
            "{}test_plugin.{}",
            if cfg!(windows) { "" } else { "lib" },
            std::env::consts::DLL_EXTENSION
        );
        let library_path = setup.path().join(library_name);
        let status = std::process::Command::new("rustc")
            .arg("--crate-type=cdylib")
            .arg("-o")
            .arg(&library_path)
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success());

        let library = unsafe { Library::new(&library_path).unwrap() };

        let actual = load_shutdown_fn(&library).unwrap_err().to_string();
        let expected = "Failed to find zos_plugin_destroy or zos_plugin_shutdown symbol";

        assert!(actual.contains(expected));
    }

    #[tokio::test]
    async fn test_load_all_plugins_from_default_directory() {
        let plugins_dir = dirs::home_dir()
            .expect("home directory should be available")
            .join("zos-server")
            .join("plugins");

        if !plugins_dir.exists() {
            return;
        }

        let mut bridge = ZosPluginBridge::<()>::new(&plugins_dir);
        let actual = bridge.load_all_plugins().await;

        assert!(
            actual.is_ok(),
            "failed to load ZOS plugins from {}: {actual:?}",
            plugins_dir.display()
        );
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
