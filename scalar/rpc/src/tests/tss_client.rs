use crate::gg20_client;
use anyhow::anyhow;
pub async fn connect_tss_server() -> Result<(), anyhow::Error> {
    let tss_host = "validator-runner";
    let tss_port = 50010;
    let tss_addr = format!("{}:{}", tss_host, tss_port);
    match gg20_client::Gg20Client::connect(tss_addr.clone()).await {
        Ok(_) => {
            println!("Connected");
            Ok(())
        }
        Err(_) => {
            println!("Connect failed");
            Err(anyhow!("Cannot connect to tss server"))
        }
    }
}
