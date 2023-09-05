use crate::{
    gg20_client, key_presence_response, message_in,
    message_out::{self, KeygenResult, SignResult},
    AnemoDeliverer, KeyPresenceRequest, KeygenInit, MessageIn, MessageOut, SignInit, TrafficIn,
};
use anemo::PeerId;
use fastcrypto::traits::{KeyPair as _, VerifyingKey};
use narwhal_config::Committee;
use narwhal_crypto::{KeyPair, NetworkKeyPair, PublicKey};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::Request;
use tonic::Status;
use tracing::{error, info, warn};
type GrpcKeygenResult = Result<KeygenResult, Status>;
type GrpcSignResult = Result<SignResult, Status>;

pub type Deliverer = AnemoDeliverer;
pub struct TssParty {
    pub client: Arc<RwLock<gg20_client::Gg20Client<tonic::transport::Channel>>>,
    pub keypair: KeyPair,
    pub committee: Committee,
}
impl TssParty {
    pub fn new(
        client: gg20_client::Gg20Client<tonic::transport::Channel>,
        // Current node's uid
        keypair: KeyPair,
        // All party uids
        committee: Committee,
    ) -> Self {
        // let name = keypair.public().clone();
        // // Figure out the id for this authority
        // let authority = committee
        //     .authority_by_key(&name)
        //     .unwrap_or_else(|| panic!("Our node with key {:?} should be in committee", name));
        // let party_uid = PeerId(authority.network_key().0.to_bytes()).to_string();
        Self {
            client: Arc::new(RwLock::new(client)),
            keypair,
            committee,
        }
    }
    pub fn get_client(&self) -> Arc<RwLock<gg20_client::Gg20Client<tonic::transport::Channel>>> {
        self.client.clone()
    }
    pub fn get_index(&self) -> u32 {
        let name = self.keypair.public().clone();
        // Figure out the id for this authority
        let authority = self
            .committee
            .authority_by_key(&name)
            .unwrap_or_else(|| panic!("Our node with key {:?} should be in committee", name));
        0
    }
    pub fn get_parties(&self) -> Vec<String> {
        let party_uids = self
            .committee
            .authorities()
            .map(|authority| PeerId(authority.network_key().0.to_bytes()).to_string())
            .collect::<Vec<String>>();
        party_uids
    }
    pub fn get_current_epoch(&self) {}
    pub async fn execute_keygen(
        &mut self,
        init: KeygenInit,
        channels: SenderReceiver,
        delivery: Deliverer,
        notify: std::sync::Arc<tokio::sync::Notify>,
    ) -> GrpcKeygenResult {
        let my_uid = init.party_uids[usize::try_from(init.my_party_index).unwrap()].clone();
        let (keygen_server_incoming, rx) = channels;
        let mut keygen_server_outgoing = self
            .client
            .write()
            .await
            .keygen(Request::new(UnboundedReceiverStream::new(rx)))
            .await
            .unwrap()
            .into_inner();

        #[allow(unused_variables)]
        let all_share_count = {
            if init.party_share_counts.is_empty() {
                init.party_uids.len()
            } else {
                init.party_share_counts.iter().sum::<u32>() as usize
            }
        };
        #[allow(unused_variables)]
        let my_share_count = {
            if init.party_share_counts.is_empty() {
                1
            } else {
                init.party_share_counts[init.my_party_index as usize] as usize
            }
        };
        // the first outbound message is keygen init info
        keygen_server_incoming
            .send(MessageIn {
                data: Some(message_in::Data::KeygenInit(init)),
            })
            .unwrap();

        // block until all parties send their KeygenInit
        notify.notified().await;
        notify.notify_one();

        #[allow(unused_variables)]
        let mut msg_count = 1;

        let result = loop {
            let msg = match keygen_server_outgoing.message().await {
                Ok(msg) => match msg {
                    Some(msg) => msg,
                    None => {
                        warn!(
                            "party [{}] keygen execution was not completed due to abort",
                            my_uid
                        );
                        return Ok(KeygenResult::default());
                    }
                },
                Err(status) => {
                    warn!(
                        "party [{}] keygen execution was not completed due to connection error: {}",
                        my_uid, status
                    );
                    return Err(status);
                }
            };

            let msg_type = msg.data.as_ref().expect("missing data");

            match msg_type {
                #[allow(unused_variables)] // allow unsused traffin in non malicious
                message_out::Data::Traffic(traffic) => {
                    // in malicous case, if we are stallers we skip the message
                    #[cfg(feature = "malicious")]
                    {
                        let round = keygen_round(msg_count, all_share_count, my_share_count);
                        if self.malicious_data.timeout_round == round {
                            warn!("{} is stalling a message in round {}", my_uid, round);
                            continue; // tough is the life of the staller
                        }
                        if self.malicious_data.disrupt_round == round {
                            warn!("{} is disrupting a message in round {}", my_uid, round);
                            let mut t = traffic.clone();
                            t.payload = traffic.payload[0..traffic.payload.len() / 2].to_vec();
                            let mut m = msg.clone();
                            m.data = Some(proto::message_out::Data::Traffic(t));
                            delivery.deliver(&m, &my_uid);
                        }
                    }
                    delivery.deliver(&msg, &my_uid);
                }
                message_out::Data::KeygenResult(res) => {
                    info!("party [{}] keygen finished!", my_uid);
                    break Ok(res.clone());
                }
                _ => panic!("party [{}] keygen error: bad outgoing message type", my_uid),
            };
            msg_count += 1;
        };

        info!("party [{}] keygen execution complete", my_uid);
        result
    }
    async fn execute_key_presence(&mut self, key_uid: String) -> bool {
        let key_presence_request = KeyPresenceRequest {
            key_uid,
            pub_key: vec![],
        };

        let response = self
            .client
            .write()
            .await
            .key_presence(Request::new(key_presence_request))
            .await
            .unwrap()
            .into_inner();

        // prost way to convert i32 to enums https://github.com/danburkert/prost#enumerations
        match key_presence_response::Response::from_i32(response.response) {
            Some(key_presence_response::Response::Present) => true,
            Some(key_presence_response::Response::Absent) => false,
            Some(key_presence_response::Response::Fail) => {
                panic!("key presence request failed")
            }
            Some(key_presence_response::Response::Unspecified) => {
                panic!("Unspecified key presence response")
            }
            None => {
                panic!("Invalid key presence response. Could not convert i32 to enum")
            }
        }
    }

    async fn execute_sign(
        &mut self,
        init: SignInit,
        channels: SenderReceiver,
        delivery: Deliverer,
        my_uid: &str,
        notify: std::sync::Arc<tokio::sync::Notify>,
    ) -> GrpcSignResult {
        let (sign_server_incoming, rx) = channels;
        let mut sign_server_outgoing = self
            .client
            .write()
            .await
            .sign(Request::new(UnboundedReceiverStream::new(rx)))
            .await
            .unwrap()
            .into_inner();

        // TODO: support multiple shares for sign
        #[allow(unused_variables)] // allow unsused traffin in non malicious
        let all_share_count = init.party_uids.len();
        #[allow(unused_variables)] // allow unsused traffin in non malicious
        let my_share_count = 1;

        // the first outbound message is sign init info
        sign_server_incoming
            .send(MessageIn {
                data: Some(message_in::Data::SignInit(init)),
            })
            .unwrap();

        // block until all parties send their SignInit
        notify.notified().await;
        notify.notify_one();

        #[allow(unused_variables)] // allow unsused traffin in non malicious
        let mut msg_count = 1;

        let result = loop {
            let msg = match sign_server_outgoing.message().await {
                Ok(msg) => match msg {
                    Some(msg) => msg,
                    None => {
                        warn!(
                            "party [{}] sign execution was not completed due to abort",
                            my_uid
                        );
                        return Ok(SignResult::default());
                    }
                },
                Err(status) => {
                    warn!(
                        "party [{}] sign execution was not completed due to connection error: {}",
                        my_uid, status
                    );
                    return Err(status);
                }
            };

            let msg_type = msg.data.as_ref().expect("missing data");

            match msg_type {
                #[allow(unused_variables)] // allow unsused traffin in non malicious
                message_out::Data::Traffic(traffic) => {
                    // in malicous case, if we are stallers we skip the message
                    #[cfg(feature = "malicious")]
                    {
                        let round = sign_round(msg_count, all_share_count, my_share_count);
                        if self.malicious_data.timeout_round == round {
                            warn!("{} is stalling a message in round {}", my_uid, round - 4); // subtract keygen rounds
                            continue; // tough is the life of the staller
                        }
                        if self.malicious_data.disrupt_round == round {
                            warn!("{} is disrupting a message in round {}", my_uid, round);
                            let mut t = traffic.clone();
                            t.payload = traffic.payload[0..traffic.payload.len() / 2].to_vec();
                            let mut m = msg.clone();
                            m.data = Some(proto::message_out::Data::Traffic(t));
                            delivery.deliver(&m, my_uid);
                        }
                    }
                    delivery.deliver(&msg, my_uid);
                }
                message_out::Data::SignResult(res) => {
                    info!("party [{}] sign finished!", my_uid);
                    break Ok(res.clone());
                }
                message_out::Data::NeedRecover(_) => {
                    info!("party [{}] needs recover", my_uid);
                    // when recovery is needed, sign is canceled. We abort the protocol manualy instead of waiting parties to time out
                    // no worries that we don't wait for enough time, we will not be checking criminals in this case
                    delivery.send_timeouts(0);
                    break Ok(SignResult::default());
                }
                _ => panic!("party [{}] sign error: bad outgoing message type", my_uid),
            };
            msg_count += 1;
        };

        info!("party [{}] sign execution complete", my_uid);
        result
    }

    async fn shutdown(mut self) {
        // self.server_shutdown_sender.send(()).unwrap(); // tell the server to shut down
        // self.server_handle.await.unwrap(); // wait for server to shut down
        // info!("party [{}] shutdown success", self.server_port);
    }
}
