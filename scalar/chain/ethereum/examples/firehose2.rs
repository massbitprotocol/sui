use core::prelude::{web3, Error};
use core::{
    endpoint::EndpointMetrics,
    env::env_var,
    log::logger,
    prelude::{prost, tokio, tonic, MetricsRegistry},
    {firehose, firehose::FirehoseEndpoint},
};
use hex_literal::hex;
use std::sync::Arc;
use tonic::Streaming;
use tracing::{debug, error, info};
use tracing_subscriber;
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let _ = tracing::subscriber::set_global_default(subscriber);
    let url = "wss://eth-mainnet.g.alchemy.com/v2/9u1mZJtSKl2NgzRA9i0rh5_QIPQi4pTU";
    //let url = "wss://api.streamingfast.io:443";
    // let transport = web3::transports::WebSocket::new(url).await?;
    // let web3 = web3::Web3::new(transport);
    info!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    // println!("Calling accounts.");
    // let mut accounts = web3.eth().accounts().await?;
    // println!("Accounts: {:?}", accounts);
    // accounts.push(hex!("00a329c0648769a73afac7f9381e08fb43dbea72").into());

    // println!("Calling balance.");
    // for account in accounts {
    //     let balance = web3.eth().balance(account, None).await?;
    //     println!("Balance of {:?}: {}", account, balance);
    // }
    let mut cursor: Option<String> = None;
    let token_env = env_var("SF_API_TOKEN", "".to_string());
    let mut token: Option<String> = None;
    if !token_env.is_empty() {
        token = Some(token_env);
    }
    let logger = logger(false);
    //let host = "https://api.streamingfast.io:443".to_string();
    let host = url.to_string();
    let metrics = Arc::new(EndpointMetrics::new(
        logger,
        &[host.clone()],
        Arc::new(MetricsRegistry::mock()),
    ));
    let firehose = Arc::new(FirehoseEndpoint::new(
        "firehose", &host, token, false, false, metrics,
    ));
    loop {
        println!("Connecting to the stream!");
        let mut stream: Streaming<firehose::Response> = match firehose
            .clone()
            .stream_blocks(firehose::Request {
                start_block_num: 12369739,
                stop_block_num: 12369739,
                cursor: match &cursor {
                    Some(c) => c.clone(),
                    None => String::from(""),
                },
                final_blocks_only: false,
                ..Default::default()
            })
            .await
        {
            Ok(s) => s,
            Err(e) => {
                println!("Could not connect to stream! {}", e);
                continue;
            }
        };
        loop {
            let resp = match stream.message().await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    println!("Stream completed");
                    return Ok(());
                }
                Err(e) => {
                    println!("Error getting message {}", e);
                    break;
                }
            };
            println!("Received message. Tobo continuing...");
        }
    }
    Ok(())
}
