use crate::proto::tofnd::{MessageIn, MessageOut};
use crate::{
    message_out, TssAnemoKeygenMessage, TssAnemoKeygenRequest, TssAnemoKeygenResponse,
    TssAnemoSignRequest, TssAnemoSignResponse, TssAnemoVerifyRequest, TssAnemoVerifyResponse,
    TssPeer, TssPeerClient,
};
use anemo::rpc::Status;
use anemo::Response;
use anemo::{Network, PeerId};
use anyhow::format_err;
use anyhow::Result;
use futures::future::join_all;
use narwhal_crypto::NetworkPublicKey;
use narwhal_network::{CancelOnDropHandler, RetryConfig};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};

#[derive(Default)]
pub struct TssPeerService {}
#[anemo::async_trait]
impl TssPeer for TssPeerService {
    async fn keygen(
        &self,
        request: anemo::Request<TssAnemoKeygenRequest>,
    ) -> Result<Response<TssAnemoKeygenResponse>, Status> {
        let body = request.into_body();
        info!("Received keygen request: {:?}", &body);
        let reply = TssAnemoKeygenResponse {
            message: format!("Hello {}!", body.message.from_party_uid),
        };
        Ok(Response::new(reply))
    }
    async fn sign(
        &self,
        request: anemo::Request<TssAnemoSignRequest>,
    ) -> Result<Response<TssAnemoSignResponse>, Status> {
        let reply = TssAnemoSignResponse {
            message: format!("Hello {}!", request.into_body().name),
        };
        Ok(Response::new(reply))
    }
    async fn verify(
        &self,
        request: anemo::Request<TssAnemoVerifyRequest>,
    ) -> Result<Response<TssAnemoVerifyResponse>, Status> {
        let reply = TssAnemoVerifyResponse {
            message: format!("Hello {}!", request.into_body().name),
        };
        Ok(Response::new(reply))
    }
}
#[derive(Clone)]
pub struct AnemoDeliverer {
    pub network: Network,
    pub peers: Vec<NetworkPublicKey>,
}

impl AnemoDeliverer {
    pub fn new(network: Network, peers: Vec<NetworkPublicKey>) -> Self {
        AnemoDeliverer { network, peers }
    }
    pub async fn deliver(&self, msg: &MessageOut, from: &str) {
        info!(
            "broadcast tss message from {:?} to peers {:?}",
            from, &self.peers
        );
        let msg = msg.data.as_ref().expect("missing data");
        let msg = match msg {
            message_out::Data::Traffic(t) => t,
            _ => {
                panic!("msg must be traffic out");
            }
        };
        let message = TssAnemoKeygenMessage {
            from_party_uid: from.to_string(),
            is_broadcast: msg.is_broadcast,
            payload: msg.payload.clone(),
        };

        let handlers = self.network.broadcast(self.peers.clone(), &message);
        let res = join_all(handlers).await;
        info!("Broadcast results {:?}", res);
    }
    pub fn get_channels(&self) -> (UnboundedSender<MessageIn>, UnboundedReceiver<MessageIn>) {
        mpsc::unbounded_channel()
    }
    pub fn send_timeouts(&self, secs: u64) {
        info!("Send timeouts");
    }
    pub fn get_notifier(&self) -> std::sync::Arc<tokio::sync::Notify> {
        std::sync::Arc::new(tokio::sync::Notify::new())
    }
}

pub trait TssNodeRpc<Request: Clone + Send + Sync> {
    type Response: Clone + Send + Sync;

    fn send(
        &self,
        peer: NetworkPublicKey,
        message: &Request,
    ) -> CancelOnDropHandler<Result<anemo::Response<Self::Response>>>;

    fn send_request(
        &self,
        peer: NetworkPublicKey,
        message: &Request,
    ) -> JoinHandle<Result<anemo::Response<Self::Response>, anemo::rpc::Status>>;

    fn broadcast(
        &self,
        peers: Vec<NetworkPublicKey>,
        message: &Request,
    ) -> Vec<JoinHandle<Result<anemo::Response<Self::Response>, anemo::rpc::Status>>> {
        let mut handles = Vec::new();
        for peer in peers {
            let handle = self.send_request(peer, message);
            handles.push(handle);
        }
        handles
    }
    fn _broadcast(
        &self,
        peers: Vec<NetworkPublicKey>,
        message: &Request,
    ) -> Vec<CancelOnDropHandler<Result<anemo::Response<Self::Response>>>> {
        let mut handlers = Vec::new();
        for peer in peers {
            let handle = self.send(peer, message);
            handlers.push(handle);
        }
        handlers
    }
}

impl TssNodeRpc<TssAnemoKeygenMessage> for anemo::Network {
    type Response = TssAnemoKeygenResponse;
    fn send(
        &self,
        peer: NetworkPublicKey,
        message: &TssAnemoKeygenMessage,
    ) -> CancelOnDropHandler<Result<anemo::Response<TssAnemoKeygenResponse>>> {
        let message = message.to_owned();
        let f = move |peer| {
            //let message = message.clone();
            info!("Anemo send message {:?}", &message);
            let request = TssAnemoKeygenRequest {
                message: message.clone(),
            };
            async move { TssPeerClient::new(peer).keygen(request).await }
        };

        send(self.clone(), peer, f)
    }

    fn send_request(
        &self,
        peer: NetworkPublicKey,
        message: &TssAnemoKeygenMessage,
    ) -> JoinHandle<Result<anemo::Response<Self::Response>, anemo::rpc::Status>> {
        let peer_id = PeerId(peer.0.to_bytes());
        let message = message.to_owned();
        let peer = self
            .peer(peer_id.clone())
            .ok_or_else(|| format_err!("AnemoTss has no connection with peer {peer_id}"))
            .unwrap();
        let handle = tokio::spawn(async move {
            let request = TssAnemoKeygenRequest {
                message: message.clone(),
            };
            info!(
                "AnemoTss send keygen message {:?} to the peer, {}",
                &message,
                peer_id.to_string()
            );
            TssPeerClient::new(peer).keygen(request).await
        });
        handle
    }
}

fn send<F, R, Fut>(
    network: anemo::Network,
    peer: NetworkPublicKey,
    f: F,
) -> CancelOnDropHandler<Result<anemo::Response<R>>>
where
    F: Fn(anemo::Peer) -> Fut + Send + Sync + 'static + Clone,
    R: Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = Result<anemo::Response<R>, anemo::rpc::Status>> + Send,
{
    // Safety
    // Since this spawns an unbounded task, this should be called in a time-restricted fashion.

    let peer_id = PeerId(peer.0.to_bytes());
    info!("Try send message to AnemoTss peer {}", peer_id.to_string());
    let message_send = move || {
        let network = network.clone();
        let f = f.clone();

        async move {
            if let Some(peer) = network.peer(peer_id) {
                f(peer).await.map_err(|e| {
                    // this returns a backoff::Error::Transient
                    // so that if anemo::Status is returned, we retry
                    backoff::Error::transient(anyhow::anyhow!("AnemoTss RPC error: {e:?}"))
                })
            } else {
                Err(backoff::Error::transient(anyhow::anyhow!(
                    "AnemoTss has node connection with peer {peer_id}"
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
