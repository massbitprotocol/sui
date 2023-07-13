use hex_literal::hex;
use tracing::{debug, error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> web3::Result<()> {
    //tracing_subscriber::fmt::init();
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let _ = tracing::subscriber::set_global_default(subscriber);
    let url = "wss://eth-mainnet.g.alchemy.com/v2/9u1mZJtSKl2NgzRA9i0rh5_QIPQi4pTU";
    //let url = "wss://api.streamingfast.io:443";
    let transport = web3::transports::WebSocket::new(url).await?;
    let web3 = web3::Web3::new(transport);
    info!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    println!("Calling accounts.");
    let mut accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);
    accounts.push(hex!("00a329c0648769a73afac7f9381e08fb43dbea72").into());

    println!("Calling balance.");
    for account in accounts {
        let balance = web3.eth().balance(account, None).await?;
        println!("Balance of {:?}: {}", account, balance);
    }

    Ok(())
}
