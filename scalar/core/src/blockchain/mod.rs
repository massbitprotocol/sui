//! The `blockchain` module exports the necessary traits and data structures to integrate a
//! blockchain into Graph Node. A blockchain is represented by an implementation of the `Blockchain`
//! trait which is the centerpiece of this module.

pub mod block_stream;
mod types;
use anyhow::{anyhow, Context, Error};
use async_trait::async_trait;
pub use types::*;
pub use types::{BlockHash, BlockPtr};
#[async_trait]
pub trait BlockIngestor: 'static + Send + Sync {
    async fn run(self: Box<Self>);
    fn network_name(&self) -> String;
}

pub trait Block: Send + Sync {
    fn ptr(&self) -> BlockPtr;
    fn parent_ptr(&self) -> Option<BlockPtr>;

    fn number(&self) -> i32 {
        self.ptr().number
    }

    fn hash(&self) -> BlockHash {
        self.ptr().hash
    }

    fn parent_hash(&self) -> Option<BlockHash> {
        self.parent_ptr().map(|ptr| ptr.hash)
    }

    /// The data that should be stored for this block in the `ChainStore`
    /// TODO: Return ChainStoreData once it is available for all chains
    fn data(&self) -> Result<serde_json::Value, serde_json::Error> {
        Ok(serde_json::Value::Null)
    }
}
