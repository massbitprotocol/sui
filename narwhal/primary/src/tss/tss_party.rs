use std::{net::Ipv4Addr, sync::Arc};

use anemo::{Network, PeerId};
use anyhow::anyhow;
use config::{Authority, Committee};
use crypto::NetworkPublicKey;
use futures::future::join_all;
use k256::ecdsa::hazmat::VerifyPrimitive;
use k256::elliptic_curve::sec1::FromEncodedPoint;
use k256::elliptic_curve::ScalarPrimitive;
use k256::EncodedPoint;
use k256::ProjectivePoint;
use network::CancelOnDropHandler;
use network::RetryConfig;
use storage::TssStore;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Channel;
use tonic::Status;
use tracing::{info, warn};
use types::message_out::keygen_result::KeygenResultData;
use types::message_out::sign_result::SignResultData;
use types::message_out::SignResult;
use types::MessageOut;
use types::TssAnemoDeliveryMessage;
use types::TssAnemoKeygenRequest;
use types::TssAnemoSignRequest;
use types::TssPeerClient;
use types::{
    gg20_client, message_in,
    message_out::{self, KeygenResult},
    KeygenOutput, MessageIn, TrafficIn,
};
use types::{gg20_client::Gg20Client, KeygenInit};
use types::{ConditionalBroadcastReceiver, SignInit};
use uuid::Uuid;

#[derive(Clone)]
pub struct TssParty {
    authority: Authority,
    committee: Committee,
    network: Network,
    tx_keygen: UnboundedSender<MessageIn>,
    tx_sign: UnboundedSender<MessageIn>,
    tofnd_client: Option<Arc<RwLock<Gg20Client<Channel>>>>,
}
impl TssParty {
    pub fn new(
        authority: Authority,
        committee: Committee,
        network: Network,
        tx_keygen: UnboundedSender<MessageIn>,
        tx_sign: UnboundedSender<MessageIn>,
    ) -> Self {
        Self {
            authority,
            committee,
            network,
            tx_keygen,
            tx_sign,
            tofnd_client: None,
        }
    }
    pub async fn create_tofnd_client(port: u16) -> Option<Gg20Client<Channel>> {
        let tss_host =
            std::env::var("TSS_HOST").unwrap_or_else(|_| Ipv4Addr::LOCALHOST.to_string());
        let tss_port = std::env::var("TSS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or_else(|| port);
        //+ authority.id().0;
        let tss_addr = format!("http://{}:{}", tss_host, tss_port);
        info!("TSS address {}", &tss_addr);

        let tofnd_client = Gg20Client::connect(tss_addr.clone()).await.ok();
        tofnd_client
    }
    pub fn get_uid(&self) -> String {
        PeerId(self.authority.network_key().0.to_bytes()).to_string()
    }
    pub fn get_parties(&self) -> Vec<String> {
        let party_uids = self
            .committee
            .authorities()
            .map(|authority| PeerId(authority.network_key().0.to_bytes()).to_string())
            .collect::<Vec<String>>();
        party_uids
    }
    pub fn create_keygen_init(&self) -> KeygenInit {
        KeygenInit {
            new_key_uid: format!("tss_session{}", self.committee.epoch()),
            party_uids: self.get_parties(),
            party_share_counts: vec![1, 1, 1, 1],
            my_party_index: self.authority.id().0 as u32,
            threshold: 2,
        }
    }
    //Tss sign only - 32 bytes hash digests
    fn create_sign_init(&self, message: Vec<u8>) -> SignInit {
        SignInit {
            new_sig_uid: uuid::Uuid::new_v4().to_string(),
            key_uid: format!("tss_session{}", self.committee.epoch()),
            party_uids: self.get_parties(),
            message_to_sign: message,
        }
    }

    // async fn create_tofnd_client(&mut self) -> Option<Arc<RwLock<Gg20Client<Channel>>>> {
    //     if self.tofnd_client.is_none() {
    //         let tss_host =
    //             std::env::var("TSS_HOST").unwrap_or_else(|_| Ipv4Addr::LOCALHOST.to_string());
    //         let tss_port = std::env::var("TSS_PORT")
    //             .ok()
    //             .and_then(|p| p.parse::<u16>().ok())
    //             .unwrap_or_else(|| 50010 + self.authority.id().0);
    //         //+ authority.id().0;
    //         let tss_addr = format!("http://{}:{}", tss_host, tss_port);
    //         info!("TSS address {}", &tss_addr);

    //         self.tofnd_client = Gg20Client::connect(tss_addr.clone())
    //             .await
    //             .map(|client| Arc::new(RwLock::new(client)))
    //             .ok();
    //     }
    //     self.tofnd_client.clone()
    // }
    pub async fn execute_keygen(
        &self,
        keygen_init: KeygenInit,
        rx_keygen: UnboundedReceiver<MessageIn>,
    ) -> Result<KeygenResult, tonic::Status> {
        let my_uid = self.get_uid();
        let port = 50010 + self.authority.id().0;
        match Self::create_tofnd_client(port).await {
            None => Err(Status::not_found("tofnd client not found")),
            Some(mut client) => {
                let mut keygen_server_outgoing = client
                    .keygen(tonic::Request::new(UnboundedReceiverStream::new(rx_keygen)))
                    .await
                    .unwrap()
                    .into_inner();
                #[allow(unused_variables)]
                let all_share_count = {
                    if keygen_init.party_share_counts.is_empty() {
                        keygen_init.party_uids.len()
                    } else {
                        keygen_init.party_share_counts.iter().sum::<u32>() as usize
                    }
                };
                #[allow(unused_variables)]
                let my_share_count = {
                    if keygen_init.party_share_counts.is_empty() {
                        1
                    } else {
                        keygen_init.party_share_counts[keygen_init.my_party_index as usize] as usize
                    }
                };
                // the first outbound message is keygen init info
                self.tx_keygen
                    .send(MessageIn {
                        data: Some(message_in::Data::KeygenInit(keygen_init)),
                    })
                    .unwrap();
                #[allow(unused_variables)]
                let mut msg_count = 1;
                let result = loop {
                    match keygen_server_outgoing.message().await {
                        Ok(Some(msg)) => {
                            let msg_type = msg.data.as_ref().expect("missing data");
                            match msg_type {
                                #[allow(unused_variables)] // allow unsused traffin in non malicious
                                message_out::Data::Traffic(traffic) => {
                                    // in malicous case, if we are stallers we skip the message
                                    #[cfg(feature = "malicious")]
                                    {
                                        let round = keygen_round(
                                            msg_count,
                                            all_share_count,
                                            my_share_count,
                                        );
                                        if self.malicious_data.timeout_round == round {
                                            warn!(
                                                "{} is stalling a message in round {}",
                                                my_uid, round
                                            );
                                            continue; // tough is the life of the staller
                                        }
                                        if self.malicious_data.disrupt_round == round {
                                            warn!(
                                                "{} is disrupting a message in round {}",
                                                my_uid, round
                                            );
                                            let mut t = traffic.clone();
                                            t.payload = traffic.payload
                                                [0..traffic.payload.len() / 2]
                                                .to_vec();
                                            let mut m = msg.clone();
                                            m.data = Some(proto::message_out::Data::Traffic(t));
                                            self.deliver_keygen(&m, &my_uid).await;
                                        }
                                    }
                                    self.deliver_keygen(&msg, &my_uid).await;
                                }
                                message_out::Data::KeygenResult(res) => {
                                    info!("party [{}] keygen finished!", my_uid);
                                    break Ok(res.clone());
                                }
                                _ => {
                                    panic!(
                                        "party [{}] keygen error: bad outgoing message type",
                                        my_uid
                                    )
                                }
                            };
                            msg_count += 1;
                        }
                        Ok(None) => {
                            warn!(
                                "party [{}] keygen execution was not completed due to abort",
                                my_uid
                            );
                            return Ok(KeygenResult::default());
                        }

                        Err(status) => {
                            warn!(
                            "party [{}] keygen execution was not completed due to connection error: {}",
                            my_uid, status
                        );
                            return Err(status);
                        }
                    }
                };
                info!("party [{}] keygen execution complete", my_uid);
                return result;
            }
        }
    }
    pub async fn execute_sign(
        &self,
        sign_init: SignInit,
        rx_sign: UnboundedReceiver<MessageIn>,
    ) -> Result<SignResult, tonic::Status> {
        let my_uid = self.get_uid();
        info!(
            "Execute sign flow for message {:?}",
            &sign_init.message_to_sign //String::from_utf8(sign_init.message_to_sign.to_vec())
        );
        let port = 50010 + self.authority.id().0;
        match Self::create_tofnd_client(port).await {
            None => Err(Status::not_found("tofnd client not found")),
            Some(mut client) => {
                info!("Call tss gRPC server for sign flow");
                let mut sign_server_outgoing = client
                    .sign(tonic::Request::new(UnboundedReceiverStream::new(rx_sign)))
                    .await
                    .unwrap()
                    .into_inner();
                info!("End Call tss gRPC server for sign flow");
                #[allow(unused_variables)]
                let all_share_count = sign_init.party_uids.len();
                #[allow(unused_variables)]
                let mut msg_count = 1;
                // the first outbound message is keygen init info
                info!("Send tss sign request to gRPC server");
                self.tx_sign
                    .send(MessageIn {
                        data: Some(message_in::Data::SignInit(sign_init)),
                    })
                    .unwrap();
                let result = loop {
                    match sign_server_outgoing.message().await {
                        Ok(Some(msg)) => {
                            let msg_type = msg.data.as_ref().expect("missing data");
                            match msg_type {
                                #[allow(unused_variables)] // allow unsused traffin in non malicious
                                message_out::Data::Traffic(traffic) => {
                                    // in malicous case, if we are stallers we skip the message
                                    #[cfg(feature = "malicious")]
                                    {
                                        let round =
                                            sign_round(msg_count, all_share_count, my_share_count);
                                        if self.malicious_data.timeout_round == round {
                                            warn!(
                                                "{} is stalling a message in round {}",
                                                my_uid,
                                                round - 4
                                            ); // subtract keygen rounds
                                            continue; // tough is the life of the staller
                                        }
                                        if self.malicious_data.disrupt_round == round {
                                            warn!(
                                                "{} is disrupting a message in round {}",
                                                my_uid, round
                                            );
                                            let mut t = traffic.clone();
                                            t.payload = traffic.payload
                                                [0..traffic.payload.len() / 2]
                                                .to_vec();
                                            let mut m = msg.clone();
                                            m.data = Some(proto::message_out::Data::Traffic(t));
                                            self.deliver_sign(&m, my_uid);
                                        }
                                    }
                                    self.deliver_sign(&msg, &my_uid).await;
                                }
                                message_out::Data::SignResult(res) => {
                                    info!("party [{}] sign finished!", my_uid);
                                    break Ok(res.clone());
                                }
                                message_out::Data::NeedRecover(_) => {
                                    info!("party [{}] needs recover", my_uid);
                                    // when recovery is needed, sign is canceled. We abort the protocol manualy instead of waiting parties to time out
                                    // no worries that we don't wait for enough time, we will not be checking criminals in this case
                                    // delivery.send_timeouts(0);
                                    break Ok(SignResult::default());
                                }
                                _ => {
                                    panic!(
                                        "party [{}] sign error: bad outgoing message type",
                                        my_uid
                                    )
                                }
                            };
                            msg_count += 1;
                        }
                        Ok(None) => {
                            warn!(
                                "party [{}] sign execution was not completed due to abort",
                                my_uid
                            );
                            return Ok(SignResult::default());
                        }

                        Err(status) => {
                            warn!(
                            "party [{}] keygen execution was not completed due to connection error: {}",
                            my_uid, status
                        );
                            return Err(status);
                        }
                    }
                };
                info!("party [{}] sign execution complete", my_uid);
                return result;
            }
        }
    }

    pub async fn deliver_keygen(&self, msg: &MessageOut, from: &str) {
        let msg = msg.data.as_ref().expect("missing data");
        let msg = match msg {
            message_out::Data::Traffic(t) => t,
            _ => {
                panic!("msg must be traffic out");
            }
        };
        let msg_in = MessageIn {
            data: Some(message_in::Data::Traffic(TrafficIn {
                from_party_uid: from.to_string(),
                is_broadcast: msg.is_broadcast,
                payload: msg.payload.clone(),
            })),
        };
        //Send to own tofnd gGpc Server
        let _ = self.tx_keygen.send(msg_in);
        //info!("Broadcast message {:?} from {:?}", msg, from);
        let mut handlers = Vec::new();
        let peers = self
            .committee
            .authorities()
            .filter(|auth| auth.id().0 != self.authority.id().0)
            .map(|auth| auth.network_key().clone())
            .collect::<Vec<NetworkPublicKey>>();
        let tss_message = TssAnemoDeliveryMessage {
            from_party_uid: from.to_string(),
            is_broadcast: msg.is_broadcast,
            payload: msg.payload.clone(),
        };
        //Send to other peers vis anemo network
        for peer in peers {
            let network = self.network.clone();
            let message = tss_message.clone();
            // info!(
            //     "Deliver keygen message from {:?} to peer {:?}",
            //     from,
            //     peer.to_string()
            // );
            let f = move |peer| {
                let request = TssAnemoKeygenRequest {
                    message: message.to_owned(),
                };
                async move {
                    let result = TssPeerClient::new(peer).keygen(request).await;
                    match result.as_ref() {
                        Ok(r) => {
                            info!("TssPeerClient keygen result {:?}", r);
                        }
                        Err(e) => {
                            info!("TssPeerClient keygen error {:?}", e);
                        }
                    }
                    result
                }
            };

            let handle = send(network, peer, f);
            handlers.push(handle);
        }
        let _results = join_all(handlers).await;
        //info!("All keygen result {:?}", results);
        //handlers
    }

    pub async fn deliver_sign(&self, msg: &MessageOut, from: &str) {
        let msg = msg.data.as_ref().expect("missing data");
        let msg = match msg {
            message_out::Data::Traffic(t) => t,
            _ => {
                panic!("msg must be traffic out");
            }
        };
        let msg_in = MessageIn {
            data: Some(message_in::Data::Traffic(TrafficIn {
                from_party_uid: from.to_string(),
                is_broadcast: msg.is_broadcast,
                payload: msg.payload.clone(),
            })),
        };
        //Send to own tofnd gGpc Server
        let _ = self.tx_sign.send(msg_in);
        //info!("Broadcast message {:?} from {:?}", msg, from);
        let mut handlers = Vec::new();
        let peers = self
            .committee
            .authorities()
            .filter(|auth| auth.id().0 != self.authority.id().0)
            .map(|auth| auth.network_key().clone())
            .collect::<Vec<NetworkPublicKey>>();
        let tss_message = TssAnemoDeliveryMessage {
            from_party_uid: from.to_string(),
            is_broadcast: msg.is_broadcast,
            payload: msg.payload.clone(),
        };
        //Send to other peers vis anemo network
        for peer in peers {
            let network = self.network.clone();
            let message = tss_message.clone();
            info!(
                "Deliver sign message from {:?} to peer {:?}",
                from,
                peer.to_string()
            );
            let f = move |peer| {
                let request = TssAnemoSignRequest {
                    message: message.to_owned(),
                };
                async move {
                    let result = TssPeerClient::new(peer).sign(request).await;
                    match result.as_ref() {
                        Ok(r) => {
                            info!("TssPeerClient sign result {:?}", r);
                        }
                        Err(e) => {
                            info!("TssPeerClient sign error {:?}", e);
                        }
                    }
                    result
                }
            };

            let handle = send(network, peer, f);
            handlers.push(handle);
        }
        let results = join_all(handlers).await;
        //info!("All sign result {:?}", results);
        //handlers
    }

    pub async fn set_key(&mut self, key_data: KeygenResultData) {
        info!("Keygen result {:?}", &key_data);
        match key_data {
            KeygenResultData::Data(data) => {
                //self.tss_store.write().await.set_key(data);
            }
            KeygenResultData::Criminals(c) => {
                warn!("Crimials {:?}", c);
            }
        }
    }
    pub async fn verify_sign_result(&mut self, message_digest: Vec<u8>, sign_data: SignResultData) {
        info!("Sign result data {:?}", &sign_data);
        match sign_data {
            SignResultData::Signature(sig) => {
                info!("Vefifying signature {:?}", sig.as_slice());
                // let pub_key = self.tss_store.read().await.get_key();
                // match pub_key {
                //     Some(key) => {
                //         info!("pub key {:?}", &key.pub_key);
                //         let verify_result = self.verify(
                //             key.pub_key.as_slice(),
                //             message_digest.as_slice(),
                //             sig.as_slice(),
                //         );
                //         info!("Verify result {:?}", verify_result);
                //     }
                //     None => warn!("Missing pubkey"),
                // }
            }
            SignResultData::Criminals(c) => {
                warn!("Crimials {:?}", c);
            }
        }
    }
    fn verify(&self, pub_key: &[u8], message: &[u8], signature: &[u8]) -> anyhow::Result<bool> {
        let signature = k256::ecdsa::Signature::from_der(signature)
            .map_err(|_| anyhow!("Invalid signature"))?;
        let scalar = ScalarPrimitive::from_slice(message)?;
        let hashed_msg = k256::Scalar::from(scalar);
        let prj_point =
            ProjectivePoint::from_encoded_point(&EncodedPoint::from_bytes(pub_key)?).unwrap();
        let res = prj_point
            .to_affine()
            .verify_prehashed(message.into(), &signature)
            .is_ok();
        Ok(res)
    }
}
impl TssParty {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn spawn(
        &mut self,
        rx_keygen: UnboundedReceiver<MessageIn>,
        rx_sign: UnboundedReceiver<MessageIn>,
        mut rx_shutdown: ConditionalBroadcastReceiver,
    ) -> JoinHandle<()> {
        let keygen_init = self.create_keygen_init();
        // let mut message = uuid::Uuid::new_v4().as_bytes().to_vec();
        // message.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        let message = [42; 32].to_vec();
        let mut party = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = rx_shutdown.receiver.recv() => {
                    warn!("Node is shuting down");
                },
                res = party.execute_keygen(keygen_init.clone(), rx_keygen) => {
                    match res {
                        Ok(KeygenResult { keygen_result_data }) => {
                            if let Some(keygen_data) = keygen_result_data {
                                party.set_key(keygen_data).await;
                                let sign_init = party.create_sign_init(message.clone());
                                info!("Starting sign flow for message {:?}", &sign_init);

                                match party.execute_sign(sign_init, rx_sign).await {
                                    Ok(SignResult { sign_result_data }) => {
                                        info!("Sign result {:?}", &sign_result_data);
                                        if let Some(data) = sign_result_data {
                                            party.verify_sign_result(message, data).await;
                                        }
                                    },
                                    Err(e) => {
                                        info!("Sign error {:?}", e);
                                    },
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Execute keygen error {:?}", e);
                        }
                    }
                }
            }
        })
    }
}

fn send<F, R, Fut>(
    network: anemo::Network,
    peer: NetworkPublicKey,
    f: F,
) -> CancelOnDropHandler<anyhow::Result<anemo::Response<R>>>
where
    F: Fn(anemo::Peer) -> Fut + Send + Sync + 'static + Clone,
    R: Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = Result<anemo::Response<R>, anemo::rpc::Status>> + Send,
{
    // Safety
    // Since this spawns an unbounded task, this should be called in a time-restricted fashion.

    let peer_id = PeerId(peer.0.to_bytes());
    let message_send = move || {
        let network = network.clone();
        let f = f.clone();

        async move {
            if let Some(peer) = network.peer(peer_id) {
                f(peer).await.map_err(|e| {
                    // this returns a backoff::Error::Transient
                    // so that if anemo::Status is returned, we retry
                    backoff::Error::transient(anyhow::anyhow!("RPC error: {e:?}"))
                })
            } else {
                Err(backoff::Error::transient(anyhow::anyhow!(
                    "not connected to peer {peer_id}"
                )))
            }
        }
    };

    let retry_config = RetryConfig {
        retrying_max_elapsed_time: None, // retry forever
        ..Default::default()
    };
    let task = tokio::spawn(retry_config.retry(message_send));

    CancelOnDropHandler(task)
}
