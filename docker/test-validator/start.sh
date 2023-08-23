#!/bin/sh
DIR="$( cd "$( dirname "$0" )" && pwd )"
BUILDER=sui-validator-builder
RUNNER=sui-validator-runner
init() {
  docker-compose -f ${DIR}/docker-compose.yaml build
}

containers() {
  docker-compose -f ${DIR}/docker-compose.yaml up --force-recreate -d
}

build() {
  docker exec -it ${BUILDER} /sui/build.sh
  # docker cp ${SCRIPT_DIR}/../../target/debug/sui-test-validator sui-validator-runner:/usr/local/bin
  # docker cp ${SCRIPT_DIR}/../../target/debug/sui sui-validator-runner:/usr/local/bin
  docker cp ${DIR}/builder/debug/sui-test-validator ${RUNNER}:/usr/local/bin
  docker cp ${DIR}/builder/debug/sui ${RUNNER}:/usr/local/bin
}

curl() {
  curl --location --request POST 'http://127.0.0.1:9000' \
  --header 'Content-Type: application/json' \
  --data-raw '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "sui_getTotalTransactionBlocks",
    "params": []
  }'
}

builder() {
  docker exec -it ${BUILDER} bash
}

runner() {
  docker exec -it ${RUNNER} bash
}

$@