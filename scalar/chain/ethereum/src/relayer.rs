use serde::Deserialize;

/// This is what we're going to decode into. Each field is optional, meaning
/// that it doesn't have to be present in TOML.
#[derive(Clone, Debug, Deserialize)]
pub struct RelayerConfigs {
    pub scalar_relayer_evm: Vec<RelayerConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RelayerConfig {
    pub name: Option<String>,
    pub contract_addr: Option<String>,
    pub rpc_addr: Option<String>,
    pub ws_addr: Option<String>,
    pub start_with_bridge: Option<bool>,
}

pub trait Relayer {}
