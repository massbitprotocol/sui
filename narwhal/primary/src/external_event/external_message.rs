use anemo::Network;
use config::{committee, AuthorityIdentifier, Committee, Epoch};
use core::time::Duration;
use crypto::{NetworkPublicKey, Signature};
use fastcrypto::{hash::Digest, signature_service::SignatureService};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use network::anemo_ext::NetworkExt;
use serde::{Deserialize, Serialize};
use storage::EventStore;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::{debug, error, info, warn};
use types::error::{DagError, DagResult};
use types::{
    ConditionalBroadcastReceiver, EventDigest, EventVerify, ExternalMessage,
    PrimaryToPrimaryClient, RequestVerifyRequest, Round,
};

#[derive(Clone)]
pub struct ExternalMessageHandler {
    pub authority_id: AuthorityIdentifier,
    pub committee: Committee,
    /// The current round of the dag.
    pub round: Round,
    pub signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
    pub event_store: EventStore,
    pub network: Network,
}

impl ExternalMessageHandler {
    pub fn run(
        self,
        mut rx_external_message: UnboundedReceiver<ExternalMessage>,
        mut rx_shutdown: ConditionalBroadcastReceiver,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = rx_shutdown.receiver.recv() => {
                        warn!("Node is shuting down");
                        break;
                    },
                    Some(msg) = rx_external_message.recv() => {
                        //Sign message and send request verification
                        info!("Receive message from external chain {:?}", &msg);
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
                        if let Ok(Some(mut stored_event)) = self.event_store.read(&digest) {
                            info!("Stored event {:?}", &stored_event);
                            if let Some(signature) = event.get_signature(&authority) {
                                stored_event.add_signature(&authority, signature.clone());
                                if let Err(e) = self.event_store.write(&stored_event) {
                                    error!("Event store error {:?}", e);
                                }
                            }
                        } else {
                            info!("Write event into node's event_store {:?}", &event);
                            self.event_store.write(&event);
                        }
                        let network = self.network.clone();
                        Self::propose_event(authority, committee, event, network).await;

                    }
                }
            }
        })
    }
    pub fn sign_message(&self, payload: [u8; 32]) -> MessageHeader {
        MessageHeader {
            epoch: self.committee.epoch(),
            payload,
        }
    }
}
impl ExternalMessageHandler {
    pub fn new(
        authority_id: AuthorityIdentifier,
        committee: Committee,
        signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
        event_store: EventStore,
        network: Network,
    ) -> Self {
        ExternalMessageHandler {
            authority_id,
            committee,
            round: 0,
            signature_service,
            event_store,
            network,
        }
    }
    pub fn spawn(
        authority_id: AuthorityIdentifier,
        committee: Committee,
        signature_service: SignatureService<Signature, { crypto::INTENT_MESSAGE_LENGTH }>,
        event_store: EventStore,
        network: Network,
        mut rx_external_message: UnboundedReceiver<ExternalMessage>,
        mut rx_shutdown: ConditionalBroadcastReceiver,
    ) -> JoinHandle<()> {
        let handler = ExternalMessageHandler::new(
            authority_id,
            committee,
            signature_service,
            event_store,
            network,
        );
        handler.run(rx_external_message, rx_shutdown)
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

        let mut client = PrimaryToPrimaryClient::new(peer);
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

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct MessageHeader {
    // Primary that created the header. Must be the same primary that broadcasted the header.
    // Validation is at: https://github.com/MystenLabs/sui/blob/f0b80d9eeef44edd9fbe606cee16717622b68651/narwhal/primary/src/primary.rs#L713-L719
    // pub author: AuthorityIdentifier,
    pub epoch: Epoch,
    pub payload: [u8; 32],
    //#[serde(skip)]
    //digests: OnceCell<HeaderDigest>,
}

pub struct MessageCertificate {}
