#!/bin/bash
rustup default nightly-2020-04-10
. ~/bin_cro/.profile
. ~/bin_cro/setup.sh
export CURRENT=$PWD
export DST=$HOME/bin_cro
export AR=/usr/bin/ar

cd $CURRENT
echo "tx-query compile"
RUSTFLAGS="-Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3,+pclmul" CFLAGS="-gz=none" cargo build --target=x86_64-fortanix-unknown-sgx -p  tx-query2-enclave-app  --release
cd ./target/x86_64-fortanix-unknown-sgx/release
echo $PWD
echo "start signing txquery"

#ftxsgx-elf2sgxs ./tx-query2-enclave-app --heap-size 0x2000000 --stack-size 0x80000 --threads 6  --debug

#sha256sum  ./tx-query2-enclave-app.sgxs  | awk '{print $1}' | xxd -r -p > $CURRENT/tqe.mrenclave
echo "tqe.mrenclave=$HOME/tqe.mrenclave"
#sgxs-sign --key $HOME/Enclave_private.pem ./tx-query2-enclave-app.sgxs ./tx-query2-enclave-app.sig -d --xfrm 7/0 --isvprodid 0 --isvsvn 0

#export TQE_SIGSTRUCT=./tx-query2-enclave-app.sig
#export TQE_MRENCLAVE=d84857c2a70fa18046dcdbdacf34cb3ad630103a1f0800ce44974ecd19f0b359
#export MRSIGNER=83d719e77deaca1470f6baf62a4d774303c899db69020f9c70ee1dfc08c7ce9e


#export TQE_SIGSTRUCT=./tx-query2-enclave-app.sig
#export TQE_MRENCLAVE=$(od -A none -t x1 --read-bytes=32 -j 960 -w32 $TQE_SIGSTRUCT | tr -d ' ')
#export MRSIGNER=$(dd if=$TQE_SIGSTRUCT bs=1 skip=128 count=384 status=none | sha256sum | awk '{print $1}')


export TQE_SIGSTRUCT=./tx-query2-enclave-app.sig
export TQE_MRENCLAVE=fce4275b6912b66736751ca698e27a46307d98b293528c8a0a865858e54e76ee
export MRSIGNER=83d719e77deaca1470f6baf62a4d774303c899db69020f9c70ee1dfc08c7ce9e

echo "TQE_SIGSTRUCT=" $TQE_SIGSTRUCT
echo "TQE_MRENCLAVE=" $TQE_MRENCLAVE
echo "MRSIGNER=" $MRSIGNER





cd $CURRENT
cargo build --release

RUSTFLAGS="-Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3,+pclmul" CFLAGS="-gz=none" cargo build --target=x86_64-fortanix-unknown-sgx -p mls --release
RUSTFLAGS="-Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3,+pclmul" CFLAGS="-gz=none" cargo build --target=x86_64-fortanix-unknown-sgx -p tx-validation-next  --release
cp ./target/x86_64-fortanix-unknown-sgx/release/mls* $DST
cp ./target/x86_64-fortanix-unknown-sgx/release/tx-query2-enclave-app* $DST
cp ./target/x86_64-fortanix-unknown-sgx/release/tx-validation-next* $DST
cp ./target/release/chain-abci $DST
cp ./target/release/client-cli $DST
cp ./target/release/client-rpc $DST
cp ./target/release/dev-utils $DST
cp ./target/release/ra-sp-server $DST
echo "COPY OK"
cd $DST
ftxsgx-elf2sgxs ./mls   --stack-size 0x40000 --heap-size 0x20000000 --threads 1
ftxsgx-elf2sgxs ./tx-validation-next   --heap-size 0x2000000 --stack-size 0x80000 --threads 6  --debug
sgxs-sign --key $HOME/Enclave_private.pem ./mls.sgxs ./mls.sig -d --xfrm 7/0 --isvprodid 0 --isvsvn 0
sgxs-sign --key $HOME/Enclave_private.pem ./tx-validation-next.sgxs ./tx-validation-next.sig -d --xfrm 7/0 --isvprodid 0 --isvsvn 0
echo "RUN ra-sp-server"
#$DST/ra-sp-server --quote-type Unlinkable --ias-key $IAS_API_KEY --spid $SPID &
sleep 2
echo "making keypackage key.txt"
#ftxsgx-runner ./mls.sgxs --signature coresident | base64 -w 0 > key.txt
#ftxsgx-runner ./mls.sgxs --signature coresident 
sleep 1
#killall ra-sp-server
cd $CURRENT
echo "OK"



echo "export TQE_SIGSTRUCT="$TQE_SIGSTRUCT
echo "export TQE_MRENCLAVE="$TQE_MRENCLAVE
echo "export MRSIGNER="$MRSIGNER


