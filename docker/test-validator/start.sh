#!/bin/bash
DIR="$( cd "$( dirname "$0" )" && pwd )"
OS=$(uname)
BUILDER=sui-validator-builder
RUNNER=sui-validator-runner
COMPOSE_FILE=${DIR}/docker-compose.yaml
if [ "$OS" == "Darwin" ]
then
    ARCH=$(uname -m)
    if [ "$ARCH" == "arm64" ]
    then
        COMPOSE_FILE=${DIR}/docker-compose-arm64.yaml
    fi
fi 

init() {
  docker-compose -f ${COMPOSE_FILE} build
}

containers() {
  COMMAND=${1:-up}
  if [ "$COMMAND" == "up" ]
  then
    docker-compose -f ${COMPOSE_FILE} up --force-recreate -d
  else
    docker-compose -f ${COMPOSE_FILE} down
  fi  
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