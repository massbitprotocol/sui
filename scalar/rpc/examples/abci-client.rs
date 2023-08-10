// pub mod pb {ScalarAbciRequest
//     tonic::include_proto!("grpc.examples.echo");
// }
// use pb::{echo_client::ScalarAbciClient, ScalarAbciRequest};
use scalar_rpc::proto::{ScalarAbciRequest, ScalarAbciResponse};
use scalar_rpc::scalar_abci_client::ScalarAbciClient;
use scalar_rpc::NAMESPACE;
use std::time::Duration;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;
fn echo_requests_iter() -> impl Stream<Item = ScalarAbciRequest> {
    tokio_stream::iter(1..usize::MAX).map(|i| ScalarAbciRequest {
        namespace: NAMESPACE.to_string(),
        message: format!("Simulated client transactions {:07}", i).into(),
    })
}

async fn streaming_scalar_abci(client: &mut ScalarAbciClient<Channel>, num: usize) {
    let stream = client
        .server_streaming_scalar_abci(ScalarAbciRequest {
            namespace: NAMESPACE.to_string(),
            message: "foo".into(),
        })
        .await
        .unwrap()
        .into_inner();

    // stream is infinite - take just 5 elements and then disconnect
    let mut stream = stream.take(num);
    while let Some(item) = stream.next().await {
        println!(
            "\treceived: {}",
            String::from_utf8(item.unwrap().message).unwrap()
        );
    }
    // stream is droped here and the disconnect info is send to server
}

async fn bidirectional_streaming_scalar_abci(client: &mut ScalarAbciClient<Channel>, num: usize) {
    let in_stream = echo_requests_iter().take(num);

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
}

async fn bidirectional_streaming_scalar_abci_throttle(
    client: &mut ScalarAbciClient<Channel>,
    dur: Duration,
) {
    let in_stream = echo_requests_iter().throttle(dur);

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let address = "http://[::1]:50051";
    let address = "http://127.0.0.1:50051";
    let mut client = ScalarAbciClient::connect(address).await.unwrap();

    // println!("Streaming echo:");
    // streaming_scalar_abci(&mut client, 5).await;
    // tokio::time::sleep(Duration::from_secs(1)).await; //do not mess server println functions

    // Echo stream that sends 17 requests then graceful end that connection
    println!("\r\nBidirectional stream echo:");
    bidirectional_streaming_scalar_abci(&mut client, 1000000).await;

    // Echo stream that sends up to `usize::MAX` requests. One request each 2s.
    // Exiting client with CTRL+C demonstrate how to distinguish broken pipe from
    // graceful client disconnection (above example) on the server side.
    // println!("\r\nBidirectional stream echo (kill client with CTLR+C):");
    // bidirectional_streaming_scalar_abci_throttle(&mut client, Duration::from_secs(2)).await;

    Ok(())
}
