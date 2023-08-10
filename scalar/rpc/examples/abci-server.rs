use scalar_rpc::proto::echo_server::{Echo, EchoServer};
use scalar_rpc::proto::{EchoRequest, EchoResponse};
use scalar_rpc::NAMESPACE;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let abci_service = GrpcService {};
    Server::builder()
        .add_service(ScalarAbciServer::new(abci_service))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
