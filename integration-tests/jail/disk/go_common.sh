#!/bin/bash
service ssh start
cd /root/bin
echo "clear folders"
rm -rf /root/bin/.enclave
rm -rf /root/bin/.cro-storage
rm -rf /root/bin/.storage
rm -rf /enclave-storage
echo "copy binaries"
mkdir /root/bin
cp /root/disk/bin/* /root/bin
echo "clear disk"
/root/bin/tendermint unsafe_reset_all
sleep 2 
source /root/disk/prepare.sh
source /opt/sgxsdk/environment
source /root/.cargo/env
echo "sgx mode=" $SGX_MODE
echo "network id=" $NETWORK_ID
echo "path=" $PATH 
echo "enclave storage=" $TX_ENCLAVE_STORAGE
echo "rust flags=" $RUSTFLAGS
echo "app port=" $APP_PORT
echo "compile chain"
echo "ready"
