use crate::{
    authority::authority_per_epoch_store::AuthorityPerEpochStore,
    transaction_manager::TransactionManager,
};
use mysten_metrics::{monitored_scope, spawn_monitored_task};
use narwhal_types::ScalarEventTransaction;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

pub struct AsyncScalarTransactionScheduler {
    sender: tokio::sync::mpsc::Sender<Vec<ScalarEventTransaction>>,
}

impl AsyncScalarTransactionScheduler {
    pub fn start(
        tx_scalar_trans: UnboundedSender<Vec<ScalarEventTransaction>>,
        epoch_store: Arc<AuthorityPerEpochStore>,
    ) -> Self {
        let (sender, recv) = tokio::sync::mpsc::channel(16);
        spawn_monitored_task!(Self::run(tx_scalar_trans, recv, epoch_store));
        Self { sender }
    }

    pub async fn schedule(&self, transactions: Vec<ScalarEventTransaction>) {
        self.sender.send(transactions).await.ok();
    }

    pub async fn run(
        tx_scalar_trans: UnboundedSender<Vec<ScalarEventTransaction>>,
        mut recv: tokio::sync::mpsc::Receiver<Vec<ScalarEventTransaction>>,
        epoch_store: Arc<AuthorityPerEpochStore>,
    ) {
        while let Some(transactions) = recv.recv().await {
            if transactions.len() > 0 {
                info!("Handle transactions: {:?}", transactions);
                tx_scalar_trans.send(transactions);
            }
            // let _guard = monitored_scope("ConsensusHandler::enqueue");
            // transaction_manager
            //     .enqueue(transactions, &epoch_store)
            //     .expect("transaction_manager::enqueue should not fail");
        }
    }
}
