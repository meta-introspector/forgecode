use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use forge_infra::Plugin;

use crate::DedupSearchService;

/// Built-in Forge plugin that exposes full-text search across the
/// dedup store's Tantivy index.
pub struct DedupSearchPlugin {
    service: Arc<DedupSearchService>,
    active: AtomicBool,
}

impl DedupSearchPlugin {
    /// Creates a new dedup search plugin with the default cache directory.
    pub fn new() -> Self {
        Self {
            service: Arc::new(DedupSearchService::new()),
            active: AtomicBool::new(false),
        }
    }

    /// Creates a dedup search plugin with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            service: Arc::new(DedupSearchService::with_cache_dir(cache_dir)),
            active: AtomicBool::new(false),
        }
    }

    /// Creates a plugin around an existing dedup search service.
    pub fn from_service(service: Arc<DedupSearchService>) -> Self {
        Self {
            service,
            active: AtomicBool::new(false),
        }
    }

    /// Returns the dedup search service owned by this plugin.
    pub fn service(&self) -> Arc<DedupSearchService> {
        self.service.clone()
    }
}

impl Default for DedupSearchPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<F> Plugin<crate::ForgeServices<F>> for DedupSearchPlugin
where
    F: forge_app::HttpInfra
        + forge_app::EnvironmentInfra<Config = forge_config::ForgeConfig>
        + forge_app::FileWriterInfra
        + forge_app::FileInfoInfra
        + forge_app::FileReaderInfra
        + forge_app::McpServerInfra
        + forge_app::WalkerInfra
        + forge_app::DirectoryReaderInfra
        + forge_app::CommandInfra
        + forge_app::UserInfra
        + forge_app::FileRemoverInfra
        + forge_app::FileDirectoryInfra
        + Clone
        + forge_app::KVStore
        + forge_app::AgentRepository
        + forge_app::StrategyFactory
        + forge_domain::FuzzySearchRepository
        + forge_domain::TextPatchRepository
        + forge_domain::SnapshotRepository
        + forge_domain::ConversationRepository
        + forge_domain::ChatRepository
        + forge_domain::ProviderRepository
        + forge_domain::WorkspaceIndexRepository
        + forge_domain::SkillRepository
        + forge_domain::ValidationRepository
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        "dedup-search"
    }

    fn description(&self) -> &str {
        "Full-text search across 36M deduplicated code chunks via Tantivy index."
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn initialize(&self, _services: Arc<crate::ForgeServices<F>>) -> anyhow::Result<()> {
        let status = self.service.index_status()?;
        if status.built {
            eprintln!(
                "[dedup-search] Tantivy index ready at {} ({:.1} MB)",
                status.location, status.size_mb
            );
        } else {
            eprintln!(
                "[dedup-search] WARNING: Tantivy index not built. Run: shmem-dedup index"
            );
        }
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.active.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
