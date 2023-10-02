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
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use storage::TssStore;
use tofn::sdk::api::{deserialize, serialize};
use tracing::{error, info, warn};
use types::KeyReservation;
#[cfg(feature = "malicious")]
pub mod malicious;

/// Gg20Service
#[derive(Clone)]
pub struct Gg20Service {
    //pub(super) kv_manager: KvManager,
    //pub(super) cfg: Config,
    pub(super) tss_store: TssStore,
}

impl Gg20Service {
    pub fn new(tss_store: TssStore) -> Gg20Service {
        Self { tss_store }
    }
}

impl Gg20Service {
    pub async fn get<V>(&self, key: &KeyReservation) -> anyhow::Result<V>
    where
        V: Serialize + DeserializeOwned,
    {
        match self.tss_store.read(key).await {
            Ok(Some(value)) => deserialize(value.as_slice()).ok_or(anyhow!("DeserializeError")),
            _ => Err(anyhow!("Cannot get value with key {:?}", key)),
        }
    }
    pub async fn put<V>(&self, key: &KeyReservation, value: V) -> anyhow::Result<()>
    where
        V: Debug + Send + Sync + Serialize + DeserializeOwned,
    {
        let bytes = serialize(&value).map_err(|_| anyhow!("SerializationErr"))?;
        match self.tss_store.write(key, &bytes).await {
            Err(e) => Err(anyhow!("{:?}", &e)),
            Ok(_) => Ok(()),
        }
    }
    pub async fn reserve_key(&self, key: &KeyReservation) -> anyhow::Result<String> {
        self.tss_store
            .reserve_key(key)
            .await
            .map_err(|err| anyhow!("Reserve_key error {:?}", &err))
    }
    pub async fn unreserve_key(&self, key: &KeyReservation) -> anyhow::Result<()> {
        self.tss_store
            .remove(key)
            .await
            .map_err(|err| anyhow!("Unreserve key with error {:?}", err))
    }
    // pub async fn new(cfg: Config) -> anyhow::Result<Self> {
    //     let password = cfg.password_method.execute()?;
    //     // Try create mnomonic
    //     // match KvManager::new(cfg.tofnd_path.clone(), password.clone())?
    //     //     .handle_mnemonic(&Cmd::Create)
    //     //     .await
    //     // {
    //     //     Ok(_) => {
    //     //         info!("Create mnomonic successfully");
    //     //     }
    //     //     Err(e) => {
    //     //         warn!("Create mnomonic with error {:?}", e);
    //     //     }
    //     // }
    //     match KvManager::new(cfg.tofnd_path.clone(), password)?
    //         .handle_mnemonic(&Cmd::Existing)
    //         .await
    //     {
    //         Ok(kv_manager) => Ok(Self { kv_manager, cfg }),
    //         Err(e) => {
    //             error!("Create KvManager with error {:?}", &e);
    //             Err(anyhow!(format!("Create KvManager with error: {:?}", &e)))
    //         }
    //     }
    // }
}

// create a new Gg20 gRPC server
// pub async fn new_service(cfg: Config) -> Gg20Service {
//     let password = cfg.password_method.execute()?;
//     let kv_manager = KvManager::new(cfg.tofnd_path.clone(), password)?
//         .handle_mnemonic(&cfg.mnemonic_cmd)
//         .await?;
//     Gg20Service { kv_manager, cfg }
// }
