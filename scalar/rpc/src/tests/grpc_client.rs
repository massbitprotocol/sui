use crate::gg20_client;
use anyhow::anyhow;
pub async fn connect_grpc_server() -> Result<(), anyhow::Error> {
    let grpc_host = "validator-runner";
    let grpc_port = 50050;
    let grpc_addr = format!("http://{}:{}", grpc_host, grpc_port);
    // match ScalarAbciClient::connect(grpc_addr.clone()).await {
    //     Ok(_) => {
    //         println!("Connected");
    //         Ok(())
    //     }
    //     Err(_) => {
    //         println!("Connect failed");
    //         Err(anyhow!("Cannot connect to tss server"))
    //     }
    // }
    Ok(())
}
