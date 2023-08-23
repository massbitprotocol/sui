use anemo::PeerId;
use anyhow::Ok;
use config::{Committee, Parameters, WorkerCache};
use futures::stream::FuturesUnordered;
use mysten_metrics::{RegistryID, RegistryService};
use narwhal_node::NodeError;
use sui_protocol_config::ProtocolConfig;
use tokio::task::JoinHandle;
use types::PreSubscribedBroadcastSender;

pub type NodeId = u32;
pub struct TssNodeInner {
    // The worker's id
    id: NodeId,
    protocol_config: ProtocolConfig,
    // The configuration parameters.
    parameters: Parameters,
    // A prometheus RegistryService to use for the metrics
    registry_service: RegistryService,
    // The latest registry id & registry used for the node
    registry: Option<(RegistryID, Registry)>,
    // The task handles created from primary
    handles: FuturesUnordered<JoinHandle<()>>,
    // The shutdown signal channel
    tx_shutdown: Option<PreSubscribedBroadcastSender>,
    // Peer ID used for local connections.
    own_peer_id: Option<PeerId>,
}
impl TssNodeInner {
    // Starts the worker node with the provided info. If the node is already running then this
    // method will return an error instead.
    #[instrument(level = "info", skip_all)]
    async fn start(
        &mut self,
        // The primary's id
        primary_name: PublicKey,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        // Client for communications.
        client: NetworkClient,
    ) -> Result<(), NodeError> {
        let handles = TssNodeInner::spawn();
        // now keep the handlers
        self.handles.clear();
        self.handles.extend(handles);
        Ok(())
    }
    fn spawn() -> Vec<JoinHandle<()>> {
        let mut handles = Vec::new();
        let handle = tokio::spawn(async move {});
        handles.push(handle);
        Ok(handles)
    }
}
#[derive(Clone)]
pub struct TssNode {
    internal: Arc<RwLock<TssNodeInner>>,
}

impl TssNode {
    pub fn new(
        id: NodeId,
        protocol_config: ProtocolConfig,
        parameters: Parameters,
        registry_service: RegistryService,
    ) -> TssNode {
        let inner = TssNodeInner {
            id,
            protocol_config,
            parameters,
            registry_service,
            registry: None,
            handles: FuturesUnordered::new(),
            tx_shutdown: None,
            own_peer_id: None,
        };

        Self {
            internal: Arc::new(RwLock::new(inner)),
        }
    }

    pub async fn start(
        &self,
        // The primary's public key of this authority.
        primary_key: PublicKey,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        // Client for communications.
        client: NetworkClient,
        // The node's store
        // TODO: replace this by a path so the method can open and independent storage
        store: &NodeStorage,
        // The transaction validator defining Tx acceptance,
        tx_validator: impl TransactionValidator,
        // An optional metrics struct
        metrics: Option<Metrics>,
    ) -> Result<(), NodeError> {
        let mut guard = self.internal.write().await;
        guard
            .start(
                primary_key,
                network_keypair,
                committee,
                worker_cache,
                client,
                store,
                tx_validator,
                metrics,
            )
            .await
    }

    pub async fn shutdown(&self) {
        let mut guard = self.internal.write().await;
        guard.shutdown().await
    }

    pub async fn is_running(&self) -> bool {
        let guard = self.internal.read().await;
        guard.is_running().await
    }

    pub async fn wait(&self) {
        let mut guard = self.internal.write().await;
        guard.wait().await
    }
}
