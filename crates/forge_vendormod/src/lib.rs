use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command as TokioCommand;

pub mod tools;

/// vendormod ZOS Plugin + Forge Tool Provider
///
/// Exposes vendormod workflows (split, metadata, flake generation, scan)
/// as ZOS plugins and Forge tools.
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
impl forge_services::ZosPlugin for VendormodPlugin {
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
