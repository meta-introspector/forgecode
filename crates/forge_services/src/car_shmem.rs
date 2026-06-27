use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use forge_app::CarShmemAccessService;
use forge_domain::{
    CarShmemBlock, CarShmemQuery, CarShmemQueryResult, CarShmemSearchEntry, CarShmemStats,
};
use serde::{Deserialize, Serialize};
use uds::{UnixSocketAddr, UnixStreamExt};

#[derive(Debug, Clone, Deserialize)]
struct IndexEntry {
    cid: String,
    offset: u64,
    length: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct MemoryBlockEntry {
    path: String,
    cid: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    size: usize,
}

/// Service that provides Forge-level access to the IPLD CAR shared-memory store.
///
/// # Arguments
///
/// * `root` - Base directory containing `pages.car` and CAR index files.
/// * `live` - Whether to query the live `@ipld_car_shmem` server before falling
///   back to local index files.
pub struct CarShmemService {
    root: PathBuf,
    live: bool,
}

impl CarShmemService {
    /// Creates a new CAR shared-memory service backed by local index files.
    pub fn new(root: PathBuf) -> Self {
        Self { root, live: false }
    }

    /// Creates a new CAR shared-memory service with live server access enabled.
    pub fn new_with_socket(root: PathBuf, live: bool) -> Self {
        Self { root, live }
    }

    /// Returns the configured shared-memory root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether live server access is enabled.
    pub fn live(&self) -> bool {
        self.live
    }

    /// Returns a summary of the shared-memory store and page registry.
    ///
    /// # Errors
    ///
    /// Returns an error if an index file cannot be read or parsed.
    pub fn stats(&self) -> Result<CarShmemStats> {
        if self.live {
            if let Ok(stats) = self.stats_live() {
                return Ok(stats);
            }
        }

        self.stats_local()
    }

    /// Returns the requested CAR page or block metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested page or block cannot be located.
    pub fn query(&self, query: CarShmemQuery) -> Result<CarShmemQueryResult> {
        match query {
            CarShmemQuery::Stats => Ok(CarShmemQueryResult::Stats(self.stats()?)),
            CarShmemQuery::Cid(cid) => self
                .get_block(cid.clone())
                .map(CarShmemQueryResult::Block)
                .or_else(|error| {
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
                    {
                        Ok(CarShmemQueryResult::NotFound { query: cid })
                    } else {
                        Err(error)
                    }
                }),
            CarShmemQuery::Path(path) => self
                .get_block_by_path(&path)
                .map(CarShmemQueryResult::Block)
                .or_else(|error| {
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
                    {
                        Ok(CarShmemQueryResult::NotFound { query: path })
                    } else {
                        Err(error)
                    }
                }),
            CarShmemQuery::Search(query) => self.search(&query).map(CarShmemQueryResult::Search),
        }
    }

    /// Returns a block by CID when it is present in the shared-memory store.
    ///
    /// # Errors
    ///
    /// Returns an error if the block cannot be read from the CAR store.
    pub fn get_block(&self, cid: String) -> Result<CarShmemBlock> {
        if self.live {
            if let Ok(block) = self.get_block_live(&cid) {
                return Ok(block);
            }
        }

        self.get_block_local(cid)
    }

    /// Returns a memory block by path when it is present in the shared-memory store.
    ///
    /// # Errors
    ///
    /// Returns an error if the block cannot be read from the CAR store.
    pub fn get_block_by_path(&self, path: &str) -> Result<CarShmemBlock> {
        if self.live {
            if let Ok(block) = self.get_block_by_path_live(path) {
                return Ok(block);
            }
        }

        self.get_block_by_path_local(path)
    }

    /// Searches memory block metadata by path or description.
    ///
    /// # Errors
    ///
    /// Returns an error if the search cannot be completed.
    pub fn search(&self, query: &str) -> Result<Vec<CarShmemSearchEntry>> {
        if self.live {
            if let Ok(results) = self.search_live(query) {
                return Ok(results);
            }
        }

        self.search_local(query)
    }

    fn stats_local(&self) -> Result<CarShmemStats> {
        let pages_path = self.root.join("pages.car");
        let registry_path = self.root.join("registry.json");

        let block_entries = self.load_index().with_context(|| {
            format!("failed to load {}", self.root.join("index.json").display())
        })?;
        let memory_blocks = self.load_memory_index().with_context(|| {
            format!("failed to load {}", self.root.join("blocks.json").display())
        })?;
        let token_entries = self.load_token_index().with_context(|| {
            format!("failed to load {}", self.root.join("tokens.json").display())
        })?;

        let registry = if registry_path.exists() {
            let raw = std::fs::read_to_string(&registry_path)
                .with_context(|| format!("failed to read {}", registry_path.display()))?;
            serde_json::from_str::<serde_json::Value>(raw.as_str())
                .with_context(|| format!("failed to parse {}", registry_path.display()))?
        } else {
            serde_json::json!({})
        };

        let page_bytes = pages_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        Ok(CarShmemStats {
            root: self.root.clone(),
            pages_path,
            page_bytes,
            block_entries: block_entries.len(),
            memory_blocks: memory_blocks.len(),
            token_entries: token_entries.len(),
            registry,
            capacity: None,
            used: None,
            needs_compaction: None,
            live: false,
        })
    }

    fn stats_live(&self) -> Result<CarShmemStats> {
        let mut client = CarShmemClient::connect()?;
        let raw_stats = client.stats()?;
        let memory_blocks = client.list_blocks().unwrap_or_default();
        let file_size = raw_stats.get("file_size").and_then(|value| value.as_u64());
        let used = raw_stats.get("used").and_then(|value| value.as_u64());
        let capacity = raw_stats.get("capacity").and_then(|value| value.as_u64());
        let needs_compaction = raw_stats
            .get("needs_compaction")
            .and_then(|value| value.as_bool());
        let block_entries = raw_stats
            .get("block_entries")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();

        Ok(CarShmemStats {
            root: self.root.clone(),
            pages_path: self.root.join("pages.car"),
            page_bytes: file_size.unwrap_or_default(),
            block_entries: block_entries as usize,
            memory_blocks: memory_blocks.len(),
            token_entries: raw_stats
                .get("token_entries")
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as usize,
            registry: serde_json::json!({}),
            capacity,
            used,
            needs_compaction,
            live: true,
        })
    }

    fn get_block_local(&self, cid: String) -> Result<CarShmemBlock> {
        let index = self.load_index()?;
        let entry = index
            .get(&cid)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "CID not found"))?;

        self.block_from_entry(entry)
    }

    fn get_block_by_path_local(&self, path: &str) -> Result<CarShmemBlock> {
        let memory_blocks = self.load_memory_index()?;
        let entry = memory_blocks
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "path not found"))?;

        self.get_block_local(entry.cid)
    }

    fn get_block_live(&self, cid: &str) -> Result<CarShmemBlock> {
        let mut client = CarShmemClient::connect()?;
        let payload = client.fetch_car(cid)?;
        if payload == b"not_found" {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "CID not found").into());
        }

        Ok(block_from_payload(cid.to_string(), 0, payload))
    }

    fn get_block_by_path_live(&self, path: &str) -> Result<CarShmemBlock> {
        let mut client = CarShmemClient::connect()?;
        let payload = client.get_block(path)?;
        if payload == b"not_found" {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "path not found").into());
        }

        let cid = client
            .list_blocks()?
            .into_iter()
            .find(|block| block.path == path)
            .map(|block| block.cid)
            .unwrap_or_else(|| format!("path:{path}"));

        Ok(block_from_payload(cid, 0, payload))
    }

    fn search_local(&self, query: &str) -> Result<Vec<CarShmemSearchEntry>> {
        let memory_blocks = self.load_memory_index()?;
        let index = self.load_index()?;
        let query = query.to_lowercase();

        Ok(memory_blocks
            .values()
            .filter(|entry| {
                entry.path.to_lowercase().contains(&query)
                    || entry.description.to_lowercase().contains(&query)
            })
            .filter_map(|entry| {
                let block = index.get(&entry.cid)?;
                let payload = self
                    .read_payload(entry.cid.as_str(), block.offset, block.length)
                    .ok()?;
                let preview = String::from_utf8(payload)
                    .ok()
                    .map(|value| value.chars().take(240).collect());

                Some(CarShmemSearchEntry {
                    path: entry.path.clone(),
                    description: entry.description.clone(),
                    read_only: entry.read_only,
                    size: entry.size,
                    cid: entry.cid.clone(),
                    preview,
                })
            })
            .collect())
    }

    fn search_live(&self, query: &str) -> Result<Vec<CarShmemSearchEntry>> {
        let mut client = CarShmemClient::connect()?;
        client.search(query)
    }

    fn block_from_entry(&self, entry: IndexEntry) -> Result<CarShmemBlock> {
        let payload = self.read_payload(&entry.cid, entry.offset, entry.length)?;
        let preview = String::from_utf8(payload)
            .ok()
            .map(|value| value.chars().take(240).collect());

        Ok(CarShmemBlock {
            cid: entry.cid,
            offset: entry.offset,
            length: entry.length,
            preview,
        })
    }

    fn read_payload(&self, cid: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
        let mut file = File::open(self.root.join("pages.car"))
            .with_context(|| format!("failed to open {}", self.root.join("pages.car").display()))?;
        file.seek(SeekFrom::Start(offset))
            .with_context(|| format!("failed to seek to offset {offset}"))?;

        let mut payload = vec![0u8; length as usize];
        file.read_exact(&mut payload).with_context(|| {
            format!("failed to read {length} bytes for CID {cid} at offset {offset}")
        })?;
        Ok(payload)
    }

    fn load_index(&self) -> Result<HashMap<String, IndexEntry>> {
        let path = self.root.join("index.json");
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let entries: Vec<IndexEntry> = serde_json::from_str(raw.as_str())
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.cid.clone(), entry))
            .collect())
    }

    fn load_memory_index(&self) -> Result<HashMap<String, MemoryBlockEntry>> {
        let path = self.root.join("blocks.json");
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let entries: Vec<MemoryBlockEntry> = serde_json::from_str(raw.as_str())
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect())
    }

    fn load_token_index(&self) -> Result<HashMap<String, serde_json::Value>> {
        let path = self.root.join("tokens.json");
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let entries: Vec<serde_json::Value> = serde_json::from_str(raw.as_str())
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                let hash = entry.get("hash")?.as_str()?.to_string();
                Some((hash, entry))
            })
            .collect())
    }
}

impl CarShmemAccessService for CarShmemService {
    fn stats(&self) -> anyhow::Result<forge_domain::CarShmemStats> {
        self.stats()
    }

    fn query(
        &self,
        query: forge_domain::CarShmemQuery,
    ) -> anyhow::Result<forge_domain::CarShmemQueryResult> {
        self.query(query)
    }

    fn get_block(&self, cid: String) -> anyhow::Result<forge_domain::CarShmemBlock> {
        self.get_block(cid)
    }

    fn get_block_by_path(&self, path: &str) -> anyhow::Result<forge_domain::CarShmemBlock> {
        self.get_block_by_path(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShMemId([u8; 20]);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShMemDescription {
    id: ShMemId,
    size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryBlockRequest {
    path: String,
    description: String,
    read_only: bool,
    #[serde(with = "serde_bytes")]
    content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryBlockPath {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryBlockDescriptionUpdate {
    path: String,
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryBlockMetadata {
    path: String,
    description: String,
    read_only: bool,
    size: usize,
    cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AstNodeRequest {
    crate_name: String,
    source_file: String,
    node_id: u64,
    node_kind: String,
    #[serde(with = "serde_bytes")]
    content: Vec<u8>,
    span_start: u64,
    span_end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AstNodeCid {
    cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AstNodeListRequest {
    crate_name: String,
    source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AstNodePath {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CarRequest {
    #[serde(with = "serde_bytes")]
    car_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CarCid {
    cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CarSearchQuery {
    query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServedShMemRequest {
    NewMap(#[serde(with = "serde_bytes")] Vec<u8>),
    ExistingMap(ShMemDescription),
    Deregister(ShMemId),
    Hello,
    PreFork,
    PostForkChildHello(i32),
    Exit,
    ListCids,
    Stats,
    Compact,
    PutBlock(MemoryBlockRequest),
    GetBlock(MemoryBlockPath),
    ListBlocks,
    DeleteBlock(MemoryBlockPath),
    UpdateBlockDescription(MemoryBlockDescriptionUpdate),
    PutAstNode(AstNodeRequest),
    GetAstNode(AstNodeCid),
    ListAstNodes(AstNodeListRequest),
    GetAstNodeCid(AstNodePath),
    PutCar(CarRequest),
    FetchCar(CarCid),
    SearchCar(CarSearchQuery),
    RmCar(CarCid),
}

struct CarShmemClient {
    stream: UnixStream,
    id: i32,
}

impl CarShmemClient {
    fn connect() -> Result<Self> {
        let stream = UnixStreamExt::connect_to_unix_addr(&UnixSocketAddr::new("@ipld_car_shmem")?)?;
        let mut client = Self { stream, id: -1 };

        client.send(&ServedShMemRequest::Hello)?;
        let (data, _fds) = client.recv()?;
        let id_str = String::from_utf8_lossy(&data);
        client.id = id_str.trim().parse()?;

        Ok(client)
    }

    fn fetch_car(&mut self, cid: &str) -> Result<Vec<u8>> {
        self.send(&ServedShMemRequest::FetchCar(CarCid {
            cid: cid.to_string(),
        }))?;
        let (data, _fds) = self.recv()?;
        Ok(data)
    }

    fn get_block(&mut self, path: &str) -> Result<Vec<u8>> {
        self.send(&ServedShMemRequest::GetBlock(MemoryBlockPath {
            path: path.to_string(),
        }))?;
        let (data, _fds) = self.recv()?;
        Ok(data)
    }

    fn list_blocks(&mut self) -> Result<Vec<MemoryBlockMetadata>> {
        self.send(&ServedShMemRequest::ListBlocks)?;
        let (data, _fds) = self.recv()?;
        Ok(serde_json::from_slice(&data)?)
    }

    fn search(&mut self, query: &str) -> Result<Vec<CarShmemSearchEntry>> {
        self.send(&ServedShMemRequest::SearchCar(CarSearchQuery {
            query: query.to_string(),
        }))?;
        let (data, _fds) = self.recv()?;
        let metadata: Vec<MemoryBlockMetadata> = serde_json::from_slice(&data)?;
        Ok(metadata
            .into_iter()
            .map(|block| CarShmemSearchEntry {
                path: block.path,
                description: block.description,
                read_only: block.read_only,
                size: block.size,
                cid: block.cid,
                preview: None,
            })
            .collect())
    }

    fn stats(&mut self) -> Result<serde_json::Value> {
        self.send(&ServedShMemRequest::Stats)?;
        let (data, _fds) = self.recv()?;
        Ok(serde_json::from_slice(&data)?)
    }

    fn send(&mut self, request: &ServedShMemRequest) -> Result<()> {
        let body = postcard::to_allocvec(request)?;
        let header = (body.len() as u32).to_be_bytes();
        let mut message = header.to_vec();
        message.extend_from_slice(&body);
        self.stream.write_all(&message)?;
        self.stream.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<(Vec<u8>, Vec<i32>)> {
        let mut size_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut size_bytes)
            .context("failed to read message size from server")?;
        let size = u32::from_be_bytes(size_bytes) as usize;
        let mut data = vec![0u8; size];
        let mut fd_buf = [-1i32; 1];

        match self.stream.recv_fds(&mut data, &mut fd_buf) {
            Ok((n, fd_count)) => {
                if n < size {
                    anyhow::bail!(
                        "failed to read whole message from server: expected {size} bytes, got {n}"
                    );
                }
                data.truncate(n);
                Ok((data, fd_buf[..fd_count].to_vec()))
            }
            Err(_) => {
                self.stream.read_exact(&mut data).with_context(|| {
                    format!("failed to read {size} byte message body from server")
                })?;
                Ok((data, Vec::new()))
            }
        }
    }
}

fn block_from_payload(cid: String, offset: u64, payload: Vec<u8>) -> CarShmemBlock {
    let preview = String::from_utf8(payload.clone())
        .ok()
        .map(|value| value.chars().take(240).collect());

    CarShmemBlock { cid, offset, length: payload.len() as u64, preview }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use forge_domain::{CarShmemQuery, CarShmemQueryResult, CarShmemStats};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    fn write_json(path: impl AsRef<Path>, value: serde_json::Value) {
        fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    }

    fn fixture() -> (tempfile::TempDir, CarShmemService) {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("pages.car"), b"hello").unwrap();
        write_json(
            temp.path().join("index.json"),
            json!([{
                "cid": "abc",
                "offset": 0,
                "length": 5,
                "refcount": 1
            }]),
        );
        write_json(
            temp.path().join("blocks.json"),
            json!([{
                "path": "memory/test.txt",
                "description": "test block",
                "read_only": true,
                "cid": "abc",
                "size": 5
            }]),
        );
        write_json(
            temp.path().join("tokens.json"),
            json!([{
                "hash": "tok",
                "content": [104, 101],
                "refcount": 1
            }]),
        );
        write_json(temp.path().join("registry.json"), json!({"source": "test"}));

        let service = CarShmemService::new(temp.path().to_path_buf());
        (temp, service)
    }

    #[test]
    fn test_stats_loads_store_indexes() {
        // Setup
        let (_temp, service) = fixture();
        let root = service.root().to_path_buf();
        let pages_path = root.join("pages.car");

        // Execute
        let actual = service.stats().unwrap();

        // Verify
        let expected = CarShmemStats {
            root,
            pages_path,
            page_bytes: 5,
            block_entries: 1,
            memory_blocks: 1,
            token_entries: 1,
            registry: json!({"source": "test"}),
            capacity: None,
            used: None,
            needs_compaction: None,
            live: false,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_query_finds_block_by_cid_and_path() {
        // Setup
        let (_temp, service) = fixture();
        let expected_block = CarShmemBlock {
            cid: "abc".to_string(),
            offset: 0,
            length: 5,
            preview: Some("hello".to_string()),
        };

        // Execute
        let actual_cid = service
            .query(CarShmemQuery::Cid("abc".to_string()))
            .unwrap();
        let actual_path = service
            .query(CarShmemQuery::Path("memory/test.txt".to_string()))
            .unwrap();

        // Verify
        assert_eq!(
            actual_cid,
            CarShmemQueryResult::Block(expected_block.clone())
        );
        assert_eq!(actual_path, CarShmemQueryResult::Block(expected_block));
    }

    #[test]
    fn test_query_searches_memory_blocks() {
        // Setup
        let (_temp, service) = fixture();

        // Execute
        let actual = service
            .query(CarShmemQuery::Search("test".to_string()))
            .unwrap();

        // Verify
        assert_eq!(
            actual,
            CarShmemQueryResult::Search(vec![CarShmemSearchEntry {
                path: "memory/test.txt".to_string(),
                description: "test block".to_string(),
                read_only: true,
                size: 5,
                cid: "abc".to_string(),
                preview: Some("hello".to_string()),
            }])
        );
    }

    #[test]
    fn test_query_returns_not_found_for_missing_cid() {
        // Setup
        let (_temp, service) = fixture();

        // Execute
        let actual = service
            .query(CarShmemQuery::Cid("missing".to_string()))
            .unwrap();

        // Verify
        assert_eq!(
            actual,
            CarShmemQueryResult::NotFound { query: "missing".to_string() }
        );
    }
}
