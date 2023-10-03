//! This mod includes the service implementation derived from

use super::types::{Config, Entropy, PartyInfo};
use crate::tss::gg20::Password;
use crate::tss::kv_manager::error::{InnerKvError, KvError};
use crate::tss::kv_manager::{kv_store::KvStore, store::Store};
use crate::tss::mnemonic::bip39_bindings::bip39_seed;
use crate::tss::mnemonic::results::mnemonic::{InnerMnemonicError, InnerMnemonicResult};
use crate::{block_synchronizer::handler::Error, tss::mnemonic};
use anyhow::anyhow;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use storage::TssStore;
use tofn::gg20::keygen::PartyKeygenData;
use tofn::sdk::api::TofnFatal;
use tofn::{
    gg20::keygen::SecretRecoveryKey,
    sdk::api::{deserialize, serialize},
};
use tracing::{error, info, warn};
use types::KeyReservation;
#[cfg(feature = "malicious")]
pub mod malicious;

/// Gg20Service
#[derive(Clone)]
pub struct Gg20Service {
    pub(super) kv_store: KvStore,
}

impl Gg20Service {
    pub fn new(tss_store: TssStore, safe_keygen: bool) -> Gg20Service {
        let kv_store = KvStore::new(tss_store, safe_keygen);
        Self { kv_store }
    }
    pub fn kv(&self) -> &KvStore {
        &self.kv_store
    }
}

impl Gg20Service {
    pub async fn get_party_info(&self, key: &KeyReservation) -> anyhow::Result<PartyInfo> {
        let party_info: PartyInfo = self
            .kv()
            .get(key)
            .await
            .map_err(|err| anyhow!("Get party_info error {:?}", err))?;
        Ok(party_info)
    }
    // pub async fn put(&self, key: &KeyReservation, value: V) -> anyhow::Result<()> {
    //     let bytes = serialize(&value).map_err(|_| anyhow!("SerializationErr"))?;
    //     match self.kv().put(key, &bytes).await {
    //         Err(e) => Err(anyhow!("{:?}", &e)),
    //         Ok(_) => Ok(()),
    //     }
    // }
    pub async fn reserve_key(&self, key: &KeyReservation) -> anyhow::Result<String> {
        self.kv()
            .reserve_key(key)
            .await
            .map_err(|err| anyhow!("Reserve_key error {:?}", &err))
    }
    pub async fn unreserve_key(&self, key: &KeyReservation) -> anyhow::Result<()> {
        self.kv()
            .remove(key)
            .await
            .map_err(|err| anyhow!("Unreserve key with error {:?}", err))
    }

    pub async fn init_mnemonic(&self) -> anyhow::Result<()> {
        info!("Init mnemonic");
        self.kv()
            .init_mnemonic()
            .await
            .map_err(|err| anyhow!("Init mnemonic error {:?}", &err))
    }
}
