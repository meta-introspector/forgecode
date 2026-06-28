use serde::{Deserialize, Serialize};

/// Query accepted by the dedup full-text search plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DedupSearchQuery {
    /// Full-text search for chunks matching the query string.
    Search { query: String, limit: Option<usize> },
    /// Return dedup store statistics.
    Stats,
    /// Return Tantivy index status.
    IndexStatus,
}

/// A single chunk search result from the dedup full-text index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DedupSearchResult {
    /// Sequential chunk ID in chunks.bin.
    pub chunk_id: u32,
    /// The chunk content (line of code).
    pub content: String,
    /// Tantivy relevance score.
    pub score: f32,
}

/// Dedup store statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DedupStats {
    /// Number of unique chunks in the store.
    pub unique_chunks: u64,
    /// Total bytes stored in chunks.bin.
    pub chunks_bin_bytes: u64,
    /// Total bytes in chunk_meta.bin.
    pub chunk_meta_bytes: u64,
    /// Whether the Tantivy index has been built.
    pub index_built: bool,
    /// Tantivy index size in MB.
    pub index_size_mb: f64,
}

/// Tantivy index status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DedupIndexStatus {
    /// Whether the index directory exists.
    pub built: bool,
    /// Path to the index directory.
    pub location: String,
    /// Index size in MB.
    pub size_mb: f64,
}

/// Result returned by a dedup search query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DedupSearchQueryResult {
    /// Search results.
    Search(Vec<DedupSearchResult>),
    /// Store statistics.
    Stats(DedupStats),
    /// Index status.
    IndexStatus(DedupIndexStatus),
    /// The query did not match any chunks.
    NotFound { query: String },
}
