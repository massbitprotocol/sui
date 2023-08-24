#!/bin/sh
SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
PROFILE=debug
docker exec -it sui-validator-builder /sui/build.sh
# docker cp ${SCRIPT_DIR}/../../target/debug/sui-test-validator sui-validator-runner:/usr/local/bin
# docker cp ${SCRIPT_DIR}/../../target/debug/sui sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/${PROFILE}/sui-test-validator sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/${PROFILE}/sui sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/../../scalar/tofnd/target/${PROFILE}/scalar-tofnd sui-validator-runner:/usr/local/bin
