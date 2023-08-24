#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

unset LD_PRELOAD
service nginx start
TSS_PORT=50010
TOFND=.tofnd
#/usr/local/bin/sui-test-validator --config-path /opt/sui/config/validator.yaml
for i in {1..4}; do
    PORT=$(($TSS_PORT+$i))
    rm -rf tss/$TOFND$i
    /usr/local/bin/scalar-tofnd --no-password -d "tss/$TOFND$i" -m create -p ${PORT}
    echo "Inited tofnd instance $i at dir tss/$TOFND$i and port $PORT"
done
/usr/local/bin/sui-test-validator \
    --epoch-duration-ms 60000 \
    --fullnode-rpc-port 9000 \
    --faucet-port 9123 \
    --indexer-rpc-port 9124
    
