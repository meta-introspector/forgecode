use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Query accepted by the CAR shared-memory access plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CarShmemQuery {
    /// Return store statistics and index counts.
    Stats,
    /// Look up a stored CAR block by CID.
    Cid(String),
    /// Look up a memory block by its relative path.
    Path(String),
    /// Search memory block metadata by path or description.
    Search(String),
}

/// Metadata for a memory block returned by a CAR shared-memory search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarShmemSearchEntry {
    /// Relative path of the memory block.
    pub path: String,
    /// Human-readable description of the memory block.
    pub description: String,
    /// Whether the memory block is read-only.
    pub read_only: bool,
    /// Content size in bytes.
    pub size: usize,
    /// Hex-encoded CID for the block.
    pub cid: String,
    /// Optional UTF-8 preview of the block payload.
    pub preview: Option<String>,
}

/// Metadata for a block stored in the CAR shared-memory file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarShmemBlock {
    /// Hex-encoded CID for the block.
    pub cid: String,
    /// Byte offset of the block payload inside `pages.car`.
    pub offset: u64,
    /// Payload length in bytes.
    pub length: u64,
    /// Optional UTF-8 preview of the payload.
    pub preview: Option<String>,
}

/// Summary information about the CAR shared-memory store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CarShmemStats {
    /// Root directory used by the plugin.
    pub root: PathBuf,
    /// Path to the CAR backing file.
    pub pages_path: PathBuf,
    /// Size of the CAR backing file in bytes.
    pub page_bytes: u64,
    /// Number of CID index entries that were loaded.
    pub block_entries: usize,
    /// Number of memory block metadata entries that were loaded.
    pub memory_blocks: usize,
    /// Number of token index entries that were loaded.
    pub token_entries: usize,
    /// Optional JSON registry contents when present.
    pub registry: serde_json::Value,
    /// Total capacity of the live CAR store when reported by the server.
    pub capacity: Option<u64>,
    /// Bytes currently used by the live CAR store when reported by the server.
    pub used: Option<u64>,
    /// Whether the live CAR store needs compaction.
    pub needs_compaction: Option<bool>,
    /// Whether the result came from the live shared-memory server.
    pub live: bool,
}

/// Result returned by a CAR shared-memory query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CarShmemQueryResult {
    /// Store statistics.
    Stats(CarShmemStats),
    /// Block metadata.
    Block(CarShmemBlock),
    /// Search results from memory block metadata.
    Search(Vec<CarShmemSearchEntry>),
    /// The query did not match any stored block.
    NotFound { query: String },
}
