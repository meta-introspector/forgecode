use std::path::{Path, PathBuf};
use bstr::ByteSlice;
use anyhow::{Context, Result};
use crate::symlink::resolve_symlink;
use crate::symlink::is_symlink;

impl crate::ForgeFS {
    pub async fn read_utf8<T: AsRef<Path>>(path: T) -> Result<String> {
        if is_symlink(path.as_ref()).await? {
            let resolved_path: PathBuf = resolve_symlink(path.as_ref()).await?;
            let path_for_read = resolved_path.clone();
            Self::read(path_for_read).await
                .map(|bytes| bytes.to_str_lossy().to_string())
                .with_context(|| format!("Failed to read symlink-expanded content: {}", resolved_path.display()))
        } else {
            Self::read(path.as_ref()).await
                .map(|bytes| bytes.to_str_lossy().to_string())
                .with_context(|| format!("Failed to read content: {}", path.as_ref().display()))
        }
    }

    pub async fn read<T: AsRef<Path>>(path: T) -> Result<Vec<u8>> {
        let path_ref = path.as_ref();
        if is_symlink(path_ref).await? {
            let resolved_path: PathBuf = resolve_symlink(path_ref).await?;
            tokio::fs::read(&resolved_path)
                .await
                .with_context(|| format!("Failed to read symlink-expanded file: {}", resolved_path.display()))
        } else {
            tokio::fs::read(path_ref)
                .await
                .with_context(|| format!("Failed to read file: {}", path_ref.display()))
        }
    }

    pub async fn read_to_string<T: AsRef<Path>>(path: T) -> Result<String> {
        if is_symlink(path.as_ref()).await? {
            let resolved_path: PathBuf = resolve_symlink(path.as_ref()).await?;
            tokio::fs::read_to_string(&resolved_path)
                .await
                .with_context(|| format!("Failed to read symlink-expanded string: {}", resolved_path.display()))
        } else {
            tokio::fs::read_to_string(path.as_ref())
                .await
                .with_context(|| format!("Failed to read string from: {}", path.as_ref().display()))
        }
    }
}