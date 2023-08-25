use clap::Parser;
use serde::Deserialize;
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Name of the person to greet
    #[arg(short = 'c', long)]
    pub config: String,
    /// Grpc address
    #[arg(short = 'h', long)]
    pub grpc_host: String,
    #[arg(short = 'p', long)]
    pub grpc_port: u32,
    #[arg(short = 'n', long)]
    pub instance: u32,
}

/// This is what we're going to decode into. Each field is optional, meaning
/// that it doesn't have to be present in TOML.
#[derive(Debug, Deserialize)]
pub struct RelayerConfigs {
    pub scalar_relayer_evm: Vec<RelayerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RelayerConfig {
    pub name: Option<String>,
    pub rpc_addr: Option<String>,
    pub ws_addr: Option<String>,
    pub start_with_bridge: Option<bool>,
}
