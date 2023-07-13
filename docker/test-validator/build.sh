#!/bin/sh
SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
docker exec -it sui-validator-builder /sui/build.sh
# docker cp ${SCRIPT_DIR}/../../target/debug/sui-test-validator sui-validator-runner:/usr/local/bin
# docker cp ${SCRIPT_DIR}/../../target/debug/sui sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/debug/sui-test-validator sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/debug/sui sui-validator-runner:/usr/local/bin
