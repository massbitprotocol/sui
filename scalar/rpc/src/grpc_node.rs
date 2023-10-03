use crate::message_out::KeygenResult;
use crate::proto::abci::{ScalarAbciRequest, ScalarAbciResponse};
use crate::scalar_abci_response::Message;
use crate::scalar_abci_server::{ScalarAbci, ScalarAbciServer};

use crate::{
    gg20_client, message_in, AnemoDeliverer, Deliverer, KeygenInit, MessageIn, TssParty, NAMESPACE,
    NUM_SHUTDOWN_RECEIVERS,
};
use anemo::PeerId;
use arc_swap::Guard;
use bytes::Bytes;
use fastcrypto::bls12381::min_sig::{BLS12381KeyPair, BLS12381PublicKey};
use fastcrypto::traits::{KeyPair as _, VerifyingKey};
use futures::future::try_join_all;
use futures::stream::FuturesUnordered;
use mysten_metrics::{RegistryID, RegistryService};
use narwhal_config::{Committee, Parameters, WorkerCache};
use narwhal_crypto::{KeyPair, NetworkKeyPair, PublicKey};
use narwhal_executor::{ExecutionState, SubscriberResult};
use narwhal_network::client::NetworkClient;
use narwhal_node::NodeError;
use narwhal_types::{ExternalMessage, ScalarEventTransaction, TransactionProto};
use narwhal_types::{PreSubscribedBroadcastSender, TransactionsClient};
use narwhal_worker::LocalNarwhalClient;
use prometheus::Registry;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Instant;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use sui_protocol_config::ProtocolConfig;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{mpsc, watch, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::transport::Channel;
use tonic::{transport::Server, Request, Response, Status, Streaming};
use tracing::{debug, error, info, instrument, warn};
type ScalarAbciResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<ScalarAbciResponse, Status>> + Send>>;
pub struct GrpcNodeInner {
    // The configuration parameters.
    parameters: Parameters,
    internal_flag: bool,
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
impl GrpcNodeInner {
    async fn start<State>(
        &mut self, // The private-public key pair of this authority.
        keypair: KeyPair,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        protocol_config: ProtocolConfig,
        // The worker information cache.
        worker_cache: WorkerCache,
        // Client for communications.
        client: NetworkClient,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
        tx_external_message: mpsc::UnboundedSender<ExternalMessage>,
        rx_scalar_transaction: Arc<Mutex<UnboundedReceiver<Vec<ScalarEventTransaction>>>>,
    ) -> Result<(), NodeError>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        if self.is_running().await {
            return Err(NodeError::NodeAlreadyRunning);
        }
        self.own_peer_id = Some(PeerId(network_keypair.public().0.to_bytes()));

        // create a new registry
        let registry = Registry::new_custom(None, None).ok();

        // create the channel to send the shutdown signal
        let mut tx_shutdown = PreSubscribedBroadcastSender::new(NUM_SHUTDOWN_RECEIVERS);
        self.handles.clear();
        // spawn primary if not already running
        let handles = Self::spawn_grpc(
            keypair.copy(),
            network_keypair.copy(),
            committee.clone(),
            worker_cache.clone(),
            protocol_config.clone(),
            self.parameters.clone(),
            self.internal_flag,
            execution_state.clone(),
            &registry.as_ref().unwrap(),
            tx_external_message,
            rx_scalar_transaction,
            &mut tx_shutdown,
        )
        .await?;
        // now keep the handlers

        self.handles.extend(handles);
        // if let Some(tss_party) = self.create_tss_party(keypair, committee.clone()).await {
        //     info!("Created TssParty {}", tss_party.get_uid());
        //     let handles = Self::spawn_tss(
        //         tss_party,
        //         committee,
        //         network_keypair,
        //         worker_cache,
        //         client.clone(),
        //         protocol_config.clone(),
        //         self.parameters.clone(),
        //         self.internal_flag,
        //         execution_state,
        //         &registry.as_ref().unwrap(),
        //         &mut tx_shutdown,
        //     )
        //     .await?;
        //     // store the registry
        //     self.handles.extend(handles);
        // } else {
        //     error!("Cannot crete tss party")
        // }

        self.swap_registry(registry);
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
        info!("Sending shutdown message to grpc node");

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
        if let Err(e) = try_join_all(&mut self.handles).await {
            error!("Thread error {:?}", e);
            panic!("{:?}", e);
        };

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
    pub async fn create_tss_party(
        &mut self, // The committee information.
        // The private-public key pair of this authority.
        keypair: KeyPair,
        committee: Committee,
    ) -> Option<TssParty> {
        let name = keypair.public().clone();
        // Figure out the id for this authority
        let authority = committee
            .authority_by_key(&name)
            .unwrap_or_else(|| panic!("Our node with key {:?} should be in committee", name));
        let tss_host =
            std::env::var("TSS_HOST").unwrap_or_else(|_| Ipv4Addr::LOCALHOST.to_string());
        let tss_port = std::env::var("TSS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or_else(|| 50010 + authority.id().0);
        //+ authority.id().0;
        let tss_addr = format!("http://{}:{}", tss_host, tss_port);
        info!("TSS address {}", &tss_addr);

        // let party_uid = PeerId(authority.network_key().0.to_bytes()).to_string();
        // //let party_uid = PeerId(authority.network_key().0.to_bytes()).to_string();
        // let party_uids = committee
        //     .authorities()
        //     .map(|authority| PeerId(authority.network_key().0.to_bytes()).to_string())
        //     .collect::<Vec<String>>();
        // info!("Party uid {}", &party_uid);
        // info!("Party uids {:?}", &party_uids);
        //Start tofnd client
        gg20_client::Gg20Client::connect(tss_addr.clone())
            .await
            .map(|client| TssParty::new(client, keypair, committee))
            .ok()
    }
    /// Spawn a new primary. Optionally also spawn the consensus and a client executing transactions.
    pub async fn spawn_grpc<State>(
        // The private-public key pair of this authority.
        keypair: KeyPair,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The committee information.
        committee: Committee,
        // The worker information cache.
        worker_cache: WorkerCache,
        protocol_config: ProtocolConfig,
        // The configuration parameters.
        parameters: Parameters,
        // Whether to run consensus (and an executor client) or not.
        // If true, an internal consensus will be used, else an external consensus will be used.
        // If an external consensus will be used, then this bool will also ensure that the
        // corresponding gRPC server that is used for communication between narwhal and
        // external consensus is also spawned.
        internal_consensus: bool,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
        // A prometheus exporter Registry to use for the metrics
        registry: &Registry,
        tx_external_message: mpsc::UnboundedSender<ExternalMessage>,
        rx_scalar_transaction: Arc<Mutex<UnboundedReceiver<Vec<ScalarEventTransaction>>>>,
        // The channel to send the shutdown signal
        tx_shutdown: &mut PreSubscribedBroadcastSender,
    ) -> SubscriberResult<Vec<JoinHandle<()>>>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        let mut handles = Vec::new();
        let (tx_transaction, mut rx_transaction) = mpsc::channel(128);
        // Compute the public key of this authority.
        let name = keypair.public().clone();

        // Figure out the id for this authority
        let authority = committee
            .authority_by_key(&name)
            .unwrap_or_else(|| panic!("Our node with key {:?} should be in committee", name));
        // let address = authority.primary_address();
        // let address = address
        //     .replace(0, |_protocol| Some(Protocol::Ip4(Ipv4Addr::UNSPECIFIED)))
        //     .unwrap();
        // let aanemo_ddress = address.to_anemo_address().unwrap();

        let grpc_host =
            std::env::var("GRPC_HOST").unwrap_or_else(|_| Ipv4Addr::UNSPECIFIED.to_string());
        let grpc_port = std::env::var("GRPC_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or_else(|| 50050u16)
            + authority.id().0;
        let grpc_addr = format!("{}:{}", grpc_host, grpc_port);
        info!("GRPC address {}", &grpc_addr);
        let narwhal_client = Arc::new(AnemoClient::new(committee, worker_cache, name));
        let abci_service = GrpcService {
            tx_external_message,
            rx_scalar_transaction,
            tx_transaction,
            narwhal_client: narwhal_client.clone(),
        };
        let mut rx_shutdown = tx_shutdown.subscribe();
        let handle = tokio::spawn(async move {
            Server::builder()
                .add_service(ScalarAbciServer::new(abci_service))
                .serve_with_shutdown(
                    grpc_addr.to_socket_addrs().unwrap().next().unwrap(),
                    async {
                        let _ = rx_shutdown.receiver.recv().await;
                    },
                )
                .await
                .unwrap();
        });
        handles.push(handle);
        //Message received from relayer must be verified bofore to send to Narwhal
        // let anemo_handle = tokio::spawn(async move {
        //     let client = narwhal_client;
        //     while let Some(abci_request) = rx_transaction.recv().await {
        //         client.send_transaction(abci_request).await;
        //     }
        // });
        // handles.push(anemo_handle);
        Ok(handles)
    }
    pub async fn spawn_tss<State>(
        mut tss_party: TssParty,
        committee: Committee,
        // The private-public network key pair of this authority.
        network_keypair: NetworkKeyPair,
        // The worker information cache.
        worker_cache: WorkerCache,
        networl_client: NetworkClient,
        protocol_config: ProtocolConfig,
        // The configuration parameters.
        parameters: Parameters,
        // Whether to run consensus (and an executor client) or not.
        // If true, an internal consensus will be used, else an external consensus will be used.
        // If an external consensus will be used, then this bool will also ensure that the
        // corresponding gRPC server that is used for communication between narwhal and
        // external consensus is also spawned.
        internal_consensus: bool,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
        // A prometheus exporter Registry to use for the metrics
        registry: &Registry,
        // The channel to send the shutdown signal
        tx_shutdown: &mut PreSubscribedBroadcastSender,
    ) -> SubscriberResult<Vec<JoinHandle<()>>> {
        let mut handles = Vec::new();
        let mut rx_shutdown: narwhal_types::ConditionalBroadcastReceiver = tx_shutdown.subscribe();
        let tss_handler = tokio::spawn(async move {
            info!("Spawn thread for execute keygen");
            tokio::select! {
                _ = rx_shutdown.receiver.recv() => {
                    warn!("Node is shuting down");
                    tss_party.shutdown().await;
                    //return Err(Status::cancelled("Node is shuting down"));
                },
                res = tss_party.execute_keygen() => {
                    match res {
                        Ok(res) => {
                            info!("Execute keygen result {:?}", res);
                        }
                        Err(e) => {
                            warn!("Execute keygen error {:?}", e);
                        }
                    }
                    //return res;
                }
            }
        });
        handles.push(tss_handler);
        // info!(
        //     "Sent keygen_init {:?} for party [{}]",
        //     &keygen_init, &party_uid
        // );
        // if let Err(err) = tx_tss.send(MessageIn {
        //     data: Some(message_in::Data::KeygenInit(keygen_init)),
        // }) {
        //     println!("Send keygen init request with error {:?}", err);
        // };
        Ok(handles)
    }
}

#[derive(Debug)]
pub struct GrpcService {
    tx_external_message: mpsc::UnboundedSender<ExternalMessage>,
    rx_scalar_transaction: Arc<Mutex<UnboundedReceiver<Vec<ScalarEventTransaction>>>>,
    tx_transaction: mpsc::Sender<ScalarAbciRequest>,
    narwhal_client: Arc<AnemoClient>,
}

#[tonic::async_trait]
impl ScalarAbci for GrpcService {
    async fn unary_scalar_abci(
        &self,
        _: Request<ScalarAbciRequest>,
    ) -> ScalarAbciResult<ScalarAbciResponse> {
        Err(Status::unimplemented("not implemented"))
    }

    type ServerStreamingScalarAbciStream = ResponseStream;

    async fn server_streaming_scalar_abci(
        &self,
        req: Request<ScalarAbciRequest>,
    ) -> ScalarAbciResult<Self::ServerStreamingScalarAbciStream> {
        info!("ScalarAbciServer::server_streaming_scalar_abci");
        info!("\tclient connected from: {:?}", req.remote_addr());

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(ScalarAbciResponse { message: None });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            info!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ServerStreamingScalarAbciStream
        ))
    }

    async fn client_streaming_scalar_abci(
        &self,
        _: Request<Streaming<ScalarAbciRequest>>,
    ) -> ScalarAbciResult<ScalarAbciResponse> {
        Err(Status::unimplemented("not implemented"))
    }

    type BidirectionalStreamingScalarAbciStream = ResponseStream;

    async fn bidirectional_streaming_scalar_abci(
        &self,
        req: Request<Streaming<ScalarAbciRequest>>,
    ) -> ScalarAbciResult<Self::BidirectionalStreamingScalarAbciStream> {
        info!("ScalarAbciServer::bidirectional_streaming_scalar_abci");

        let mut in_stream = req.into_inner();
        let (tx_abci, rx_abci) = mpsc::channel(128);
        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.
        let tx_transaction = self.tx_transaction.clone();
        let tx_external_message = self.tx_external_message.clone();
        let mut handles = Vec::new();
        let rx_trans = self.rx_scalar_transaction.clone();
        let tx_abci_trans = tx_abci.clone();
        let handle = tokio::spawn(async move {
            while let Some(trans) = rx_trans.lock().await.recv().await {
                info!("Consensus made transactions {:?}", &trans);
                for event_trans in trans.into_iter() {
                    let json = serde_json::to_string(&event_trans).unwrap();
                    let message = Message::Tran(crate::ScalarOutTransaction { message: json });
                    let abci_response = ScalarAbciResponse {
                        message: Some(message),
                    };
                    let _ = tx_abci_trans.send(Ok(abci_response)).await;
                }
            }
        });
        handles.push(handle);
        let handle = tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(v) => {
                        let message = v.payload.clone();
                        info!(
                            "ScalarAbciServer::receiver message {:?}, then send to narwhal via tx_external_message",
                            &message
                        );
                        let external_message = ExternalMessage::new(message.clone().into_bytes());
                        let _ = tx_external_message.send(external_message);
                        let _ = tx_transaction.send(v.clone()).await;

                        let response_message = Message::Ark(crate::RequestArk {
                            payload: message.clone(),
                        });
                        let send_response = tx_abci
                            .send(Ok(ScalarAbciResponse {
                                message: Some(response_message),
                            }))
                            .await;
                        if let Err(_err) = send_response {
                            info!("Cannot send response to Scalar client");
                        }
                    }
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }

                        match tx_abci.send(Err(err)).await {
                            Ok(_) => (),
                            Err(_err) => break, // response was droped
                        }
                    }
                }
            }
            println!("\tstream ended");
        });
        handles.push(handle);
        // scalarAbci just write the same data that was received
        let out_stream = ReceiverStream::new(rx_abci);

        Ok(Response::new(
            Box::pin(out_stream) as Self::BidirectionalStreamingScalarAbciStream
        ))
    }
}

#[derive(Debug)]
pub struct AnemoClient {
    committee: Committee,
    worker_cache: WorkerCache,
    name: BLS12381PublicKey,
}
impl AnemoClient {
    pub fn new(committee: Committee, worker_cache: WorkerCache, name: BLS12381PublicKey) -> Self {
        Self {
            committee,
            worker_cache,
            name,
        }
    }
    fn create_remote_client(&self) -> TransactionsClient<Channel> {
        let target = self
            .worker_cache
            .worker(&self.name, /* id */ &0)
            .expect("Our key or worker id is not in the worker cache")
            .transactions;
        let config = mysten_network::config::Config::new();
        let channel = config.connect_lazy(&target).unwrap();
        //Remote client
        TransactionsClient::new(channel)
    }
    fn create_local_client(&self) -> Guard<Arc<LocalNarwhalClient>> {
        let target = self
            .worker_cache
            .worker(&self.name, /* id */ &0)
            .expect("Our key or worker id is not in the worker cache")
            .transactions;
        LocalNarwhalClient::get_global(&target).unwrap().load()
    }
    async fn send_transaction(&self, trans: ScalarAbciRequest) {
        println!("Call anemo client send_transaction {}", &trans.payload);
        //Remote client
        let mut remote_client = self.create_remote_client();
        let local_client = self.create_local_client();
        //let epoch = self.committee.epoch();
        let request = TransactionProto {
            //transaction: Bytes::from(epoch.to_be_bytes().to_vec()),
            transaction: Bytes::from(trans.payload),
        };
        // This transaciton must be ConsensusTransaction
        //let result = local_client.submit_transaction(trans.message).await;
        let result = remote_client.submit_transaction(request).await;
        if result.is_ok() {
            info!("ScalarAbciServer::AnemoClient send_transaction successfully");
        } else {
            debug!("ScalarAbciServer::AnemoClient send_transaction failed");
        }
    }
}
pub struct GrpcNode {
    internal: Arc<RwLock<GrpcNodeInner>>,
}
impl GrpcNode {
    pub fn new(
        parameters: Parameters,
        internal_flag: bool,
        registry_service: RegistryService,
    ) -> Self {
        let inner = GrpcNodeInner {
            parameters,
            internal_flag,
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
        protocol_config: ProtocolConfig,
        // The worker information cache.
        worker_cache: WorkerCache,
        // Client for communications.
        client: NetworkClient,
        // The state used by the client to execute transactions.
        execution_state: Arc<State>,
        tx_external_message: mpsc::UnboundedSender<ExternalMessage>,
        rx_scalar_transaction: Arc<Mutex<UnboundedReceiver<Vec<ScalarEventTransaction>>>>,
    ) -> Result<(), NodeError>
    where
        State: ExecutionState + Send + Sync + 'static,
    {
        let mut guard = self.internal.write().await;
        guard
            .start(
                keypair,
                network_keypair,
                committee,
                protocol_config,
                worker_cache,
                client,
                execution_state,
                tx_external_message,
                rx_scalar_transaction,
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
    pub async fn create_tss_party(
        &self,
        keypair: KeyPair,
        committee: Committee,
    ) -> Option<TssParty> {
        let mut guard = self.internal.write().await;
        guard.create_tss_party(keypair, committee).await
    }
}

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
