#!/bin/bash
./chain-abci --host 0.0.0.0 --port 26658 --chain_id test-ab  --genesis_app_hash 06F2C8A94B439884C3A8B65E9BFE64AE03A4793008C76599B300DA91C5FB006E    --enclave_server tcp://127.0.0.1:25933
