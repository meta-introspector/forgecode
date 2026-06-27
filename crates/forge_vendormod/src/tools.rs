use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

/// vendormod tool definitions for Forge
pub struct VendormodTools {
    vendormod_path: PathBuf,
}

impl VendormodTools {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { vendormod_path: path.into() }
    }

    /// Run vendormod CLI with the given arguments
    fn run(&self, args: &[&str]) -> Result<String> {
        let manifest = self.vendormod_path.join("Cargo.toml");
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--manifest-path")
            .arg(&manifest)
            .args(args);

        let output = cmd.output().context("Failed to execute vendormod")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "vendormod failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub fn tool_split(&self, args: &serde_json::Value) -> Result<String> {
        let input = args
            .get("input_file")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        self.run(&["--input-file", input])
    }

    pub fn tool_split_lean4(&self, args: &serde_json::Value) -> Result<String> {
        let src = args
            .get("mathlib_src")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let out = args
            .get("output_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("./mathlib-split");
        self.run(&["split-lean4", "--mathlib-src", src, "--output-dir", out])
    }

    pub fn tool_metadata(&self, args: &serde_json::Value) -> Result<String> {
        let path = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        self.run(&["--input-file", path])
    }

    pub fn tool_flake(&self, args: &serde_json::Value) -> Result<String> {
        let path = args
            .get("crate_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        self.run(&["--input-file", path])
    }

    pub fn tool_scan(&self, _args: &serde_json::Value) -> Result<String> {
        self.run(&[])
    }
}
