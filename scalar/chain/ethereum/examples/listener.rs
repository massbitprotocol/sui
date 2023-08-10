use common::{
    endpoint::EndpointMetrics,
    env::env_var,
    firehose,
    firehose::FirehoseEndpoint,
    grpc,
    log::logger,
    prelude::{prost::Message, Error, MetricsRegistry},
};
use scalar_chain_ethereum::codec;
use std::sync::Arc;
use tonic::Streaming;
const URL: &str = "https://eth-mainnet.g.alchemy.com/v2/9u1mZJtSKl2NgzRA9i0rh5_QIPQi4pTU";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut cursor: Option<String> = None;
    let token_env = env_var("CHAIN_ACCESS_TOKEN", "".to_string());
    let mut token: Option<String> = None;
    if !token_env.is_empty() {
        token = Some(token_env);
    }
    let host = URL.to_string();
    let logger = logger(false);
    let metrics = Arc::new(EndpointMetrics::new(
        logger,
        &[host.clone()],
        Arc::new(MetricsRegistry::mock()),
    ));
    let grpc = Arc::new(grpc::new("grpc", &host, token, false, false, metrics));

    loop {
        println!("Connecting to the stream!");
        let request = grpc::GRpcBlockRequest {
            start_block_num: 12369739,
            stop_block_num: 12369739,
            cursor: match &cursor {
                Some(c) => c.clone(),
                None => String::from(""),
            },
            final_blocks_only: false,
            ..Default::default()
        };
        let mut stream: Streaming<grpc::Response> = match grpc.clone().stream_blocks(request).await
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
            let b = codec::Block::decode(resp.block.unwrap().value.as_ref());
        }
    }
    Ok(())
}
