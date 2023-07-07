#!/bin/sh
DIR="$( cd "$( dirname "$0" )" && pwd )"
docker-compose -f ${DIR}/docker-compose.yaml up --force-recreate -d
curl --location --request POST 'http://127.0.0.1:9000' \
--header 'Content-Type: application/json' \
--data-raw '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "sui_getTotalTransactionBlocks",
  "params": []
}'
docker exec -it sui-validator-runner bash