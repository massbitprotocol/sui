// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::NodeStorage;
use store::rocks::ReadWriteOptions;
use store::rocks::{open_cf, DBMap, MetricConf};
use store::{reopen, Map, TypedStoreError};
use sui_macros::fail_point;
use types::{EventDigest, EventVerify};

#[derive(Clone)]
pub struct EventStore {
    store: DBMap<EventDigest, EventVerify>,
}

impl EventStore {
    pub fn new(event_store: DBMap<EventDigest, EventVerify>) -> Self {
        Self { store: event_store }
    }

    pub fn new_for_tests() -> Self {
        let rocksdb = open_cf(
            tempfile::tempdir().unwrap(),
            None,
            MetricConf::default(),
            &[NodeStorage::EVENTS_CF],
        )
        .expect("Cannot open database");
        let map = reopen!(&rocksdb, NodeStorage::EVENTS_CF;<EventDigest, EventVerify>);
        Self::new(map)
    }

    pub fn read(&self, id: &EventDigest) -> Result<Option<EventVerify>, TypedStoreError> {
        self.store.get(id)
    }

    #[allow(clippy::let_and_return)]
    pub fn write(&self, event: &EventVerify) -> Result<(), TypedStoreError> {
        fail_point!("narwhal-store-before-write");

        let result = self.store.insert(&event.digest(), event);

        fail_point!("narwhal-store-after-write");
        result
    }

    pub fn remove_all(
        &self,
        keys: impl IntoIterator<Item = EventDigest>,
    ) -> Result<(), TypedStoreError> {
        self.store.multi_remove(keys)
    }
}
