pub mod grpc_node;
pub mod proto;
pub use grpc_node::*;
pub use proto::*;
pub const NAMESPACE: &str = "scalar";
pub const NUM_SHUTDOWN_RECEIVERS: u64 = 8;
