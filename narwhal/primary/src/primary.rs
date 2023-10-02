// Copyright (c) 2021, Facebook, Inc. and its affiliates
// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::tss::proto::{gg20_peer_server::Gg20PeerServer, Gg20AnemoService};
use crate::{
    block_synchronizer::{handler::BlockSynchronizerHandler, BlockSynchronizer},
    block_waiter::BlockWaiter,
    certificate_fetcher::CertificateFetcher,
    certifier::Certifier,
    grpc_server::ConsensusAPIGrpc,
    metrics::{initialise_metrics, PrimaryMetrics},
    proposer::{OurDigestMessage, Proposer},
    scalar_event::{ScalarEventHandler, ScalarEventService},
    state_handler::StateHandler,
    synchronizer::Synchronizer,
    tss::{TssParty, TssPeerService},
    BlockRemover,
};
use anemo::{codegen::InboundRequestLayer, types::Address};
use anemo::{types::PeerInfo, Network, PeerId};
use anemo_tower::auth::RequireAuthorizationLayer;
use anemo_tower::set_header::SetResponseHeaderLayer;
use anemo_tower::{
    auth::AllowedPeers,
    callback::CallbackLayer,
    inflight_limit, rate_limit,
    set_header::SetRequestHeaderLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, TraceLayer},
};
use async_trait::async_trait;
use config::{Authority, AuthorityIdentifier, Committee, Parameters, WorkerCache};
use consensus::consensus::{ConsensusRound, LeaderSchedule};
use consensus::dag::Dag;
use crypto::traits::EncodeDecodeBase64;
use crypto::{KeyPair, NetworkKeyPair, NetworkPublicKey, Signature};
use fastcrypto::{
    hash::Hash,
    signature_service::SignatureService,
    traits::{KeyPair as _, ToFromBytes},
};
use mysten_metrics::metered_channel::{channel_with_total, Receiver, Sender};
use mysten_metrics::{monitored_scope, spawn_monitored_task};
use mysten_network::{multiaddr::Protocol, Multiaddr};
use network::{
    client::NetworkClient,
    epoch_filter::{AllowedEpoch, EPOCH_HEADER_KEY},
};
use network::{failpoints::FailpointsMakeCallbackHandler, metrics::MetricsMakeCallbackHandler};
use parking_lot::Mutex;
use prometheus::Registry;
use std::collections::{btree_map::Entry, BTreeMap, HashMap};
use std::{
    cmp::Reverse,
    collections::{BTreeSet, BinaryHeap},
    net::Ipv4Addr,
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use storage::{
    CertificateStore, EventStore, HeaderStore, PayloadStore, ProposerStore, VoteDigestStore,
};
use sui_protocol_config::ProtocolConfig;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::{sync::watch, task::JoinHandle};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use tower::ServiceBuilder;
use tracing::{debug, error, info, instrument, warn};
use types::{
    ensure,
    error::{DagError, DagResult},
    now,
    scalar_event_server::ScalarEventServer,
    Certificate, CertificateAPI, CertificateDigest, ExternalMessage, FetchCertificatesRequest,
    FetchCertificatesResponse, GetCertificatesRequest, GetCertificatesResponse, Header, HeaderAPI,
    MetadataAPI, PayloadAvailabilityRequest, PayloadAvailabilityResponse,
    PreSubscribedBroadcastSender, PrimaryToPrimary, PrimaryToPrimaryServer, RequestVerifyRequest,
    RequestVerifyResponse, RequestVoteRequest, RequestVoteResponse, Round, SendCertificateRequest,
    SendCertificateResponse, TssPeerServer, Vote, VoteInfoAPI, WorkerOthersBatchMessage,
    WorkerOurBatchMessage, WorkerOwnBatchMessage, WorkerToPrimary, WorkerToPrimaryServer,
};
#[cfg(any(test))]
#[path = "tests/primary_tests.rs"]
pub mod primary_tests;

/// The default channel capacity for each channel of the primary.
pub const CHANNEL_CAPACITY: usize = 1_000;

/// The number of shutdown receivers to create on startup. We need one per component loop.
pub const NUM_SHUTDOWN_RECEIVERS: u64 = 27;

/// Maximum duration to fetch certificates from local storage.
const FETCH_CERTIFICATES_MAX_HANDLER_TIME: Duration = Duration::from_secs(10);

pub struct Primary;

impl Primary {
    // Spawns the primary and returns the JoinHandles of its tasks, as well as a metered receiver for the Consensus.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        authority: Authority,
        signer: KeyPair,
        network_signer: NetworkKeyPair,
        committee: Committee,
        worker_cache: WorkerCache,
        _protocol_config: ProtocolConfig,
        parameters: Parameters,
        client: NetworkClient,
        header_store: HeaderStore,
        certificate_store: CertificateStore,
        proposer_store: ProposerStore,
        payload_store: PayloadStore,
        vote_digest_store: VoteDigestStore,
        event_store: EventStore,
        tx_new_certificates: Sender<Certificate>,
        rx_committed_certificates: Receiver<(Round, Vec<Certificate>)>,
        rx_consensus_round_updates: watch::Receiver<ConsensusRound>,
        dag: Option<Arc<Dag>>,
        tx_shutdown: &mut PreSubscribedBroadcastSender,
        tx_committed_certificates: Sender<(Round, Vec<Certificate>)>,
        registry: &Registry,
        leader_schedule: LeaderSchedule,
        rx_external_message: UnboundedReceiver<ExternalMessage>,
    ) -> Vec<JoinHandle<()>> {
        // Write the parameters to the logs.
        parameters.tracing();

        // Some info statements
        let own_peer_id = PeerId(network_signer.public().0.to_bytes());
        info!(
            "Boot primary node with peer id {} and public key {}",
            own_peer_id,
            authority.protocol_key().encode_base64(),
        );

        // Initialize the metrics
        let metrics = initialise_metrics(registry);
        let endpoint_metrics = metrics.endpoint_metrics.unwrap();
        let mut primary_channel_metrics = metrics.primary_channel_metrics.unwrap();
        let inbound_network_metrics = Arc::new(metrics.inbound_network_metrics.unwrap());
        let outbound_network_metrics = Arc::new(metrics.outbound_network_metrics.unwrap());
        let node_metrics = Arc::new(metrics.node_metrics.unwrap());
        let network_connection_metrics = metrics.network_connection_metrics.unwrap();

        let (tx_our_digests, rx_our_digests) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_our_digests,
            &primary_channel_metrics.tx_our_digests_total,
        );
        let (tx_parents, rx_parents) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_parents,
            &primary_channel_metrics.tx_parents_total,
        );
        let (tx_headers, rx_headers) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_headers,
            &primary_channel_metrics.tx_headers_total,
        );
        let (tx_certificate_fetcher, rx_certificate_fetcher) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_certificate_fetcher,
            &primary_channel_metrics.tx_certificate_fetcher_total,
        );
        let (tx_block_synchronizer_commands, rx_block_synchronizer_commands) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_block_synchronizer_commands,
            &primary_channel_metrics.tx_block_synchronizer_commands_total,
        );
        let (tx_committed_own_headers, rx_committed_own_headers) = channel_with_total(
            CHANNEL_CAPACITY,
            &primary_channel_metrics.tx_committed_own_headers,
            &primary_channel_metrics.tx_committed_own_headers_total,
        );

        // we need to hack the gauge from this consensus channel into the primary registry
        // This avoids a cyclic dependency in the initialization of consensus and primary
        let committed_certificates_gauge = tx_committed_certificates.gauge().clone();
        primary_channel_metrics.replace_registered_committed_certificates_metric(
            registry,
            Box::new(committed_certificates_gauge),
        );

        let new_certificates_gauge = tx_new_certificates.gauge().clone();
        primary_channel_metrics
            .replace_registered_new_certificates_metric(registry, Box::new(new_certificates_gauge));

        let (tx_narwhal_round_updates, rx_narwhal_round_updates) = watch::channel(0u64);
        let (tx_synchronizer_network, rx_synchronizer_network) = oneshot::channel();

        let synchronizer = Arc::new(Synchronizer::new(
            authority.id(),
            committee.clone(),
            worker_cache.clone(),
            parameters.gc_depth,
            client.clone(),
            certificate_store.clone(),
            payload_store.clone(),
            tx_certificate_fetcher,
            tx_new_certificates,
            tx_parents,
            rx_consensus_round_updates.clone(),
            rx_synchronizer_network,
            dag.clone(),
            node_metrics.clone(),
            &primary_channel_metrics,
        ));

        let signature_service = SignatureService::new(signer.copy());

        // Spawn the network receiver listening to messages from the other primaries.
        let address = authority.primary_address();
        let address = address
            .replace(0, |_protocol| Some(Protocol::Ip4(Ipv4Addr::UNSPECIFIED)))
            .unwrap();
        let mut primary_service = PrimaryToPrimaryServer::new(PrimaryReceiverHandler {
            authority_id: authority.id(),
            committee: committee.clone(),
            worker_cache: worker_cache.clone(),
            synchronizer: synchronizer.clone(),
            signature_service: signature_service.clone(),
            header_store: header_store.clone(),
            certificate_store: certificate_store.clone(),
            payload_store: payload_store.clone(),
            vote_digest_store,
            event_store: event_store.clone(),
            rx_narwhal_round_updates,
            parent_digests: Default::default(),
            metrics: node_metrics.clone(),
        })
        // Allow only one inflight RequestVote RPC at a time per peer.
        // This is required for correctness.
        .add_layer_for_request_vote(InboundRequestLayer::new(
            inflight_limit::InflightLimitLayer::new(1, inflight_limit::WaitMode::ReturnError),
        ))
        // Allow only one inflight FetchCertificates RPC at a time per peer.
        // These are already a batch request; an individual peer should never need more than one.
        .add_layer_for_fetch_certificates(InboundRequestLayer::new(
            inflight_limit::InflightLimitLayer::new(1, inflight_limit::WaitMode::ReturnError),
        ));

        // Apply other rate limits from configuration as needed.
        if let Some(limit) = parameters.anemo.send_certificate_rate_limit {
            primary_service = primary_service.add_layer_for_send_certificate(
                InboundRequestLayer::new(rate_limit::RateLimitLayer::new(
                    governor::Quota::per_second(limit),
                    rate_limit::WaitMode::Block,
                )),
            );
        }
        if let Some(limit) = parameters.anemo.get_payload_availability_rate_limit {
            primary_service = primary_service.add_layer_for_get_payload_availability(
                InboundRequestLayer::new(rate_limit::RateLimitLayer::new(
                    governor::Quota::per_second(limit),
                    rate_limit::WaitMode::Block,
                )),
            );
        }
        if let Some(limit) = parameters.anemo.get_certificates_rate_limit {
            primary_service = primary_service.add_layer_for_get_certificates(
                InboundRequestLayer::new(rate_limit::RateLimitLayer::new(
                    governor::Quota::per_second(limit),
                    rate_limit::WaitMode::Block,
                )),
            );
        }

        let worker_receiver_handler = WorkerReceiverHandler {
            tx_our_digests,
            payload_store: payload_store.clone(),
        };

        client.set_worker_to_primary_local_handler(Arc::new(worker_receiver_handler.clone()));

        let worker_service = WorkerToPrimaryServer::new(worker_receiver_handler);

        let addr = address.to_anemo_address().unwrap();

        let epoch_string: String = committee.epoch().to_string();

        let our_worker_peer_ids = worker_cache
            .our_workers(authority.protocol_key())
            .unwrap()
            .into_iter()
            .map(|worker_info| PeerId(worker_info.name.0.to_bytes()));
        let worker_to_primary_router = anemo::Router::new()
            .add_rpc_service(worker_service)
            // Add an Authorization Layer to ensure that we only service requests from our workers
            .route_layer(RequireAuthorizationLayer::new(AllowedPeers::new(
                our_worker_peer_ids,
            )))
            .route_layer(RequireAuthorizationLayer::new(AllowedEpoch::new(
                epoch_string.clone(),
            )));
        // Channel for communication between Tss spawn and Anemo Tss service
        let (tx_tss_keygen, rx_tss_keygen) = mpsc::unbounded_channel();
        let (tx_tss_sign, rx_tss_sign) = mpsc::unbounded_channel();
        let (tx_message_out, rx_message_out) = mpsc::unbounded_channel();
        // Send sign init from Scalar Event handler to Tss spawn
        let (tx_tss_sign_init, rx_tss_sign_init) = mpsc::unbounded_channel();
        // Send sign result from tss spawn to Scalar handler
        let (tx_tss_sign_result, rx_tss_sign_result) = mpsc::unbounded_channel();

        // let tss_service = TssPeerServer::new(TssPeerService::new(
        //     tx_tss_keygen.clone(),
        //     tx_tss_sign.clone(),
        // ));
        let scalar_event_service = ScalarEventServer::new(ScalarEventService::new(
            committee.clone(),
            event_store.clone(),
            tx_tss_sign_init,
        ));
        let gg20_service = Gg20PeerServer::new(Gg20AnemoService::new(
            tx_tss_keygen.clone(),
            tx_tss_sign.clone(),
        ));
        let routes = anemo::Router::new()
            .add_rpc_service(primary_service)
            //.add_rpc_service(tss_service)
            .add_rpc_service(scalar_event_service)
            .add_rpc_service(gg20_service)
            .route_layer(RequireAuthorizationLayer::new(AllowedEpoch::new(
                epoch_string.clone(),
            )))
            .merge(worker_to_primary_router);

        let service = ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_server_errors()
                    .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                    .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
            )
            .layer(CallbackLayer::new(MetricsMakeCallbackHandler::new(
                inbound_network_metrics,
                parameters.anemo.excessive_message_size(),
            )))
            .layer(CallbackLayer::new(FailpointsMakeCallbackHandler::new()))
            .layer(SetResponseHeaderLayer::overriding(
                EPOCH_HEADER_KEY.parse().unwrap(),
                epoch_string.clone(),
            ))
            .service(routes);

        let outbound_layer = ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_client_and_server_errors()
                    .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                    .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
            )
            .layer(CallbackLayer::new(MetricsMakeCallbackHandler::new(
                outbound_network_metrics,
                parameters.anemo.excessive_message_size(),
            )))
            .layer(CallbackLayer::new(FailpointsMakeCallbackHandler::new()))
            .layer(SetRequestHeaderLayer::overriding(
                EPOCH_HEADER_KEY.parse().unwrap(),
                epoch_string,
            ))
            .into_inner();

        let anemo_config = {
            let mut quic_config = anemo::QuicConfig::default();
            // Allow more concurrent streams for burst activity.
            quic_config.max_concurrent_bidi_streams = Some(10_000);
            // Increase send and receive buffer sizes on the primary, since the primary also
            // needs to fetch payloads.
            // With 200MiB buffer size and ~500ms RTT, the max throughput ~400MiB/s.
            quic_config.stream_receive_window = Some(100 << 20);
            quic_config.receive_window = Some(200 << 20);
            quic_config.send_window = Some(200 << 20);
            quic_config.crypto_buffer_size = Some(1 << 20);
            quic_config.socket_receive_buffer_size = Some(20 << 20);
            quic_config.socket_send_buffer_size = Some(20 << 20);
            quic_config.allow_failed_socket_buffer_size_setting = true;
            quic_config.max_idle_timeout_ms = Some(30_000);
            // Enable keep alives every 5s
            quic_config.keep_alive_interval_ms = Some(5_000);
            let mut config = anemo::Config::default();
            config.quic = Some(quic_config);
            // Set the max_frame_size to be 2 GB to work around the issue of there being too many
            // delegation events in the epoch change txn.
            config.max_frame_size = Some(2 << 30);
            // Set a default timeout of 300s for all RPC requests
            config.inbound_request_timeout_ms = Some(300_000);
            config.outbound_request_timeout_ms = Some(300_000);
            config.shutdown_idle_timeout_ms = Some(1_000);
            config.connectivity_check_interval_ms = Some(2_000);
            config.connection_backoff_ms = Some(1_000);
            config.max_connection_backoff_ms = Some(20_000);
            config
        };

        let network;
        let mut retries_left = 90;

        loop {
            let network_result = anemo::Network::bind(addr.clone())
                .server_name("narwhal")
                .private_key(network_signer.copy().private().0.to_bytes())
                .config(anemo_config.clone())
                .outbound_request_layer(outbound_layer.clone())
                .start(service.clone());
            match network_result {
                Ok(n) => {
                    network = n;
                    break;
                }
                Err(_) => {
                    retries_left -= 1;

                    if retries_left <= 0 {
                        panic!("Failed to initialize Network!");
                    }
                    error!(
                        "Address {} should be available for the primary Narwhal service, retrying in one second",
                        addr
                    );
                    sleep(Duration::from_secs(1));
                }
            }
        }
        if tx_synchronizer_network.send(network.clone()).is_err() {
            panic!("Failed to send Network to Synchronizer!");
        }

        info!("Primary {} listening on {}", authority.id(), address);

        let mut peer_types = HashMap::new();

        // Add my workers
        for worker in worker_cache.our_workers(authority.protocol_key()).unwrap() {
            let (peer_id, address) =
                Self::add_peer_in_network(&network, worker.name, &worker.worker_address);
            peer_types.insert(peer_id, "our_worker".to_string());
            info!(
                "Adding our worker with peer id {} and address {}",
                peer_id, address
            );
        }

        // Add others workers
        for (_, worker) in worker_cache.others_workers(authority.protocol_key()) {
            let (peer_id, address) =
                Self::add_peer_in_network(&network, worker.name, &worker.worker_address);
            peer_types.insert(peer_id, "other_worker".to_string());
            info!(
                "Adding others worker with peer id {} and address {}",
                peer_id, address
            );
        }

        // Add other primaries
        let primaries = committee
            .others_primaries_by_id(authority.id())
            .into_iter()
            .map(|(_, address, network_key)| (network_key, address));

        for (public_key, address) in primaries {
            let (peer_id, address) = Self::add_peer_in_network(&network, public_key, &address);
            peer_types.insert(peer_id, "other_primary".to_string());
            info!(
                "Adding others primaries with peer id {} and address {}",
                peer_id, address
            );
        }

        let (connection_monitor_handle, _) = network::connectivity::ConnectionMonitor::spawn(
            network.downgrade(),
            network_connection_metrics,
            peer_types,
            Some(tx_shutdown.subscribe()),
        );

        info!(
            "Primary {} listening to network admin messages on 127.0.0.1:{}",
            authority.id(),
            parameters
                .network_admin_server
                .primary_network_admin_server_port
        );

        let admin_handles = network::admin::start_admin_server(
            parameters
                .network_admin_server
                .primary_network_admin_server_port,
            network.clone(),
            tx_shutdown.subscribe(),
        );

        let core_handle = Certifier::spawn(
            authority.id(),
            committee.clone(),
            header_store.clone(),
            certificate_store.clone(),
            synchronizer.clone(),
            signature_service,
            tx_shutdown.subscribe(),
            rx_headers,
            node_metrics.clone(),
            network.clone(),
        );

        let signature_service = SignatureService::new(signer.copy());
        let tss_handles = TssParty::spawn_v2(
            authority.clone(),
            committee.clone(),
            network.clone(),
            tx_tss_keygen,
            rx_tss_keygen,
            tx_tss_sign,
            rx_tss_sign,
            rx_message_out,
            rx_tss_sign_init,
            tx_tss_sign_result,
            tx_shutdown.subscribe(),
        );
        let scalar_event_handle = ScalarEventHandler::spawn(
            signer.copy(),
            authority.clone(),
            committee.clone(),
            worker_cache.clone(),
            signature_service,
            event_store,
            network.clone(),
            rx_tss_sign_result,
            rx_external_message,
            tx_shutdown.subscribe(),
        );
        // The `CertificateFetcher` waits to receive all the ancestors of a certificate before looping it back to the
        // `Synchronizer` for further processing.
        let certificate_fetcher_handle = CertificateFetcher::spawn(
            authority.id(),
            committee.clone(),
            network.clone(),
            certificate_store.clone(),
            rx_consensus_round_updates,
            tx_shutdown.subscribe(),
            rx_certificate_fetcher,
            synchronizer.clone(),
            node_metrics.clone(),
        );

        // When the `Synchronizer` collects enough parent certificates, the `Proposer` generates
        // a new header with new batch digests from our workers and sends it to the `Certifier`.
        let proposer_handle = Proposer::spawn(
            authority.id(),
            committee.clone(),
            proposer_store,
            parameters.header_num_of_batches_threshold,
            parameters.max_header_num_of_batches,
            parameters.max_header_delay,
            parameters.min_header_delay,
            None,
            tx_shutdown.subscribe(),
            rx_parents,
            rx_our_digests,
            tx_headers,
            tx_narwhal_round_updates,
            rx_committed_own_headers,
            node_metrics,
            leader_schedule,
        );

        let mut handles = vec![
            core_handle,
            certificate_fetcher_handle,
            proposer_handle,
            connection_monitor_handle,
            scalar_event_handle,
        ];
        handles.extend(tss_handles);
        handles.extend(admin_handles);

        // If a DAG component is present then we are not using the internal consensus (Bullshark)
        // but rather an external one and we are leveraging a pure DAG structure, and more components
        // need to get initialised.
        if dag.is_some() {
            let (tx_certificate_synchronizer, mut rx_certificate_synchronizer) =
                mpsc::channel(CHANNEL_CAPACITY);
            spawn_monitored_task!(async move {
                while let Some(cert) = rx_certificate_synchronizer.recv().await {
                    // Ok to ignore error including Suspended,
                    // because fetching would be kicked off.
                    let _ = synchronizer.try_accept_certificate(cert).await;
                }
                // BlockSynchronizer has shut down.
            });

            let block_synchronizer_handler = Arc::new(BlockSynchronizerHandler::new(
                tx_block_synchronizer_commands,
                tx_certificate_synchronizer,
                certificate_store.clone(),
                parameters
                    .block_synchronizer
                    .handler_certificate_deliver_timeout,
            ));

            // Responsible for finding missing blocks (certificates) and fetching
            // them from the primary peers by synchronizing also their batches.
            let block_synchronizer_handle = BlockSynchronizer::spawn(
                authority.id(),
                committee.clone(),
                worker_cache.clone(),
                tx_shutdown.subscribe(),
                rx_block_synchronizer_commands,
                network.clone(),
                payload_store.clone(),
                certificate_store.clone(),
                parameters.clone(),
            );

            // Retrieves a block's data by contacting the worker nodes that contain the
            // underlying batches and their transactions.
            // TODO: (Laura) pass shutdown signal here
            let block_waiter = BlockWaiter::new(
                authority.id(),
                committee.clone(),
                worker_cache.clone(),
                network.clone(),
                block_synchronizer_handler.clone(),
            );

            // Orchestrates the removal of blocks across the primary and worker nodes.
            // TODO: (Laura) pass shutdown signal here
            let block_remover = BlockRemover::new(
                authority.id(),
                committee.clone(),
                worker_cache,
                certificate_store,
                header_store,
                payload_store,
                dag.clone(),
                network.clone(),
                tx_committed_certificates,
            );

            // Spawn a grpc server to accept requests from external consensus layer.
            let consensus_api_handle = ConsensusAPIGrpc::spawn(
                authority.id(),
                parameters.consensus_api_grpc.socket_addr,
                block_waiter,
                block_remover,
                parameters.consensus_api_grpc.get_collections_timeout,
                parameters.consensus_api_grpc.remove_collections_timeout,
                block_synchronizer_handler,
                dag,
                committee,
                endpoint_metrics,
                tx_shutdown.subscribe(),
            );

            handles.extend(vec![block_synchronizer_handle, consensus_api_handle]);
        }

        // Keeps track of the latest consensus round and allows other tasks to clean up their their internal state
        let state_handler_handle = StateHandler::spawn(
            authority.id(),
            rx_committed_certificates,
            tx_shutdown.subscribe(),
            Some(tx_committed_own_headers),
            network,
        );
        handles.push(state_handler_handle);

        // NOTE: This log entry is used to compute performance.
        info!(
            "Primary {} successfully booted on {}",
            authority.id(),
            authority.primary_address()
        );

        handles
    }

    fn add_peer_in_network(
        network: &Network,
        peer_name: NetworkPublicKey,
        address: &Multiaddr,
    ) -> (PeerId, Address) {
        let peer_id = PeerId(peer_name.0.to_bytes());
        let address = address.to_anemo_address().unwrap();
        let peer_info = PeerInfo {
            peer_id,
            affinity: anemo::types::PeerAffinity::High,
            address: vec![address.clone()],
        };
        network.known_peers().insert(peer_info);

        (peer_id, address)
    }
}

/// Defines how the network receiver handles incoming primary messages.
#[derive(Clone)]
struct PrimaryReceiverHandler {
    /// The id of this primary.
    authority_id: AuthorityIdentifier,
    committee: Committee,
    worker_cache: WorkerCache,
    synchronizer: Arc<Synchronizer>,
    /// Service to sign headers.
    signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
    header_store: HeaderStore,
    certificate_store: CertificateStore,
    payload_store: PayloadStore,
    /// The store to persist the last voted round per authority, used to ensure idempotence.
    vote_digest_store: VoteDigestStore,
    event_store: EventStore,
    /// Get a signal when the round changes.
    rx_narwhal_round_updates: watch::Receiver<Round>,
    /// Known parent digests that are being fetched from header proposers.
    /// Values are where the digests are first known from.
    /// TODO: consider limiting maximum number of digests from one authority, allow timeout
    /// and retries from other authorities.
    parent_digests: Arc<Mutex<BTreeMap<(Round, CertificateDigest), AuthorityIdentifier>>>,
    metrics: Arc<PrimaryMetrics>,
}

#[allow(clippy::result_large_err)]
impl PrimaryReceiverHandler {
    fn find_next_round(
        &self,
        origin: AuthorityIdentifier,
        current_round: Round,
        skip_rounds: &BTreeSet<Round>,
    ) -> Result<Option<Round>, anemo::rpc::Status> {
        let mut current_round = current_round;
        while let Some(round) = self
            .certificate_store
            .next_round_number(origin, current_round)
            .map_err(|e| anemo::rpc::Status::from_error(Box::new(e)))?
        {
            if !skip_rounds.contains(&round) {
                return Ok(Some(round));
            }
            current_round = round;
        }
        Ok(None)
    }

    #[allow(clippy::mutable_key_type)]
    async fn process_request_vote(
        &self,
        request: anemo::Request<RequestVoteRequest>,
    ) -> DagResult<RequestVoteResponse> {
        let header = &request.body().header;
        let committee = self.committee.clone();
        header.validate(&committee, &self.worker_cache)?;

        let num_parents = request.body().parents.len();
        ensure!(
            num_parents <= committee.size(),
            DagError::TooManyParents(num_parents, committee.size())
        );
        self.metrics
            .certificates_in_votes
            .inc_by(num_parents as u64);

        // Vote request must come from the Header's author.
        let peer_id = request
            .peer_id()
            .ok_or_else(|| DagError::NetworkError("Unable to access remote peer ID".to_owned()))?;
        let peer_network_key = NetworkPublicKey::from_bytes(&peer_id.0).map_err(|e| {
            DagError::NetworkError(format!(
                "Unable to interpret remote peer ID {peer_id:?} as a NetworkPublicKey: {e:?}"
            ))
        })?;
        let peer_authority = committee
            .authority_by_network_key(&peer_network_key)
            .ok_or_else(|| {
                DagError::NetworkError(format!(
                    "Unable to find authority with network key {peer_network_key:?}"
                ))
            })?;
        ensure!(
            header.author() == peer_authority.id(),
            DagError::NetworkError(format!(
                "Header author {:?} must match requesting peer {peer_authority:?}",
                header.author()
            ))
        );

        debug!(
            "Processing vote request for {:?} round:{:?}",
            header,
            header.round()
        );

        // Request missing parent certificates from the header proposer, to reduce voting latency
        // when some certificates are not broadcasted to many primaries.
        // This is only a latency optimization, and not required for liveness.
        let parents = request.body().parents.clone();
        if parents.is_empty() {
            // If any parent is still unknown, ask the header proposer to include them with another
            // vote request.
            let unknown_digests = self.get_unknown_parent_digests(header).await?;
            if !unknown_digests.is_empty() {
                debug!(
                    "Received vote request for {:?} with unknown parents {:?}",
                    header, unknown_digests
                );
                return Ok(RequestVoteResponse {
                    vote: None,
                    missing: unknown_digests,
                });
            }
        } else {
            // If requester has provided parent certificates, try to accept them.
            // It is ok to not check for additional unknown digests, because certificates can
            // become available asynchronously from broadcast or certificate fetching.
            self.try_accept_unknown_parents(header, parents).await?;
        }

        // Ensure the header has all parents accepted. If some are missing, waits until they become
        // available from broadcast or certificate fetching. If no certificate becomes available
        // for a digest, this request will time out or get cancelled by the requestor eventually.
        // This check is necessary for correctness.
        let parents = self
            .synchronizer
            .notify_read_parent_certificates(header)
            .await?;

        // Check the parent certificates. Ensure the parents:
        // - form a quorum
        // - are all from the previous round
        // - are from unique authorities
        let mut parent_authorities = BTreeSet::new();
        let mut stake = 0;
        for parent in parents.iter() {
            ensure!(
                parent.round() + 1 == header.round(),
                DagError::HeaderHasInvalidParentRoundNumbers(header.digest())
            );
            ensure!(
                header.created_at() >= parent.header().created_at(),
                DagError::HeaderHasInvalidParentTimestamp(header.digest())
            );
            ensure!(
                parent_authorities.insert(parent.header().author()),
                DagError::HeaderHasDuplicateParentAuthorities(header.digest())
            );
            stake += committee.stake_by_id(parent.origin());
        }
        ensure!(
            stake >= committee.quorum_threshold(),
            DagError::HeaderRequiresQuorum(header.digest())
        );

        // Synchronize all batches referenced in the header.
        self.synchronizer
            .sync_header_batches(header, /* max_age */ 0)
            .await?;

        // Check that the time of the header is smaller than the current time. If not but the difference is
        // small, just wait. Otherwise reject with an error.
        const TOLERANCE_MS: u64 = 1_000;
        let current_time = now();
        if current_time < *header.created_at() {
            if *header.created_at() - current_time < TOLERANCE_MS {
                // for a small difference we simply wait
                tokio::time::sleep(Duration::from_millis(*header.created_at() - current_time))
                    .await;
            } else {
                // For larger differences return an error, and log it
                warn!(
                    "Rejected header {:?} due to timestamp {} newer than {current_time}",
                    header,
                    *header.created_at()
                );
                return Err(DagError::InvalidTimestamp {
                    created_time: *header.created_at(),
                    local_time: current_time,
                });
            }
        }

        // Store the header.
        self.header_store
            .write(header)
            .map_err(DagError::StoreError)?;

        // Check if we can vote for this header.
        // Send the vote when:
        // 1. when there is no existing vote for this publicKey & epoch/round
        // 2. when there is a vote for this publicKey & epoch/round, and the vote is the same
        // Taking the inverse of these two, the only time we don't want to vote is when:
        // there is a digest for the publicKey & epoch/round, and it does not match the digest
        // of the vote we create for this header.
        // Also when the header is older than one we've already voted for, it is useless to vote,
        // so we don't.
        let result = self
            .vote_digest_store
            .read(&header.author())
            .map_err(DagError::StoreError)?;

        if let Some(vote_info) = result {
            ensure!(
                header.epoch() == vote_info.epoch(),
                DagError::InvalidEpoch {
                    expected: header.epoch(),
                    received: vote_info.epoch()
                }
            );
            ensure!(
                header.round() >= vote_info.round(),
                DagError::AlreadyVotedNewerHeader(
                    header.digest(),
                    header.round(),
                    vote_info.round(),
                )
            );
            if header.round() == vote_info.round() {
                // Make sure we don't vote twice for the same authority in the same epoch/round.
                let vote = Vote::new(header, &self.authority_id, &self.signature_service).await;
                if vote.digest() != vote_info.vote_digest() {
                    warn!(
                        "Authority {} submitted different header {:?} for voting",
                        header.author(),
                        header,
                    );
                    self.metrics.votes_dropped_equivocation_protection.inc();
                    return Err(DagError::AlreadyVoted(
                        vote_info.vote_digest(),
                        header.digest(),
                        header.round(),
                    ));
                }
                debug!(
                    "Resending vote {vote:?} for {} at round {}",
                    header,
                    header.round()
                );
                return Ok(RequestVoteResponse {
                    vote: Some(vote),
                    missing: Vec::new(),
                });
            }
        }

        // Make a vote and send it to the header's creator.
        let vote = Vote::new(header, &self.authority_id, &self.signature_service).await;
        debug!(
            "Created vote {vote:?} for {} at round {}",
            header,
            header.round()
        );

        // Update the vote digest store with the vote we just sent.
        self.vote_digest_store.write(&vote)?;

        Ok(RequestVoteResponse {
            vote: Some(vote),
            missing: Vec::new(),
        })
    }

    // Tries to accept certificates if they have been requested from the header author.
    // The filtering is to avoid overload from unrequested certificates. It is ok that this
    // filter may result in a certificate never arriving via header proposals, because
    // liveness is guaranteed by certificate fetching.
    async fn try_accept_unknown_parents(
        &self,
        header: &Header,
        mut parents: Vec<Certificate>,
    ) -> DagResult<()> {
        {
            let parent_digests = self.parent_digests.lock();
            parents.retain(|cert| {
                let Some(from) = parent_digests.get(&(cert.round(), cert.digest())) else {
                    return false;
                };
                // Only process a certificate from the primary where it is first known.
                *from == header.author()
            });
        }
        for parent in parents {
            self.synchronizer.try_accept_certificate(parent).await?;
        }
        Ok(())
    }

    /// Gets parent certificate digests not known before, in storage, among suspended certificates,
    /// or being requested from other header proposers.
    async fn get_unknown_parent_digests(
        &self,
        header: &Header,
    ) -> DagResult<Vec<CertificateDigest>> {
        // Get digests not known by the synchronizer, in storage or among suspended certificates.
        let mut digests = self.synchronizer.get_unknown_parent_digests(header).await?;

        // Maximum header age is chosen to strike a balance between allowing for slightly older
        // certificates to still have a chance to be included in the DAG while not wasting
        // resources on very old vote requests. This value affects performance but not correctness
        // of the algorithm.
        const HEADER_AGE_LIMIT: Round = 3;

        // Lock to ensure consistency between limit_round and where parent_digests are gc'ed.
        let mut parent_digests = self.parent_digests.lock();

        // Check that the header is not too old.
        let narwhal_round = *self.rx_narwhal_round_updates.borrow();
        let limit_round = narwhal_round.saturating_sub(HEADER_AGE_LIMIT);
        ensure!(
            limit_round <= header.round(),
            DagError::TooOld(header.digest().into(), header.round(), narwhal_round)
        );

        // Drop old entries from parent_digests.
        while let Some(((round, _digest), _authority)) = parent_digests.first_key_value() {
            // Minimum header round is limit_round, so minimum parent round is limit_round - 1.
            if *round < limit_round.saturating_sub(1) {
                parent_digests.pop_first();
            } else {
                break;
            }
        }

        // Filter out digests that are already requested from other header proposers.
        digests.retain(
            |digest| match parent_digests.entry((header.round() - 1, *digest)) {
                Entry::Occupied(_) => false,
                Entry::Vacant(v) => {
                    v.insert(header.author());
                    true
                }
            },
        );

        Ok(digests)
    }

    #[allow(clippy::mutable_key_type)]
    async fn process_request_event_verify(
        &self,
        request: anemo::Request<RequestVerifyRequest>,
    ) -> DagResult<RequestVerifyResponse> {
        let event = &request.body().event;
        let event_digest = event.digest();
        if let Some(mut stored_event) = self.event_store.read(&event_digest).await? {
            info!("Stored event {:?}", &stored_event);
            for (authoriry, signature) in event.signatures.iter() {
                stored_event.add_signature(authoriry, signature.clone());
            }
            if let Err(e) = self.event_store.write(&stored_event).await {
                error!("Event store error {:?}", e);
            }
            let mut total_stake = 0;
            for (id, _) in stored_event.signatures.iter() {
                if let Some(authority) = self.committee.authority(id) {
                    total_stake += authority.stake();
                }
            }
            if total_stake > self.committee.quorum_threshold() {
                info!("Stake: {}/{}", total_stake, total_stake);
                //Request tss then create N&B Transaction
                //Create transaction
            }
        } else {
            info!("Stored event notfound, write peer's event into the storage");
            self.event_store.write(event).await;
        }
        // Get event from event store
        //
        info!("Received request_event_verify {:?}", event);

        Ok(RequestVerifyResponse {
            event: Some(event.clone()),
        })
    }
}

#[async_trait]
impl PrimaryToPrimary for PrimaryReceiverHandler {
    async fn send_certificate(
        &self,
        request: anemo::Request<SendCertificateRequest>,
    ) -> Result<anemo::Response<SendCertificateResponse>, anemo::rpc::Status> {
        let _scope = monitored_scope("PrimaryReceiverHandler::send_certificate");
        let certificate = request.into_body().certificate;
        match self.synchronizer.try_accept_certificate(certificate).await {
            Ok(()) => Ok(anemo::Response::new(SendCertificateResponse {
                accepted: true,
            })),
            Err(DagError::Suspended(_)) => Ok(anemo::Response::new(SendCertificateResponse {
                accepted: false,
            })),
            Err(e) => Err(anemo::rpc::Status::internal(e.to_string())),
        }
    }

    async fn request_vote(
        &self,
        request: anemo::Request<RequestVoteRequest>,
    ) -> Result<anemo::Response<RequestVoteResponse>, anemo::rpc::Status> {
        self.process_request_vote(request)
            .await
            .map(anemo::Response::new)
            .map_err(|e| {
                anemo::rpc::Status::new_with_message(
                    match e {
                        // Report unretriable errors as 400 Bad Request.
                        DagError::InvalidSignature
                        | DagError::InvalidEpoch { .. }
                        | DagError::InvalidHeaderDigest
                        | DagError::HeaderHasBadWorkerIds(_)
                        | DagError::HeaderHasInvalidParentRoundNumbers(_)
                        | DagError::HeaderHasDuplicateParentAuthorities(_)
                        | DagError::AlreadyVoted(_, _, _)
                        | DagError::AlreadyVotedNewerHeader(_, _, _)
                        | DagError::HeaderRequiresQuorum(_)
                        | DagError::TooOld(_, _, _) => {
                            anemo::types::response::StatusCode::BadRequest
                        }
                        // All other errors are retriable.
                        _ => anemo::types::response::StatusCode::Unknown,
                    },
                    format!("{e:?}"),
                )
            })
    }

    async fn get_certificates(
        &self,
        request: anemo::Request<GetCertificatesRequest>,
    ) -> Result<anemo::Response<GetCertificatesResponse>, anemo::rpc::Status> {
        let digests = request.into_body().digests;
        if digests.is_empty() {
            return Ok(anemo::Response::new(GetCertificatesResponse {
                certificates: Vec::new(),
            }));
        }

        // TODO [issue #195]: Do some accounting to prevent bad nodes from monopolizing our resources.
        let certificates = self.certificate_store.read_all(digests).map_err(|e| {
            anemo::rpc::Status::internal(format!("error while retrieving certificates: {e}"))
        })?;
        Ok(anemo::Response::new(GetCertificatesResponse {
            certificates: certificates.into_iter().flatten().collect(),
        }))
    }

    #[instrument(level = "debug", skip_all, peer = ?request.peer_id())]
    async fn fetch_certificates(
        &self,
        request: anemo::Request<FetchCertificatesRequest>,
    ) -> Result<anemo::Response<FetchCertificatesResponse>, anemo::rpc::Status> {
        let time_start = Instant::now();
        let peer = request
            .peer_id()
            .map_or_else(|| "None".to_string(), |peer_id| format!("{}", peer_id));
        let request = request.into_body();
        let mut response = FetchCertificatesResponse {
            certificates: Vec::new(),
        };
        if request.max_items == 0 {
            return Ok(anemo::Response::new(response));
        }

        // Use a min-queue for (round, authority) to keep track of the next certificate to fetch.
        //
        // Compared to fetching certificates iteratatively round by round, using a heap is simpler,
        // and avoids the pathological case of iterating through many missing rounds of a downed authority.
        let (lower_bound, skip_rounds) = request.get_bounds();
        debug!(
            "Fetching certificates after round {lower_bound} for peer {:?}, elapsed = {}ms",
            peer,
            time_start.elapsed().as_millis(),
        );

        let mut fetch_queue = BinaryHeap::new();
        const MAX_SKIP_ROUNDS: usize = 1000;
        for (origin, rounds) in &skip_rounds {
            if rounds.len() > MAX_SKIP_ROUNDS {
                warn!(
                    "Peer has sent {} rounds to skip on origin {}, indicating peer's problem with \
                    committing or keeping track of GC rounds. elapsed = {}ms",
                    rounds.len(),
                    origin,
                    time_start.elapsed().as_millis(),
                );
            }
            let next_round = self.find_next_round(*origin, lower_bound, rounds)?;
            if let Some(r) = next_round {
                fetch_queue.push(Reverse((r, origin)));
            }
        }
        debug!(
            "Initialized origins and rounds to fetch, elapsed = {}ms",
            time_start.elapsed().as_millis(),
        );

        // Iteratively pop the next smallest (Round, Authority) pair, and push to min-heap the next
        // higher round of the same authority that should not be skipped.
        // The process ends when there are no more pairs in the min-heap.
        while let Some(Reverse((round, origin))) = fetch_queue.pop() {
            // Allow the request handler to be stopped after timeout.
            tokio::task::yield_now().await;
            match self
                .certificate_store
                .read_by_index(*origin, round)
                .map_err(|e| anemo::rpc::Status::from_error(Box::new(e)))?
            {
                Some(cert) => {
                    response.certificates.push(cert);
                    let next_round =
                        self.find_next_round(*origin, round, skip_rounds.get(origin).unwrap())?;
                    if let Some(r) = next_round {
                        fetch_queue.push(Reverse((r, origin)));
                    }
                }
                None => continue,
            };
            if response.certificates.len() == request.max_items {
                debug!(
                    "Collected enough certificates (num={}, elapsed={}ms), returning.",
                    response.certificates.len(),
                    time_start.elapsed().as_millis(),
                );
                break;
            }
            if time_start.elapsed() >= FETCH_CERTIFICATES_MAX_HANDLER_TIME {
                debug!(
                    "Spent enough time reading certificates (num={}, elapsed={}ms), returning.",
                    response.certificates.len(),
                    time_start.elapsed().as_millis(),
                );
                break;
            }
            assert!(response.certificates.len() < request.max_items);
        }

        // The requestor should be able to process certificates returned in this order without
        // any missing parents.
        Ok(anemo::Response::new(response))
    }

    async fn get_payload_availability(
        &self,
        request: anemo::Request<PayloadAvailabilityRequest>,
    ) -> Result<anemo::Response<PayloadAvailabilityResponse>, anemo::rpc::Status> {
        let digests = request.into_body().certificate_digests;
        let certificates = self
            .certificate_store
            .read_all(digests.to_owned())
            .map_err(|e| {
                anemo::rpc::Status::internal(format!("error reading certificates: {e:?}"))
            })?;

        let mut result: Vec<(CertificateDigest, bool)> = Vec::new();
        for (id, certificate_option) in digests.into_iter().zip(certificates) {
            // Find batches only for certificates that exist.
            if let Some(certificate) = certificate_option {
                let payload_available = match self.payload_store.read_all(
                    certificate
                        .header()
                        .payload()
                        .iter()
                        .map(|(batch, (worker_id, _))| (*batch, *worker_id)),
                ) {
                    Ok(payload_result) => payload_result.into_iter().all(|x| x.is_some()),
                    Err(err) => {
                        // Assume that we don't have the payloads available,
                        // otherwise an error response should be sent back.
                        error!("Error while retrieving payloads: {err}");
                        false
                    }
                };
                result.push((id, payload_available));
            } else {
                // We don't have the certificate available in first place,
                // so we can't even look up the batches.
                result.push((id, false));
            }
        }

        Ok(anemo::Response::new(PayloadAvailabilityResponse {
            payload_availability: result,
        }))
    }

    async fn request_event_verify(
        &self,
        request: anemo::Request<RequestVerifyRequest>,
    ) -> Result<anemo::Response<RequestVerifyResponse>, anemo::rpc::Status> {
        self.process_request_event_verify(request)
            .await
            .map(anemo::Response::new)
            .map_err(|e| {
                anemo::rpc::Status::new_with_message(
                    match e {
                        // Report unretriable errors as 400 Bad Request.
                        DagError::InvalidSignature
                        | DagError::InvalidEpoch { .. }
                        | DagError::InvalidHeaderDigest
                        | DagError::HeaderHasBadWorkerIds(_)
                        | DagError::HeaderHasInvalidParentRoundNumbers(_)
                        | DagError::HeaderHasDuplicateParentAuthorities(_)
                        | DagError::AlreadyVoted(_, _, _)
                        | DagError::AlreadyVotedNewerHeader(_, _, _)
                        | DagError::HeaderRequiresQuorum(_)
                        | DagError::TooOld(_, _, _) => {
                            anemo::types::response::StatusCode::BadRequest
                        }
                        // All other errors are retriable.
                        _ => anemo::types::response::StatusCode::Unknown,
                    },
                    format!("{e:?}"),
                )
            })
    }
}

/// Defines how the network receiver handles incoming workers messages.
#[derive(Clone)]
struct WorkerReceiverHandler {
    tx_our_digests: Sender<OurDigestMessage>,
    payload_store: PayloadStore,
}

#[async_trait]
impl WorkerToPrimary for WorkerReceiverHandler {
    // TODO: Remove once we have upgraded to protocol version 12.
    async fn report_our_batch(
        &self,
        request: anemo::Request<WorkerOurBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();

        let (tx_ack, rx_ack) = oneshot::channel();
        let response = self
            .tx_our_digests
            .send(OurDigestMessage {
                digest: message.digest,
                worker_id: message.worker_id,
                timestamp: message.metadata.created_at,
                ack_channel: Some(tx_ack),
            })
            .await
            .map(|_| anemo::Response::new(()))
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        // If we are ok, then wait for the ack
        rx_ack
            .await
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        Ok(response)
    }

    async fn report_own_batch(
        &self,
        request: anemo::Request<WorkerOwnBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();

        let (tx_ack, rx_ack) = oneshot::channel();
        let response = self
            .tx_our_digests
            .send(OurDigestMessage {
                digest: message.digest,
                worker_id: message.worker_id,
                timestamp: *message.metadata.created_at(),
                ack_channel: Some(tx_ack),
            })
            .await
            .map(|_| anemo::Response::new(()))
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        // If we are ok, then wait for the ack
        rx_ack
            .await
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        Ok(response)
    }

    async fn report_others_batch(
        &self,
        request: anemo::Request<WorkerOthersBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();
        self.payload_store
            .write(&message.digest, &message.worker_id)
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;
        Ok(anemo::Response::new(()))
    }
}
