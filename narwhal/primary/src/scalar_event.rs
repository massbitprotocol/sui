use crate::tss::TssParty;
use anemo::{Network, PeerId};
use config::Authority;
use config::{committee, AuthorityIdentifier, Committee, Epoch};
use core::time::Duration;
use crypto::{NetworkPublicKey, Signature};
use fastcrypto::{hash::Digest, signature_service::SignatureService};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use network::anemo_ext::NetworkExt;
use serde::{Deserialize, Serialize};
use storage::EventStore;
use tokio::sync::mpsc::UnboundedSender;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::{debug, error, info, warn};
use types::error::{DagError, DagResult};
use types::scalar_event_client::ScalarEventClient;
use types::SignInit;
use types::{
    message_out::KeygenResult,
    scalar_event_server::{ScalarEvent, ScalarEventServer},
    ConditionalBroadcastReceiver, CrossChainTransaction, EventDigest, EventVerify, ExternalMessage,
    MessageIn, PrimaryToPrimaryClient, RequestVerifyRequest, RequestVerifyResponse, Round,
};

#[derive(Clone)]
pub struct ScalarEventHandler {
    pub authority_id: AuthorityIdentifier,
    pub committee: Committee,
    /// The current round of the dag.
    pub round: Round,
    pub signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
    pub event_store: EventStore,
    pub network: Network,
    pub tss_party: TssParty,
}

impl ScalarEventHandler {
    pub fn run(
        mut self,
        rx_tss_keygen: UnboundedReceiver<MessageIn>,
        rx_tss_sign: UnboundedReceiver<MessageIn>,
        mut rx_tss_sign_init: UnboundedReceiver<SignInit>,
        mut rx_external_message: UnboundedReceiver<ExternalMessage>,
        mut rx_shutdown: ConditionalBroadcastReceiver,
    ) -> JoinHandle<()> {
        let keygen_init = self.tss_party.create_keygen_init();
        tokio::spawn(async move {
            info!(
                "Spawn external event for node with authority {:?}",
                &self.authority_id
            );
            let port = 50010 + self.authority_id.0;
            let tofnd_client = TssParty::create_tofnd_client(port).await.unwrap();
            tokio::select! {
                _ = rx_shutdown.receiver.recv() => {
                    warn!("Node is shuting down");
                },
                res = self.tss_party.execute_keygen(keygen_init.clone(), rx_tss_keygen) => {
                    match res {
                        Ok(KeygenResult { keygen_result_data }) => {
                            if let Some(keygen_data) = keygen_result_data {
                                // party.set_key(keygen_data).await;
                                // let sign_init = party.create_sign_init(message.clone());
                                info!("Tss keygen result {:?}", &keygen_data);
                                // match party.execute_sign(sign_init, rx_sign).await {
                                //     Ok(SignResult { sign_result_data }) => {
                                //         info!("Sign result {:?}", &sign_result_data);
                                //         if let Some(data) = sign_result_data {
                                //             party.verify_sign_result(message, data).await;
                                //         }
                                //     },
                                //     Err(e) => {
                                //         info!("Sign error {:?}", e);
                                //     },
                                // }
                            }
                        }
                        Err(e) => {
                            warn!("Execute keygen error {:?}", e);
                        }
                    }
                }
            }
            loop {
                tokio::select! {
                    _ = rx_shutdown.receiver.recv() => {
                        warn!("Node is shuting down");
                        break;
                    },
                    Some(msg) = rx_external_message.recv() => {
                        //Sign message and send request verification
                        info!("{:?} Receive message from external chain {:?}", &self.authority_id, &msg);
                        let digest = EventDigest (msg.block.hash.unwrap().0.clone());
                        let round = self.round.clone();
                        let committee = self.committee.clone();
                        let authority = self.authority_id.clone();
                        let signature_service = self.signature_service.clone();
                        let event = EventVerify::new(
                            authority.clone(),
                            round,
                            committee.epoch(),
                            digest.clone(),
                            signature_service
                        )
                        .await;
                        if let Ok(Some(mut stored_event)) = self.event_store.read(&digest).await {
                            info!("Stored event {:?}", &stored_event);
                            if let Some(signature) = event.get_signature(&authority) {
                                stored_event.add_signature(&authority, signature.clone());
                                if let Err(e) = self.event_store.write(&stored_event).await {
                                    error!("Event store error {:?}", e);
                                }
                            }
                        } else {
                            info!("Write event into node's event_store {:?}", &event);
                            self.event_store.write(&event);
                        }
                        let network = self.network.clone();
                        Self::propose_event(authority, committee, event, network).await;

                    },
                    Some(sign_init) = rx_tss_sign_init.recv() => {
                        info!("{:?} Receive SignInit from ScalarEventService {:?}", &self.authority_id, &sign_init);
                        // match self.tss_party.execute_sign(sign_init, rx_sign).await {
                        //     Ok(SignResult { sign_result_data }) => {
                        //         info!("Sign result {:?}", &sign_result_data);
                        //         if let Some(data) = sign_result_data {
                        //             party.verify_sign_result(message, data).await;
                        //         }
                        //     },
                        //     Err(e) => {
                        //         info!("Sign error {:?}", e);
                        //     },
                        // }
                    }
                }
            }
        })
    }
    // pub fn sign_message(&self, payload: [u8; 32]) -> MessageHeader {
    //     MessageHeader {
    //         epoch: self.committee.epoch(),
    //         payload,
    //     }
    // }
}
impl ScalarEventHandler {
    pub fn new(
        authority_id: AuthorityIdentifier,
        committee: Committee,
        signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
        event_store: EventStore,
        network: Network,
        tss_party: TssParty,
    ) -> Self {
        ScalarEventHandler {
            authority_id,
            committee,
            round: 0,
            signature_service,
            event_store,
            network,
            tss_party,
        }
    }
    pub fn spawn(
        authority: Authority,
        committee: Committee,
        signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
        event_store: EventStore,
        network: Network,
        tx_tss_keygen: UnboundedSender<MessageIn>,
        tx_tss_sign: UnboundedSender<MessageIn>,
        rx_tss_keygen: UnboundedReceiver<MessageIn>,
        rx_tss_sign: UnboundedReceiver<MessageIn>,
        rx_tss_sign_init: UnboundedReceiver<SignInit>,
        rx_external_message: UnboundedReceiver<ExternalMessage>,
        rx_shutdown: ConditionalBroadcastReceiver,
    ) -> JoinHandle<()> {
        let tss_party = TssParty::new(
            authority.clone(),
            committee.clone(),
            network.clone(),
            tx_tss_keygen,
            tx_tss_sign,
        );
        let mut handler = ScalarEventHandler::new(
            authority.id(),
            committee,
            signature_service,
            event_store,
            network,
            tss_party,
        );
        handler.run(
            rx_tss_keygen,
            rx_tss_sign,
            rx_tss_sign_init,
            rx_external_message,
            rx_shutdown,
        )
    }
    pub async fn propose_event(
        authority_id: AuthorityIdentifier,
        committee: Committee,
        event: EventVerify,
        network: anemo::Network,
    ) -> DagResult<()> {
        info!("propose_event {:?}", &event);
        let peers = committee
            .others_primaries_by_id(authority_id)
            .into_iter()
            .map(|(name, _, network_key)| (name, network_key));
        let mut requests: FuturesUnordered<_> = peers
            .map(|(name, target)| {
                let event = event.clone();
                Self::request_verify(network.clone(), committee.clone(), name, target, event)
            })
            .collect();
        loop {
            // if certificate.is_some() {
            //     break;
            // }
            tokio::select! {
                result = &mut requests.next() => {
                    match result {
                        Some(Ok(event)) => {
                            // certificate = votes_aggregator.append(
                            //     vote,
                            //     &committee,
                            //     &header,
                            // )?;
                            info!("event_verify result {:?}", &event);
                        },
                        Some(Err(e)) => debug!("failed to get vote for header {event:?}: {e:?}"),
                        None => break,
                    }
                },
                // _ = &mut cancel => {
                //     debug!("canceling Header proposal {header} for round {}", header.round());
                //     return Err(DagError::Canceled)
                // },
            }
        }
        Ok(())
    }
    async fn request_verify(
        network: anemo::Network,
        committee: Committee,
        authority: AuthorityIdentifier,
        target: NetworkPublicKey,
        event: EventVerify,
    ) -> DagResult<()> {
        let peer_id = anemo::PeerId(target.0.to_bytes());
        let peer = network.waiting_peer(peer_id);

        //let mut client = PrimaryToPrimaryClient::new(peer);
        let mut client = ScalarEventClient::new(peer);
        let request = anemo::Request::new(RequestVerifyRequest {
            event: event.clone(),
        })
        .with_timeout(Duration::from_secs(30));
        info!("Send a request_event_verify via anemo network {:?}", &event);
        match client.request_event_verify(request).await {
            Ok(response) => {
                let response = response.into_body();
                if response.event.is_some() {
                    //break response.vote.unwrap();
                }
                //missing_parents = response.missing;
            }
            Err(status) => {
                if status.status() == anemo::types::response::StatusCode::BadRequest {
                    return Err(DagError::NetworkError(format!(
                        "unrecoverable error requesting vote for {event:?}: {status:?}"
                    )));
                }
                //missing_parents = Vec::new();
            }
        }
        Ok(())
    }
}

pub struct ScalarEventService {
    committee: Committee,
    event_store: EventStore,
    tx_tss_sign: UnboundedSender<SignInit>,
}
impl ScalarEventService {
    pub fn new(
        committee: Committee,
        event_store: EventStore,
        tx_tss_sign: UnboundedSender<SignInit>,
    ) -> Self {
        Self {
            committee,
            event_store,
            tx_tss_sign,
        }
    }
    fn create_sign_init(&self, event: &EventVerify) -> SignInit {
        let party_uids = self
            .committee
            .authorities()
            .map(|authority| PeerId(authority.network_key().0.to_bytes()).to_string())
            .collect::<Vec<String>>();
        let key_uid = format!("tss_session{}", self.committee.epoch());
        let message = event.digest().0.to_vec();
        info!(
            "Message to sign has length {}: {:?}",
            message.len(),
            &message
        );
        let sig_uid = String::from_utf8(message.clone()).unwrap();
        //Hash digest into Vec<u8> of 32 length
        SignInit {
            new_sig_uid: sig_uid,
            key_uid,
            party_uids,
            message_to_sign: message,
        }
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
            if (total_stake > self.committee.quorum_threshold()) {
                info!("Stake: {}/{}", total_stake, total_stake);
                //Request tss then create N&B Transaction
                let sign_message = self.create_sign_init(&stored_event);
                //Send message to tss signer
                self.tx_tss_sign.send(sign_message);
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
#[anemo::async_trait]
impl ScalarEvent for ScalarEventService {
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

    async fn create_cross_chain_transaction(
        &self,
        request: anemo::Request<CrossChainTransaction>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();
        // if let Err(err) = self
        //     .validator
        //     .validate_batch(&message.batch, &self.protocol_config)
        //     .await
        // {
        //     println!("report_batch {:?}", &message.batch);
        //     return Err(anemo::rpc::Status::new_with_message(
        //         StatusCode::BadRequest,
        //         format!("Invalid batch: {err}"),
        //     ));
        // }
        // let digest = message.batch.digest();

        // let mut batch = message.batch.clone();

        // // TODO: Remove once we have upgraded to protocol version 12.
        // if self.protocol_config.narwhal_versioned_metadata() {
        //     // Set received_at timestamp for remote batch.
        //     batch.versioned_metadata_mut().set_received_at(now());
        // }
        // self.store.insert(&digest, &batch).map_err(|e| {
        //     anemo::rpc::Status::internal(format!("failed to write to batch store: {e:?}"))
        // })?;
        // self.client
        //     .report_others_batch(WorkerOthersBatchMessage {
        //         digest,
        //         worker_id: self.id,
        //     })
        //     .await
        //     .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;
        Ok(anemo::Response::new(()))
    }
}
// #[derive(Clone, Default, Deserialize, Serialize)]
// pub struct MessageHeader {
//     // Primary that created the header. Must be the same primary that broadcasted the header.
//     // Validation is at: https://github.com/MystenLabs/sui/blob/f0b80d9eeef44edd9fbe606cee16717622b68651/narwhal/primary/src/primary.rs#L713-L719
//     // pub author: AuthorityIdentifier,
//     pub epoch: Epoch,
//     pub payload: [u8; 32],
//     //#[serde(skip)]
//     //digests: OnceCell<HeaderDigest>,
// }

// pub struct MessageCertificate {}
