#!/bin/sh
SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
PROFILE=debug
RUNNER=sui-validator-runner
#BIN_DIR=${SCRIPT_DIR}/../../target/${PROFILE}
BIN_DIR=${SCRIPT_DIR}/validator/${PROFILE}
docker exec -it sui-validator-builder /sui/build.sh
docker cp ${BIN_DIR}/sui-test-validator ${RUNNER}:/usr/local/bin
docker cp ${BIN_DIR}/sui ${RUNNER}:/usr/local/bin
docker cp ${BIN_DIR}/scalar-tss ${RUNNER}:/usr/local/bin

docker cp ${SCRIPT_DIR}/../../scalar/tofnd/target/${PROFILE}/scalar-tofnd ${RUNNER}:/usr/local/bin
#docker cp ${SCRIPT_DIR}/../../scalar/relayer/target/${PROFILE}/scalar-relayer ${RUNNER}:/usr/local/bin
