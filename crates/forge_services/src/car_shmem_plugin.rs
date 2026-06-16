use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use forge_infra::Plugin;

use crate::CarShmemService;

/// Built-in Forge plugin that exposes IPLD CAR shared-memory access.
pub struct CarShmemPlugin {
    service: Arc<CarShmemService>,
    active: AtomicBool,
}

impl CarShmemPlugin {
    /// Creates a new CAR shared-memory plugin.
    pub fn new(root: PathBuf) -> Self {
        Self {
            service: Arc::new(CarShmemService::new(root)),
            active: AtomicBool::new(false),
        }
    }

    /// Creates a plugin around an existing shared-memory service.
    pub fn from_service(service: Arc<CarShmemService>) -> Self {
        Self { service, active: AtomicBool::new(false) }
    }

    /// Returns the shared-memory service owned by this plugin.
    pub fn service(&self) -> Arc<CarShmemService> {
        self.service.clone()
    }
}

#[async_trait::async_trait]
impl<F> Plugin<crate::ForgeServices<F>> for CarShmemPlugin
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
        "car-shmem"
    }

    fn description(&self) -> &str {
        "Access IPLD CAR blocks from the live @ipld_car_shmem server and local cache fallback."
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn initialize(&self, _services: Arc<crate::ForgeServices<F>>) -> anyhow::Result<()> {
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
