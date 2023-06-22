// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{setup_db_state, SnapshotReader};
use sui_types::sui_system_state::epoch_start_sui_system_state::EpochStartSystemStateTrait;
use sui_types::sui_system_state::get_sui_system_state;
use sui_types::sui_system_state::SuiSystemStateTrait;

pub struct SnapshotRestorer<S> {
    config: SnapshotRestoreConfig,
    store: S,
    archive_balancer: Arc<ArchiveReaderBalancer>,
    local_object_store_path: PathBuf,
}

impl SnapshotRestorer {
    pub async fn new<S>(
        config: SnapshotRestoreConfig,
        store: S,
        archive_balancer: Arc<ArchiveReaderBalancer>,
        local_object_store_path: PathBuf,
    ) -> Result<Self>
    where
        S: WriteStore,
        <S as ReadStore>::Error: std::error::Error,
    {
        let snapshot_reader = StateSnapshotReaderV1::new(
            config.epoch,
            &config.object_store_config,
            ObjectStoreConfig {
                directory: Some(local_object_store_path),
                ..Default::default()
            },
            usize::MAX,
            config.download_concurrency,
        )
        .await?;

        Self {
            config,
            store,
            reader: Arc::new(snapshot_reader),
            archive_balancer,
            local_object_store_path,
        }
    }

    pub fn run(&self) {
        let epoch = config.epoch;
        let checkpoint_sync_handle = tokio::task::spawn(async move {
            let archive_latest = self
                .archive_balancer
                .get_archive_watermark()
                .await?
                .expect("Archive watermark not set");
            let archive_reader = self.archive_balancer.pick_one_random(0..archive_latest);
            archive_reader
                .load_summaries_upto(snapshot_restore_config.epoch.saturating_add(1))
                .await?;
        });

        let (sha3_digests, accumulator) = snapshot_reader.get_checksums()?;
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let read_handle = tokio::task::spawn(async move {
            snapshot_reader
                .read(&perpetual_tables, sha3_digests, abort_registration)
                .await?
        });

        // Upon successful exit of this task, we should have all checkpoint summaries
        // from genesis to the restore epoch, as well as the final checkpoint mapping
        // populated. Get last checkpoint of restore epoch for root state hash commitment.
        checkpoint_sync_handle.await?;
        let checkpoint_summary = checkpoint_store.epoch_last_checkpoint_map(epoch).await?;

        let commitments = checkpoint_summary
            .end_of_epoch_data
            .unwrap_or_else(|| panic!("Expected end of epoch checkpoint summary to be returned"))
            .checkpoint_commitments;
        if commitments.is_empty() {
            panic!(
                "Formal snapshot verification not supported for epoch {} on this node",
                epoch
            );
        }

        if accumulator.digest() != commitment[0] {
            abort_handle.abort();
            panic!(
                "Formal snapshot accumulator digest ({}) does not match root state digest commitment ({}) for epoch {}",
                epoch,
                acc.digest(),
                commitment[0],
            );
        }
        info!("Formal snapshot verification passed for epoch {}", epoch);
        read_handle.await?;

        self.setup_db_state(
            epoch,
            accumulator,
            perpetual_db,
            checkpoint_store,
            committee_store,
        )
        .await?

        // TODOS:
        // Fix checkpoint executor watermark stuff
        // Finish db restore stuff
        // Do hard linking stuff
    }

    pub async fn setup_db_state(
        &self,
        epoch: u64,
        accumulator: Accumulator,
        perpetual_db: Arc<AuthorityPerpetualTables>,
        checkpoint_store: Arc<CheckpointStore>,
        committee_store: Arc<CommitteeStore>,
    ) -> Result<()> {
        // This function should be called once state accumulator based hash verification
        // is complete and live object set state is downloaded to local store
        let system_state_object = get_sui_system_state(&perpetual_db)?;
        let new_epoch_start_state = system_state_object.into_epoch_start_state();
        let next_epoch_committee = new_epoch_start_state.get_sui_committee();
        let last_checkpoint = checkpoint_store
            .get_epoch_last_checkpoint(epoch)
            .expect("Error loading last checkpoint for current epoch")
            .expect("Could not load last checkpoint for current epoch");
        let epoch_start_configuration =
            EpochStartConfiguration::new(new_epoch_start_state, *last_checkpoint.digest());
        perpetual_db
            .set_epoch_start_configuration(&epoch_start_configuration)
            .await?;
        perpetual_db.insert_root_state_hash(epoch, last_checkpoint.sequence_number, accumulator)?;
        perpetual_db.set_highest_pruned_checkpoint_without_wb(last_checkpoint.sequence_number)?;
        committee_store.insert_new_committee(&next_epoch_committee)?;
        checkpoint_store.update_highest_executed_checkpoint(&last_checkpoint)?;

        Ok(())
    }
}
