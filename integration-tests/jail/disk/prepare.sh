#!/bin/bash
export SGX_MODE=HW
export NETWORK_ID=AB
export SPID=4F883E85867521D3B8CD9F6DFB7F4FB0
export IAS_API_KEY=d120972e39fb484e8519f0347bda6973
export RUSTFLAGS=-Ctarget-feature=+aes,+ssse3
export PATH=$HOME/.cargo/bin:$HOME/bin:$PATH
export APP_PORT=25933
export TX_ENCLAVE_STORAGE=/enclave-storage
export LD_LIBRARY_PATH=$HOME/lib
export PKG_CONFIG_PATH=$HOME/lib/pkgconfig
source ~/.bashrc
