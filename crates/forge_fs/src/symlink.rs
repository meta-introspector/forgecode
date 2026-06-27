use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use anyhow::Context;
use tokio::fs;
use std::io;

static SYMLINK_CACHE: Lazy<Mutex<HashMap<PathBuf, PathBuf>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// Shmem path prefix for CAR shared memory lookups
const SHMEM_PATH_PREFIX: &str = "/mnt/data1/shmem/";

/// Resolve symlink chain recursively with caching and shmem integration
pub async fn resolve_symlink_with_caching(path: &Path) -> anyhow::Result<PathBuf> {
    let mut current_path = path.to_path_buf();
    let mut depth = 0;
    let mut visited: HashSet<PathBuf> = HashSet::new();

    loop {
        // Cycle detection
        if !visited.insert(current_path.clone()) {
            return Err(anyhow::anyhow!(
                "Symlink cycle detected for path: {}",
                path.display()
            ));
        }

        // Check cache first
        if let Some(cached) = SYMLINK_CACHE.lock().unwrap().get(&current_path) {
            return Ok(cached.clone());
        }

        match fs::read_link(&current_path).await {
            Ok(target) => {
                depth += 1;
                if depth > 40 {
                    return Err(anyhow::anyhow!(
                        "Symlink chain too deep (exceeded 40) for path: {}",
                        path.display()
                    ));
                }

                // Handle shmem paths - resolve through CAR shmem service
                if target.starts_with(SHMEM_PATH_PREFIX) {
                    let shmem_path = target.strip_prefix(SHMEM_PATH_PREFIX)
                        .unwrap_or(&target);
                    // For shmem paths, we return the path as-is since the CAR service
                    // handles the actual content access
                    let resolved = PathBuf::from(SHMEM_PATH_PREFIX).join(shmem_path);
                    SYMLINK_CACHE.lock().unwrap().insert(current_path.clone(), resolved.clone());
                    return Ok(resolved);
                }

                // Resolve relative symlinks relative to the symlink's parent directory
                let target_path = if target.is_absolute() {
                    PathBuf::from(target)
                } else {
                    current_path
                        .parent()
                        .map_or_else(|| PathBuf::from(&target), |parent| {
                            PathBuf::from(parent).join(target.clone())
                        })
                };

                // Store intermediate result in cache
                SYMLINK_CACHE.lock().unwrap().insert(current_path.clone(), target_path.clone());
                current_path = target_path;
            }
            Err(e) if e.kind() == io::ErrorKind::InvalidInput => {
                // Not a symlink - cache and return
                SYMLINK_CACHE.lock().unwrap().insert(current_path.clone(), current_path.clone());
                return Ok(current_path);
            }
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read symlink: {}", path.display()));
            }
        }
    }
}

/// Legacy function for backward compatibility
pub async fn resolve_symlink(path: &Path) -> anyhow::Result<PathBuf> {
    resolve_symlink_with_caching(path).await
}

/// Checks if a path is a symbolic link
pub async fn is_symlink(path: &Path) -> anyhow::Result<bool> {
    let metadata = tokio::fs::metadata(path).await?;
    Ok(metadata.file_type().is_symlink())
}

/// Convenience function to check if a path is a symlink without following it
pub async fn is_symlink_fast(path: &Path) -> anyhow::Result<bool> {
    let metadata = tokio::fs::symlink_metadata(path).await?;
    Ok(metadata.file_type().is_symlink())
}

/// Gets all symlink targets in a chain
pub async fn resolve_symlink_chain(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut chain = Vec::new();
    let mut current_path = path.to_path_buf();
    let mut visited: HashSet<PathBuf> = HashSet::new();

    loop {
        chain.push(current_path.clone());
        if !visited.insert(current_path.clone()) {
            return Err(anyhow::anyhow!(
                "Symlink cycle detected during chain resolution")
            );
        }

        match fs::read_link(&current_path).await {
            Ok(target) => {
                if visited.contains(&target) {
                    return Err(anyhow::anyhow!(
                        "Symlink cycle detected during chain resolution")
                    );
                }
                current_path = target;
            }
            Err(_) => break,
        }
    }

    Ok(chain)
}

/// Reads a file, handling symlinks transparently
pub async fn read_transparent(path: &Path) -> anyhow::Result<Vec<u8>> {
    let resolved = resolve_symlink_with_caching(path).await?;
    tokio::fs::read(&resolved)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

/// Reads a file into a string, handling symlinks transparently
pub async fn read_transparent_string(path: &Path) -> anyhow::Result<String> {
    let resolved = resolve_symlink_with_caching(path).await?;
    tokio::fs::read_to_string(&resolved)
        .await
        .with_context(|| format!("Failed to read string from: {}", path.display()))
}

/// Creates a symlink (requires appropriate permissions)
pub async fn create_symlink(target: &Path, link_name: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(target, link_name)
            .map_err(|e| anyhow::anyhow!("Failed to create symlink: {}", e))
    }
    #[cfg(not(unix))]
    {
        Err(anyhow::anyhow!("Symlink creation not supported on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_resolve_symlink_nonexistent() {
        let path = PathBuf::from("/nonexistent/path");
        let result = resolve_symlink(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_is_symlink_false_for_regular_path() {
        let path = PathBuf::from("/etc/passwd");
        let result = is_symlink(&path).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_resolve_symlink_chain() {
        let path = PathBuf::from("/etc/passwd");
        let result = resolve_symlink_chain(&path).await;
        assert!(result.is_ok());
        let chain = result.unwrap();
        assert_eq!(chain.len(), 1);
    }
}