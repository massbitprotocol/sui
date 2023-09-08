pub mod grpc_node;
pub mod proto;
pub mod tests;
pub mod tss;
pub mod tss_network;
pub mod tss_types;
pub use grpc_node::*;
pub use proto::*;
pub use tss::*;
pub use tss_network::*;
pub use tss_types::*;
pub const NAMESPACE: &str = "scalar";
pub const NUM_SHUTDOWN_RECEIVERS: u64 = 8;
pub mod tss_anemo {
    include!(concat!(env!("OUT_DIR"), "/tss.network.TssPeer.rs"));
}
pub use tss_anemo::{
    tss_peer_client::TssPeerClient,
    tss_peer_server::{TssPeer, TssPeerServer},
};
