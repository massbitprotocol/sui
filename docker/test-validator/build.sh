#!/bin/sh
SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
PROFILE=debug
RUNNER=sui-validator-runner
docker exec -it sui-validator-builder /sui/build.sh
# docker cp ${SCRIPT_DIR}/../../target/debug/sui-test-validator sui-validator-runner:/usr/local/bin
# docker cp ${SCRIPT_DIR}/../../target/debug/sui sui-validator-runner:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/${PROFILE}/sui-test-validator ${RUNNER}:/usr/local/bin
docker cp ${SCRIPT_DIR}/validator/${PROFILE}/sui ${RUNNER}:/usr/local/bin
docker cp ${SCRIPT_DIR}/../../scalar/tofnd/target/${PROFILE}/scalar-tofnd ${RUNNER}:/usr/local/bin
docker cp ${SCRIPT_DIR}/../../scalar/relayer/target/${PROFILE}/scalar-relayer ${RUNNER}:/usr/local/bin
