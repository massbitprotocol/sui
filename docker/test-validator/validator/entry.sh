#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

unset LD_PRELOAD
#service nginx start
TSS_PORT=50010
GRPC_PORT=50050
TOFND=.tofnd
CONFIG_DIR=/scalar/config

tss() {
    PORT=$TSS_PORT
    for ind in {0..3}
    do
        rm -rf tss/${TOFND}${ind}
    done
    /usr/local/bin/scalar-tofnd --no-password -d "tss/$TOFND" -p ${PORT} -m create
    #echo "Inited tofnd at dir tss/$TOFND and port $PORT"
    for ind in {0..3}
    do
        rm tss/${TOFND}${ind}/export
    done
    #echo "Running tofnd at dir tss/$TOFND and port $PORT as daemon"
    /usr/local/bin/scalar-tofnd --no-password -d "tss/$TOFND" -p ${PORT} -m existing
    #echo "Finished tofnd at dir tss/$TOFND and port $PORT"
}

validator() {
    # /usr/local/bin/sui-test-validator --config-path /opt/sui/config/validator.yaml
    /usr/local/bin/sui-test-validator \
        --epoch-duration-ms 60000 \
        --fullnode-rpc-port 9000 \
        --faucet-port 9123 \
        --indexer-rpc-port 9124
}

relayer() {
    /usr/local/bin/scalar-relayer --configdir  "${CONFIG_DIR}" \
        -h "http://127.0.0.1" \
        -p ${GRPC_PORT} 
}

$@