use crate::Relayer;
use crate::RelayerConfigs;
use crate::{NAMESPACE, NUM_SHUTDOWN_RECEIVERS};
use anemo::PeerId;
use anyhow::anyhow;
use ethers::prelude::*;
use eyre;
use fastcrypto::traits::KeyPair as _;
use futures::future::try_join_all;
use futures::stream::FuturesUnordered;
use mysten_metrics::{RegistryID, RegistryService};
use narwhal_config::{Committee, Parameters, WorkerCache};
use narwhal_crypto::{KeyPair, NetworkKeyPair, PublicKey};
use narwhal_executor::{ExecutionState, SubscriberResult};
use narwhal_node::NodeError;
use narwhal_types::TransactionProto;
use narwhal_types::{PreSubscribedBroadcastSender, TransactionsClient};
use narwhal_worker::LocalNarwhalClient;
use prometheus::Registry;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use sui_protocol_config::ProtocolConfig;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tonic::Status;
use tracing::field::AsField;
use tracing::{info, instrument};
//use web3;
//const WSS_URL: &str = "wss://eth-mainnet.g.alchemy.com/v2/9u1mZJtSKl2NgzRA9i0rh5_QIPQi4pTU";
//const WSS_URL: &str = "wss://mainnet.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";
pub struct EvmRelayerInner {
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

impl EvmRelayerInner {
    async fn start<State>(
        &mut self, // The private-public key pair of this authority.
        keypair: KeyPair,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
    ) -> Result<(), anyhow::Error>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        if self.is_running().await {
            return Err(anyhow!("Relayer already running"));
        }
        self.own_peer_id = Some(PeerId(network_keypair.public().0.to_bytes()));

        // create a new registry
        let registry = Registry::new_custom(None, None).ok();

        // create the channel to send the shutdown signal
        let mut tx_shutdown = PreSubscribedBroadcastSender::new(NUM_SHUTDOWN_RECEIVERS);

        // spawn primary if not already running
        let handles = Self::spawn(
            keypair,
            network_keypair,
            committee,
            worker_cache,
            self.parameters.clone(),
            execution_state,
            &registry.as_ref().unwrap(),
            &mut tx_shutdown,
        )
        .await?;
        // store the registry
        self.swap_registry(registry);
        // now keep the handlers
        self.handles.clear();
        self.handles.extend(handles);
        self.tx_shutdown = Some(tx_shutdown);

        Ok(())
    }
    // Will shutdown the primary node and wait until the node has shutdown by waiting on the
    // underlying components handles. If the node was not already running then the
    // method will return immediately.
    #[instrument(level = "info", skip_all)]
    async fn shutdown(&mut self) {
        if !self.is_running().await {
            return;
        }

        // send the shutdown signal to the node
        let now = Instant::now();
        info!("Sending shutdown message to relayer node");

        // if let Some(c) = self.client.take() {
        //     c.shutdown();
        // }

        if let Some(tx_shutdown) = self.tx_shutdown.as_ref() {
            tx_shutdown
                .send()
                .expect("Couldn't send the shutdown signal to downstream components");
            self.tx_shutdown = None
        }

        // Now wait until handles have been completed
        try_join_all(&mut self.handles).await.unwrap();

        self.swap_registry(None);

        info!(
            "Narwhal Grpc Node shutdown is complete - took {} seconds",
            now.elapsed().as_secs_f64()
        );
    }
    // Helper method useful to wait on the execution of the primary node
    async fn wait(&mut self) {
        try_join_all(&mut self.handles).await.unwrap();
    }
    // If any of the underlying handles haven't still finished, then this method will return
    // true, otherwise false will returned instead.
    async fn is_running(&self) -> bool {
        self.handles.iter().any(|h| !h.is_finished())
    }
    // Accepts an Option registry. If it's Some, then the new registry will be added in the
    // registry service and the registry_id will be updated. Also, any previous registry will
    // be removed. If None is passed, then the registry_id is updated to None and any old
    // registry is removed from the RegistryService.
    fn swap_registry(&mut self, registry: Option<Registry>) {
        if let Some((registry_id, _registry)) = self.registry.as_ref() {
            self.registry_service.remove(*registry_id);
        }

        if let Some(registry) = registry {
            self.registry = Some((self.registry_service.add(registry.clone()), registry));
        } else {
            self.registry = None
        }
    }
    pub async fn spawn<State>(
        // The private-public key pair of this authority.
        keypair: KeyPair,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        // The configuration parameters.
        parameters: Parameters,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
        // A prometheus exporter Registry to use for the metrics
        registry: &Registry,
        // The channel to send the shutdown signal
        tx_shutdown: &mut PreSubscribedBroadcastSender,
    ) -> Result<Vec<JoinHandle<()>>, anyhow::Error>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        let mut handles = Vec::new();
        // Compute the public key of this authority.
        let name = keypair.public().clone();
        // Figure out the id for this authority
        let authority = committee
            .authority_by_key(&name)
            .unwrap_or_else(|| panic!("Our node with key {:?} should be in committee", name));
        // let mut rx_shutdown = tx_shutdown.subscribe();

        let relayer_config_dir =
            std::env::var("CONFIG_DIR").unwrap_or_else(|_| String::from("/opt/sui/config"));
        let config_path = format!(
            "{}/evm_relayer{}.toml",
            relayer_config_dir,
            authority.id().0
        );
        let config_str = fs::read_to_string(config_path.as_str())
            .map_err(|e| {
                let msg = format!("{:?}", e);
                println!("{}", msg.as_str());
                anyhow!(msg)
            })
            .expect(format!("Failed to read relayer config file {}", config_path).as_str());
        let relayer_configs: RelayerConfigs = toml::from_str(config_str.as_str()).unwrap();
        //info!("Evm relayer config {}", config_str.as_str());
        let mut ws_handles = relayer_configs
            .scalar_relayer_evm
            .into_iter()
            .map(|config| {
                let mut rx_shutdown = tx_shutdown.subscribe();
                tokio::spawn(async move {
                    // A Ws provider can be created from a ws(s) URI.
                    // In case of wss you must add the "rustls" or "openssl" feature
                    // to the ethers library dependency in `Cargo.toml`
                    if let (Some(ws_url), Some(start)) = (config.ws_addr, config.start_with_bridge)
                    {
                        if start {
                            let provider = Provider::<Ws>::connect(ws_url.as_str()).await.expect(
                                format!("Cannot connect to websocket url {:?}", ws_url.as_str())
                                    .as_str(),
                            );

                            let mut stream =
                                provider.subscribe_blocks().await.expect("Cannot subscribe");
                            loop {
                                if let Ok(_) = rx_shutdown.receiver.recv().await {
                                    info!("Received shutdown event. Quick current loop");
                                    break;
                                }
                                if let Some(block) = stream.next().await {
                                    //Received event from source chain
                                    //Broadcast a poll
                                    //Send it to the worker for create poll
                                    info!("Received evm block {:?}", &block.hash);
                                } else {
                                    info!("Received Nothing");
                                }
                            }
                        }
                    }
                })
            });
        handles.extend(&mut ws_handles);
        Ok(handles)
    }
}
pub struct EvmRelayer {
    internal: Arc<RwLock<EvmRelayerInner>>,
}

impl EvmRelayer {
    pub fn new(parameters: Parameters, registry_service: RegistryService) -> Self {
        let inner = EvmRelayerInner {
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
    #[instrument(level = "info", skip_all)]
    pub async fn start<State>(
        &self, // The private-public key pair of this authority.
        keypair: KeyPair,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
    ) -> Result<(), anyhow::Error>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        let mut guard = self.internal.write().await;
        guard
            .start(
                keypair,
                network_keypair,
                committee,
                worker_cache,
                execution_state,
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

    pub async fn registry(&self) -> Option<(RegistryID, Registry)> {
        let guard = self.internal.read().await;
        guard.registry.clone()
    }
}

impl Relayer for EvmRelayer {}

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}
