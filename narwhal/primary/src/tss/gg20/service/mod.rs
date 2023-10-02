//! This mod includes the service implementation derived from

use crate::tss::{
    kv_manager::KvManager,
    mnemonic::{self, Cmd},
};

use super::types::Config;

#[cfg(feature = "malicious")]
pub mod malicious;

/// Gg20Service
#[derive(Clone)]
pub struct Gg20Service {
    pub(super) kv_manager: KvManager,
    pub(super) cfg: Config,
}

impl Gg20Service {
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        let password = cfg.password_method.execute()?;
        let mnemonic_cmd = Cmd::Create;
        let mut kv_manager = KvManager::new(cfg.tofnd_path.clone(), password)?
            .handle_mnemonic(&mnemonic_cmd)
            .await?;
        let password = cfg.password_method.execute()?;
        kv_manager = KvManager::new(cfg.tofnd_path.clone(), password)?
            .handle_mnemonic(&Cmd::Existing)
            .await?;
        Ok(Self { kv_manager, cfg })
    }
}

// create a new Gg20 gRPC server
// pub async fn new_service(cfg: Config) -> Gg20Service {
//     let password = cfg.password_method.execute()?;
//     let kv_manager = KvManager::new(cfg.tofnd_path.clone(), password)?
//         .handle_mnemonic(&cfg.mnemonic_cmd)
//         .await?;
//     Gg20Service { kv_manager, cfg }
// }
