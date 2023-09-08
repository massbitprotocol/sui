use std::{collections::HashMap, vec};

use anemo::{
    types::{Address, PeerEvent, PeerInfo},
    Network, PeerId,
};
use anemo_tower::trace::TraceLayer;
use futures::future::join_all;
use scalar_tss::{TssParty, TssPeerClient, TssPeerServer, TssPeerService};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tower::Layer;
use tracing::{info, trace};
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // tracing_subscriber::fmt::init();
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;
    let mut handles = vec![];
    let mut comittee = vec![];
    let server_name = String::from("tss");
    //Start servers
    for i in 0..4 {
        let private_key = random_key();
        let addr = format!("localhost:{}", 50000 + i);
        info!("Start network at address {}", &addr);
        let peer_info = PeerInfo {
            peer_id: PeerId(private_key.clone()),
            affinity: anemo::types::PeerAffinity::High,
            address: vec![addr.clone().into()],
        };
        comittee.push(peer_info);
        // let network =
        //     TssParty::create_network(addr, private_key.clone(), server_name.clone()).unwrap();
        // networks.push(network);
        //Start client
        // let handle = TssParty::spawn_client("localhost:0", private_key, server_name).await;
        // handles.push(handle);
    }

    let networks = TssParty::spawn_servers(server_name, comittee).await;
    for network in networks.clone().into_iter() {
        //let peer_id = network.connect(network.local_addr()).await.unwrap();
        // let peer = network.peer(peer_id).unwrap();
        // let client_handle = TssParty::spawn_client(peer).await;
        // handles.push(client_handle);
        // network.broadcast();
        let network_handle = tokio::spawn(async move {
            let (mut receiver, mut peers) = network.subscribe().unwrap();
            loop {
                match receiver.recv().await {
                    Ok(PeerEvent::NewPeer(peer_id)) => {
                        info!("New peer {:?}", peer_id);
                    }
                    Ok(PeerEvent::LostPeer(peer_id, _)) => {
                        info!("Lost peer {:?}", peer_id);
                    }
                    Err(RecvError::Closed) => {
                        panic!("PeerEvent channel shouldn't be able to be closed");
                    }

                    Err(RecvError::Lagged(_)) => {
                        trace!("State-Sync fell behind processing PeerEvents");
                    }
                }
            }
            // let peer_id = {
            //     if peers.is_empty() {
            //         match receiver.recv().await.unwrap() {
            //             PeerEvent::NewPeer(peer_id) => peer_id,
            //             PeerEvent::LostPeer(_, _) => todo!(),
            //         }
            //     } else {
            //         peers.pop().unwrap()
            //     }
            // };
            // loop {}
        });

        handles.push(network_handle);
    }
    let size = networks.len();
    // for i in 0..size {
    //     let cur_network = networks.get(i).unwrap();
    //     let next_network = networks.get((i + 1) % size).unwrap();
    //     let peer_id = cur_network
    //         .connect(next_network.local_addr())
    //         .await
    //         .unwrap();
    //     let peer = cur_network.peer(peer_id).unwrap();
    //     let client_handle = TssParty::spawn_client(peer).await;
    //     handles.push(client_handle);
    // }
    // for network in networks.into_iter() {
    //     info!("Network address {:?}", network.local_addr());
    //     let peer_id = network.connect(network.local_addr()).await.unwrap();
    //     let peer = network.peer(peer_id).unwrap();
    //     let client_handle = TssParty::spawn_client(peer).await;
    //     handles.push(client_handle);
    // }
    //let client_handles = TssParty::spawn_clients(networks).await;
    //handles.extend(client_handles);
    println!("Waiting for all tss threads");
    join_all(handles).await;
    println!("Main thread");
    Ok(())
}

fn random_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rng, &mut bytes[..]);
    bytes
}
