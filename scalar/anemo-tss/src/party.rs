use std::vec;

//use crate::TssClient;
use crate::{KeygenRequest, TssPeerClient, TssPeerServer, TssPeerService};
use anemo::{
    types::{Address, PeerEvent, PeerInfo},
    Network, Peer, PeerId,
};
use anemo_tower::trace::TraceLayer;
use tokio::task::JoinHandle;
use tower::Layer;
use tracing::{error, info, warn};
pub struct TssParty {
    pub service: Network,
    //pub client: TssClient,
}

impl TssParty {
    pub fn new(service: Network) -> Self {
        Self { service }
    }
    pub async fn spawn_servers(server_name: String, comittee: Vec<PeerInfo>) -> Vec<Network> {
        //let mut handles = vec![];
        let mut networks = comittee
            .iter()
            .map(|peer_info| {
                info!(
                    "Address {:?}, PeerId {:?}",
                    peer_info.address.get(0).unwrap().clone(),
                    peer_info.peer_id.clone()
                );

                let network = Network::bind(peer_info.address.get(0).unwrap().clone())
                    .private_key(peer_info.peer_id.0.clone())
                    .server_name(server_name.clone())
                    .outbound_request_layer(TraceLayer::new_for_client_and_server_errors())
                    .start(TssPeerServer::new(TssPeerService::default()))
                    .unwrap();
                network
            })
            .collect::<Vec<Network>>();
        networks.iter_mut().for_each(|network| {
            Self::add_peers_to_network(network, comittee.clone());
        });
        networks
    }
    pub fn create_network(
        address: String,
        private_key: [u8; 32],
        server_name: String,
    ) -> anyhow::Result<Network> {
        Network::bind(address)
            .private_key(private_key.clone())
            .server_name(server_name)
            .outbound_request_layer(TraceLayer::new_for_client_and_server_errors())
            .start(TssPeerServer::new(TssPeerService::default()))
    }
    fn add_peers_to_network(network: &mut Network, comittee: Vec<PeerInfo>) {
        let peer_id = network.peer_id();
        comittee
            .iter()
            .filter(|info| info.peer_id != peer_id)
            .for_each(|info| {
                network.known_peers().insert(info.clone());
            });
    }
    pub async fn spawn_clients(networks: Vec<Network>) -> Vec<JoinHandle<()>> {
        let mut handles = vec![];
        for network in networks.iter() {
            let peer_id = network.connect(network.local_addr()).await.unwrap();
            //let peer_id = network.peer_id();
            let peer = network.peer(peer_id).unwrap();
            let handle = TssParty::spawn_client(peer).await;
            handles.push(handle);
        }
        handles
    }
    pub async fn spawn_server<A>(
        addr: A,
        private_key: [u8; 32],
        server_name: String,
    ) -> JoinHandle<()>
    where
        A: Into<Address> + Clone,
    {
        let peer_id = PeerId(private_key.clone());
        let network = Network::bind(addr)
            .private_key(private_key.clone())
            .server_name(server_name)
            .outbound_request_layer(TraceLayer::new_for_client_and_server_errors())
            .start(TssPeerServer::new(TssPeerService::default()))
            .unwrap();
        info!(
            "Starting tss node with private key {:?}",
            PeerId(private_key)
        );
        tokio::spawn(async move {
            // if let Some(mut client) = network.peer(peer_id).map(|peer| TssPeerClient::new(peer)) {
            //     let response = client
            //         .keygen(KeygenRequest {
            //             name: "Brandon".into(),
            //         })
            //         .await
            //         .unwrap();

            //     info!("{:#?}", response);
            // } else {
            //     warn!("Cannot create peer client");
            // }
            if let Ok((mut receiver, mut peers)) = network.subscribe() {
                //info!("Subscribed to network {:?}", PeerId(private_key));
                let peer_id = {
                    if peers.is_empty() {
                        match receiver.recv().await.unwrap() {
                            PeerEvent::NewPeer(peer_id) => peer_id,
                            PeerEvent::LostPeer(_, _) => todo!(),
                        }
                    } else {
                        peers.pop().unwrap()
                    }
                };
                info!("Subscribed to network {:?}", &peer_id);
                let peer = network.peer(peer_id).unwrap();
                let mut client = TssPeerClient::new(peer);
                let response = client
                    .keygen(KeygenRequest {
                        name: "Test".to_string(),
                    })
                    .await;
                // .unwrap()
                // .into_inner();
                info!("request response {:?}", response);
            } else {
                error!("Cannot subscribe to the network");
            }
        })
    }
    pub async fn spawn_client(peer: Peer) -> JoinHandle<()> {
        let mut client = TssPeerClient::new(peer);
        tokio::spawn(async move {
            let keygen_request = KeygenRequest {
                name: "keygenrequest".to_string(),
            };
            let response = client.keygen(keygen_request).await;
            println!("{:?}", response);
            loop {
                //info!("Inside loop");
            }
        })
    }
}
