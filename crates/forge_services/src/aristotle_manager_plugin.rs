use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Result, Context};
use forge_fs::{resolve_symlink_chain, is_symlink_fast};
use async_trait::async_trait;
use tokio::process::Command as TokioCommand;

/// Plugin trait for ZOS integration
#[async_trait]
pub trait ZosPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, args: &[String]) -> Result<String>;
}

/// Aristotle Manager plugin for ZOS integration
pub struct AristotleManagerPlugin {
    aristotle_path: PathBuf,
}

impl AristotleManagerPlugin {
    pub fn new() -> Self {
        Self {
            aristotle_path: PathBuf::from("/mnt/data1/time-2026/05-may/07/arist"),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { aristotle_path: path }
    }

    /// Resolve symlinks with shmem integration for aristotle-manager paths
    async fn resolve_with_shmem(&self, path: &Path) -> Result<PathBuf> {
        let chain = resolve_symlink_chain(path).await?;
        
        // Get the final resolved path (last element in chain)
        let resolved_path = chain.last().unwrap_or(&chain[0]).clone();
        
        // Check if resolved path contains shmem reference
        if resolved_path.starts_with("/shmem/") || resolved_path.starts_with("/mnt/data1/shmem/") {
            Ok(resolved_path)
        } else {
            Ok(resolved_path)
        }
    }

    /// Execute aristotle-manager command with given arguments
    async fn run_command(&self, args: &[String]) -> Result<String> {
        let output = Command::new("cargo")
            .args(&["run", "--manifest-path", 
                &format!("{}/Cargo.toml", self.aristotle_path.display())])
            .args(args)
            .output()
            .context("Failed to execute aristotle-manager command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "aristotle-manager failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

impl Default for AristotleManagerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ZosPlugin for AristotleManagerPlugin {
    fn name(&self) -> &'static str {
        "aristotle-manager"
    }

    async fn execute(&self, args: &[String]) -> Result<String> {
        match args.first().map(|s| s.as_str()) {
            Some("fetch") => {
                let url = args.get(1).map(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("fetch requires a URL argument"))?;
                
                // Check if URL is actually a symlink path
                if Path::new(url).exists() && is_symlink_fast(Path::new(url)).await? {
                    let resolved = self.resolve_with_shmem(Path::new(url)).await?;
                    self.run_command(&[resolved.to_str().unwrap().to_string()]).await
                } else {
                    self.run_command(&[url.to_string()]).await
                }
            }
            Some("download") => {
                let repo = args.get(1).map(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("download requires a repository argument"))?;
                self.run_command(&[repo.to_string()]).await
            }
            Some("split") => {
                let project = args.get(1).map(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!("split requires a project argument"))?;
                
                if Path::new(project).exists() {
                    let resolved = self.resolve_with_shmem(Path::new(project)).await?;
                    self.run_command(&[resolved.to_str().unwrap().to_string()]).await
                } else {
                    self.run_command(&[project.to_string()]).await
                }
            }
            Some("index") => {
                self.run_command(&[]).await
            }
            Some("pipeline") => {
                self.run_command(&[]).await
            }
            _ => self.run_command(args).await,
        }
    }
}

/// Plugin registry
use std::sync::Arc;
use dashmap::DashMap;

pub struct PluginRegistry {
    plugins: DashMap<String, Arc<dyn ZosPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
        }
    }

    pub fn register(&self, plugin: Arc<dyn ZosPlugin>) {
        self.plugins.insert(plugin.name().to_string(), plugin);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn ZosPlugin>> {
        self.plugins.get(name).map(|p| p.clone())
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.key().clone()).collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// vendormod plugin for ZOS integration
pub struct VendormodPlugin {
    vendormod_path: PathBuf,
}

impl VendormodPlugin {
    pub fn new() -> Self {
        Self {
            vendormod_path: PathBuf::from("/home/mdupont/projects/cargo-clean/tools/cargo-vendormod"),
        }
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { vendormod_path: path.into() }
    }

    /// Run a vendormod subcommand and return stdout
    async fn run_vendormod(&self, args: &[&str]) -> Result<String> {
        let output = TokioCommand::new("cargo")
            .args(&["run", "--manifest-path"])
            .arg(&self.vendormod_path.join("Cargo.toml"))
            .args(&["--", "split"])
            .args(args)
            .output()
            .await
            .context("Failed to execute vendormod command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "vendormod failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

impl Default for VendormodPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ZosPlugin for VendormodPlugin {
    fn name(&self) -> &'static str {
        "vendormod"
    }

    async fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("vendormod plugin: use 'split', 'split-lean4', 'metadata', 'flake', 'scan'".into());
        }

        match args[0].as_str() {
            "split" => self.run_vendormod(&["--input-file", args.get(1).map(|s| s.as_str()).unwrap_or(".")]).await,
            "split-lean4" => {
                let src = args.get(1).map(|s| s.as_str()).unwrap_or(".");
                let out = args.get(2).map(|s| s.as_str()).unwrap_or("./mathlib-split");
                self.run_vendormod(&["split-lean4", "--mathlib-src", src, "--output-dir", out]).await
            }
            "metadata" => {
                let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
                self.run_vendormod(&["--input-file", path]).await
            }
            "flake" => {
                let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
                self.run_vendormod(&["--input-file", path]).await
            }
            "scan" => {
                self.run_vendormod(&[]).await
            }
            _ => self.run_vendormod(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await,
        }
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_REGISTRY: PluginRegistry = {
        let registry = PluginRegistry::new();
        registry.register(Arc::new(AristotleManagerPlugin::new()));
        registry.register(Arc::new(VendormodPlugin::new()));
        registry
    };
}

pub fn get_registry() -> &'static PluginRegistry {
    &GLOBAL_REGISTRY
}