use std::path::PathBuf;

use anyhow::{Context, Result};
use forge_domain::{
    DedupIndexStatus, DedupSearchQuery, DedupSearchQueryResult, DedupSearchResult, DedupStats,
};

/// Default dedup cache directory.
const DEFAULT_CACHE_DIR: &str = "/mnt/data1/dasl-cache";

/// Metadata entry size: 8 bytes hash + 4 bytes length.
const META_ENTRY_SIZE: u64 = 12;

/// Service that provides full-text search across the dedup store via the
/// Tantivy index.
///
/// This service reads the pre-built Tantivy index from the dedup cache
/// directory. The index must be built with `shmem-dedup index` before use.
pub struct DedupSearchService {
    cache_dir: PathBuf,
}

impl DedupSearchService {
    /// Creates a new dedup search service pointing at the default cache directory.
    pub fn new() -> Self {
        Self {
            cache_dir: PathBuf::from(DEFAULT_CACHE_DIR),
        }
    }

    /// Creates a dedup search service with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Returns the configured cache directory.
    pub fn cache_dir(&self) -> &std::path::Path {
        &self.cache_dir
    }

    /// Execute a dedup search query.
    pub fn query(&self, query: DedupSearchQuery) -> Result<DedupSearchQueryResult> {
        match query {
            DedupSearchQuery::Search { query, limit } => {
                let results = self.search(&query, limit.unwrap_or(10))?;
                if results.is_empty() {
                    Ok(DedupSearchQueryResult::NotFound { query })
                } else {
                    Ok(DedupSearchQueryResult::Search(results))
                }
            }
            DedupSearchQuery::Stats => {
                let stats = self.stats()?;
                Ok(DedupSearchQueryResult::Stats(stats))
            }
            DedupSearchQuery::IndexStatus => {
                let status = self.index_status()?;
                Ok(DedupSearchQueryResult::IndexStatus(status))
            }
        }
    }

    /// Full-text search across all deduplicated chunks.
    ///
    /// Uses the Tantivy index for sub-millisecond search.
    /// The index must be built first with `shmem-dedup index`.
    fn search(&self, query: &str, limit: usize) -> Result<Vec<DedupSearchResult>> {
        let index_dir = self.cache_dir.join("tantivy-dedup");
        if !index_dir.exists() {
            anyhow::bail!(
                "Tantivy index not found at {}. Run: shmem-dedup index",
                index_dir.display()
            );
        }

        // Open the Tantivy index directly (minimal, no path dependency)
        use tantivy::collector::TopDocs;
        use tantivy::query::QueryParser;
        use tantivy::schema::Value;
        use tantivy::schema::{STORED, FAST, STRING, TEXT};
        use tantivy::Index;

        let mut schema_builder = tantivy::schema::Schema::builder();
        let chunk_id_field = schema_builder.add_i64_field("chunk_id", STORED | FAST);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let _hash_field = schema_builder.add_text_field("hash", STRING | STORED);
        let _length_field = schema_builder.add_i64_field("length", STORED | FAST);
        let _schema = schema_builder.build();

        let index = Index::open_in_dir(&index_dir)
            .with_context(|| format!("Failed to open Tantivy index at {}", index_dir.display()))?;
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&index, vec![content_field]);

        // Try Tantivy query syntax, fall back to phrase query for raw text.
        let parsed_query = match query_parser.parse_query(query) {
            Ok(q) => q,
            Err(_) => {
                let phrase = format!("\"{}\"", query);
                query_parser.parse_query(&phrase)?
            }
        };

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let chunk_id = doc
                .get_first(chunk_id_field)
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u32;
            let content = doc
                .get_first(content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            results.push(DedupSearchResult {
                chunk_id,
                content,
                score,
            });
        }

        Ok(results)
    }

    /// Get dedup store statistics.
    fn stats(&self) -> Result<DedupStats> {
        let chunk_meta_path = self.cache_dir.join("chunk_meta.bin");
        let chunks_bin_path = self.cache_dir.join("chunks.bin");
        let index_dir = self.cache_dir.join("tantivy-dedup");

        let chunk_meta_bytes = file_size(&chunk_meta_path);
        let chunks_bin_bytes = file_size(&chunks_bin_path);
        let unique_chunks = if chunk_meta_bytes > 0 {
            chunk_meta_bytes / META_ENTRY_SIZE
        } else {
            0
        };
        let index_built = index_dir.exists();
        let index_size_mb = if index_built {
            dir_size_mb(&index_dir)
        } else {
            0.0
        };

        Ok(DedupStats {
            unique_chunks,
            chunks_bin_bytes,
            chunk_meta_bytes,
            index_built,
            index_size_mb,
        })
    }

    /// Get Tantivy index status.
    pub fn index_status(&self) -> Result<DedupIndexStatus> {
        let index_dir = self.cache_dir.join("tantivy-dedup");
        let built = index_dir.exists();
        let size_mb = if built {
            dir_size_mb(&index_dir)
        } else {
            0.0
        };

        Ok(DedupIndexStatus {
            built,
            location: index_dir.to_string_lossy().to_string(),
            size_mb,
        })
    }
}

impl Default for DedupSearchService {
    fn default() -> Self {
        Self::new()
    }
}

/// Get file size, or 0 if the file doesn't exist.
fn file_size(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Calculate directory size in MB.
fn dir_size_mb(path: &std::path::Path) -> f64 {
    dir_size_bytes(path) as f64 / 1_048_576.0
}

/// Recursive directory size in bytes.
fn dir_size_bytes(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += dir_size_bytes(&entry.path());
                }
            }
        }
    }
    total
}
