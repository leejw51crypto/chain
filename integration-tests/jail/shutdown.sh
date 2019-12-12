#!/bin/bash
export JAIL_CLIENT_RPC=9981
export JAIL_CHAIN_RPC=26657
export APP_HASH=00000000
export CURRENT_HASH=$(git rev-parse HEAD)
echo "shutdown CURRENT_HASH=" $CURRENT_HASH
docker-compose -p $CURRENT_HASH down
