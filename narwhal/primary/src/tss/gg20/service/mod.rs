//! This mod includes the service implementation derived from

use crate::tss::kv_manager::KvManager;

use super::types::Config;

#[cfg(feature = "malicious")]
pub mod malicious;

/// Gg20Service
#[derive(Clone)]
pub struct Gg20Service {
    pub(super) kv_manager: KvManager,
    pub(super) cfg: Config,
}

/// create a new Gg20 gRPC server
pub fn new_service(
    cfg: Config,
    kv_manager: KvManager,
) -> impl crate::tss::narwhal_types::gg20_server::Gg20 {
    Gg20Service { kv_manager, cfg }
}
