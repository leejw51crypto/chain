#!/bin/bash
./chain-abci --host 0.0.0.0 --port 26658 --chain_id test-ab  --genesis_app_hash  E212B0DBF5BC8396775A9EDFAEC5DA51E2D4E8276E31210026B1F2E474191294     --enclave_server tcp://127.0.0.1:25933
