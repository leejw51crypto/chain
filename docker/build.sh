#!/bin/bash
set -e
cd "$(dirname "${BASH_SOURCE[0]}")"

if [ -f $SGX_SDK/environment ]; then
    source $SGX_SDK/environment
fi

BUILD_PROFILE=${BUILD_PROFILE:-debug}
BUILD_MODE=${BUILD_MODE:-sgx}

if [ $BUILD_PROFILE == "debug" ]; then
    export SGX_DEBUG=1
    CARGO_ARGS=
else
    export SGX_DEBUG=0
    CARGO_ARGS=--release
fi

cd ..

if [ $BUILD_MODE == "sgx" ]; then
    echo "Build sgx"
    cargo build $CARGO_ARGS
    cargo build $CARGO_ARGS -p tx-validation-app
    cargo build $CARGO_ARGS -p tx-query-app
    make -C chain-tx-enclave/tx-validation
    make -C chain-tx-enclave/tx-query
else
    echo "Build mock"
    cargo build $CARGO_ARGS --features mock-enc-dec --features mock-validation --manifest-path chain-abci/Cargo.toml
    cargo build $CARGO_ARGS --features mock-enc-dec  --manifest-path client-rpc/Cargo.toml
    cargo build $CARGO_ARGS --features mock-enc-dec  --manifest-path client-cli/Cargo.toml
    cargo build $CARGO_ARGS -p dev-utils
fi
