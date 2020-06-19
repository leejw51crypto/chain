WASM BUILD GUIDE
----------------------------

## install emcc for compile WASM
1. use ubuntu 18.x
2. git clone https://github.com/emscripten-core/emsdk.git
3. cd emsdk
4. ./emsdk install latest
5. ./emsdk activate latest
6. source ./emsdk_env.sh

## check toolchain is OK
emcc -v

## install target

rustup target add wasm32-unknown-emscripten

## hello
1. cargo new hello
2. cd hello
3. vi ./src/main.rs
```
use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
fn main() {
    let secp = Secp256k1::new();
    println!("{:?}", secp);
}
```
4. vi ./Cargo.toml

add to dependencies
```
secp256k1 = ""
rand=""
```

## build
1. cargo build --target=wasm32-unknown-emscripten
2. cd target/wasm32-unknown-emscripten/debug/

## run wasm
```
node hello.js
```
## to build wasm file  
src/main.rs is necessary to build `WASM` file
1. make staticlib (or you can remove `[lib]`)
```
[lib]
crate-type = ["staticlib"]
```
2. you need to add `main.rs`
vi ./src/main.rs
3. edit like this
```
fn main()
{
    println!("WASM OK");
}
```

## how to build chain-core
$HOME/chain is the chain location
1. cd $HOME/chain/chain-core
2. cargo build --target=wasm32-unknown-emscripten --no-default-features
3. ls -la ../target/wasm32-unknown-emscripten/debug/*.wasm
```
../target/wasm32-unknown-emscripten/debug/chain_core.wasm
```

## run the WASM
1. cd ../target/wasm32-unknown-emscripten/debug/
2. node chain-core.js
```
chain-core wasm ok
```
3. you can confirm that WASM is working
   



