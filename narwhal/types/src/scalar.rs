use ethers::types::{Block, H256};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalMessage {
    block: Block<H256>,
}

impl ExternalMessage {
    pub fn new(block: Block<H256>) -> Self {
        Self { block }
    }
}
