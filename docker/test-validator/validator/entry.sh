#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

unset LD_PRELOAD
service nginx start
#/usr/local/bin/sui-test-validator --config-path /opt/sui/config/validator.yaml
/usr/local/bin/sui-test-validator \
    --epoch-duration-ms 60000 \
    --fullnode-rpc-port 9000 \
    --faucet-port 9123 \
    --indexer-rpc-port 9124
    
