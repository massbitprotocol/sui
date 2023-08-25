#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

unset LD_PRELOAD
service nginx start
TSS_PORT=50010
GRPC_PORT=50050
TOFND=.tofnd
CONFIG_DIR=/scalar/config

validator() {
    # /usr/local/bin/sui-test-validator --config-path /opt/sui/config/validator.yaml
    # for i in {0..3}; do
    #     PORT=$(($TSS_PORT+$i))
    #     rm -rf tss/$TOFND$i
    #     /usr/local/bin/scalar-tofnd --no-password -d "tss/$TOFND$i" -m create -p ${PORT}
    #     echo "Inited tofnd instance $i at dir tss/$TOFND$i and port $PORT"
    # done

    /usr/local/bin/sui-test-validator \
        --epoch-duration-ms 60000 \
        --fullnode-rpc-port 9000 \
        --faucet-port 9123 \
        --indexer-rpc-port 9124
}

relayers() {
    /usr/local/bin/scalar-relayer --config  "${CONFIG_DIR}" \
        -h "http://127.0.0.1" \
        -p ${GRPC_PORT} \
        -n 4 
}

$@