#!/bin/bash
./chain-abci --host 0.0.0.0 --port 26658 --chain_id test-ab  --genesis_app_hash  2AC4B133256CD3B8DEDBDEA48A6C1ACDC6269BBA6214DF5308ECB815CE71942B     --enclave_server tcp://127.0.0.1:25933
