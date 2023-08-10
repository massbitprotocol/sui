use ethers::prelude::*;
use eyre;
//use web3;
//const WSS_URL: &str = "wss://eth-mainnet.g.alchemy.com/v2/9u1mZJtSKl2NgzRA9i0rh5_QIPQi4pTU";
const WSS_URL: &str = "wss://mainnet.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // A Ws provider can be created from a ws(s) URI.
    // In case of wss you must add the "rustls" or "openssl" feature
    // to the ethers library dependency in `Cargo.toml`.
    let provider = Provider::<Ws>::connect(WSS_URL).await?;

    let mut stream = provider.subscribe_blocks().await?;
    while let Some(block) = stream.next().await {
        //println!("{:?}", block.hash);
        println!("{:?}", block);
    }

    Ok(())
}
