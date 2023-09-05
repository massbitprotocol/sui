mod config;
mod proto;
use crate::proto::{ScalarAbciClient, ScalarAbciRequest};
use anyhow::anyhow;
use clap::Parser;
use config::*;
use ethers::prelude::*;
use futures::future::join_all;
use std::fs;
use tokio::sync::mpsc;
use tokio::{self, task::JoinHandle};
use tokio_stream::{Stream, StreamExt};
use tracing::info;
pub const NAMESPACE: &str = "scalar";
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    set_up_logs();
    //let address = "http://[::1]:50051";
    //let address = "http://127.0.0.1:50051";
    let args = Args::parse();
    let mut all_handlers = vec![];
    for i in 0..args.instance {
        let config = format!("{}/evm_relayer{}.toml", args.config.as_str(), i);
        let grpc_addr = format!("{}:{}", &args.grpc_host, args.grpc_port + i);
        info!("Start relayer with Rpc address {:?}", grpc_addr.as_str());
        println!("Start relayer with Rpc address {:?}", grpc_addr.as_str());
        if let Ok(handlers) = start_relayer(config, grpc_addr).await {
            all_handlers.extend(handlers);
        }
    }
    let _ = join_all(all_handlers).await;
    println!("Exist relayers");
    Ok(())
}

fn set_up_logs() {
    // enable only tofnd and tofn debug logs - disable serde, tonic, tokio, etc.
    tracing_subscriber::fmt()
        .with_env_filter("relayer=debug")
        .json()
        .with_ansi(atty::is(atty::Stream::Stdout))
        .with_target(false)
        .with_current_span(false)
        .flatten_event(true) // make logs complient with datadog
        .init();
}

async fn start_relayer(
    config: String,
    grpc_addr: String,
) -> Result<Vec<JoinHandle<()>>, anyhow::Error> {
    let mut handlers = vec![];
    let (tx_external_event, mut rx_external_event) = mpsc::channel(128);
    let config_str = fs::read_to_string(config.as_str())
        .map_err(|e| {
            let msg = format!("{:?}", e);
            println!("{}", msg.as_str());
            anyhow!(msg)
        })
        .expect(format!("Failed to read relayer config file {}", config.as_str()).as_str());
    let relayer_configs: RelayerConfigs = toml::from_str(config_str.as_str()).unwrap();
    let ws_handlers = relayer_configs
        .scalar_relayer_evm
        .into_iter()
        .map(|config| {
            let tx = tx_external_event.clone();
            tokio::spawn(async move {
                // A Ws provider can be created from a ws(s) URI.
                // In case of wss you must add the "rustls" or "openssl" feature
                // to the ethers library dependency in `Cargo.toml`
                if let (Some(ws_url), Some(start)) = (config.ws_addr, config.start_with_bridge) {
                    if start {
                        let provider = Provider::<Ws>::connect(ws_url.as_str()).await.expect(
                            format!("Cannot connect to websocket url {:?}", ws_url.as_str())
                                .as_str(),
                        );
                        info!("Connected to websocket endpoint {}", ws_url.as_str());
                        println!("Connected to websocket endpoint {}", ws_url.as_str());
                        let mut stream =
                            provider.subscribe_blocks().await.expect("Cannot subscribe");
                        while let Some(block) = stream.next().await {
                            //Received event from source chain
                            //Broadcast a poll
                            //Send it to the worker for create poll
                            info!("Received evm block {:?}", &block.hash);
                            println!("Received evm block {:?}", &block.hash);
                            let _ = tx.send(block).await;
                        }
                    }
                }
            })
        });
    handlers.extend(ws_handlers);
    //let addr = grpc_addr.clone();
    if let Ok(mut client) = ScalarAbciClient::connect(grpc_addr.clone()).await {
        let grpc_handler = tokio::spawn(async move {
            // Echo stream that sends 17 requests then graceful end that connection
            // println!("\r\nBidirectional stream echo:");
            let in_stream = async_stream::stream! {
                while let Some(item) = rx_external_event.recv().await {
                    let value = serde_json::to_string(&item).unwrap();
                    println!("Push block into stream {:?}", &value);
                    yield ScalarAbciRequest{
                        namespace: String::from(NAMESPACE),
                        message: value.as_bytes().to_vec(),
                    };
                }
            };
            //let in_stream = echo_requests_iter().take(num);

            let response = client
                .bidirectional_streaming_scalar_abci(in_stream)
                .await
                .unwrap();

            let mut resp_stream = response.into_inner();

            while let Some(received) = resp_stream.next().await {
                let received = received.unwrap();
                println!(
                    "\treceived message: `{}`",
                    String::from_utf8(received.message).unwrap()
                );
            }
            // Echo stream that sends up to `usize::MAX` requests. One request each 2s.
            // Exiting client with CTRL+C demonstrate how to distinguish broken pipe from
            // graceful client disconnection (above example) on the server side.
            // println!("\r\nBidirectional stream echo (kill client with CTLR+C):");
            // bidirectional_streaming_scalar_abci_throttle(&mut client, Duration::from_secs(2)).await;
            // bidirectional_streaming_scalar_abci(&mut client, 1000000).await;
        });
        handlers.push(grpc_handler);
    } else {
        println!("Cannot connect to rpc address {}", grpc_addr.as_str());
    }
    Ok(handlers)
}
