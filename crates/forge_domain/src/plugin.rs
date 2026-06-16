use serde::{Deserialize, Serialize};

/// Lightweight metadata for a loaded Forge plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin name.
    pub name: String,
    /// Human-readable plugin description.
    pub description: String,
    /// Plugin version.
    pub version: String,
    /// Whether the plugin is currently initialized and active.
    pub active: bool,
}
