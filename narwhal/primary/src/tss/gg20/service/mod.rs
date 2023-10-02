//! This mod includes the service implementation derived from

use super::types::Config;
use crate::{
    block_synchronizer::handler::Error,
    tss::{
        kv_manager::KvManager,
        mnemonic::{self, Cmd},
    },
};
use anyhow::anyhow;
use tracing::{error, info, warn};
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
        // Try create mnomonic
        // match KvManager::new(cfg.tofnd_path.clone(), password.clone())?
        //     .handle_mnemonic(&Cmd::Create)
        //     .await
        // {
        //     Ok(_) => {
        //         info!("Create mnomonic successfully");
        //     }
        //     Err(e) => {
        //         warn!("Create mnomonic with error {:?}", e);
        //     }
        // }
        match KvManager::new(cfg.tofnd_path.clone(), password)?
            .handle_mnemonic(&Cmd::Existing)
            .await
        {
            Ok(kv_manager) => Ok(Self { kv_manager, cfg }),
            Err(e) => {
                error!("Create KvManager with error {:?}", &e);
                Err(anyhow!(format!("Create KvManager with error: {:?}", &e)))
            }
        }
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
