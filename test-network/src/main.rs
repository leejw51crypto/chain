use chain_core::init::address::RedeemAddress;
use chain_core::tx::data::address::ExtendedAddr;

use chain_core::state::account::StakedState;
use chain_core::state::account::StakedStateAddress;
use chain_tx_validation::witness::verify_tx_recover_address;
use client_common::storage::MemoryStorage;
use client_common::tendermint::{Client, RpcClient};
use client_core::signer::DefaultSigner;
use client_core::wallet::DefaultWalletClient;
use client_core::{PrivateKey, PublicKey};
use client_network::network_ops::{DefaultNetworkOpsClient, NetworkOpsClient};
use jsonrpc::client::Client as JsonRpcClient;
use parity_codec::{Decode, Encode};
use serde_json::{json, Value};
use std::convert::TryFrom;
use std::str::FromStr;

fn test1() {
    println!("test network");
    let tendermint_url = "http://localhost:26657/";
    let storage = MemoryStorage::default();
    let signer = DefaultSigner::new(storage.clone());

    let wallet_client = DefaultWalletClient::builder()
        .with_wallet(storage)
        .build()
        .unwrap();
    let tendermint_client = RpcClient::new(&tendermint_url);
    let network_ops_client =
        DefaultNetworkOpsClient::new(&wallet_client, &signer, &tendermint_client);

    let b = "0x0db221c4f57d5d38b968139c06e9132aaf84e8df".as_bytes();
    let d = &b[2..];
    let e = hex::decode(d).unwrap();
    let g = StakedStateAddress::try_from(e.as_slice()).unwrap();
    let nonce = network_ops_client.get_staked_state_nonce(g).unwrap();
    println!("Nonce={}", nonce);
}
fn test2() {
    let height = 0;
    let params = [json!(null)];
    let mut client = jsonrpc::client::Client::new("http://localhost:26657".to_owned(), None, None);
    let request = client.build_request("validators", &params);
    client.send_request(&request).and_then(|res| {
        println!("{:?}", res);
        Ok(res)
    });
}
fn test3() {
    let height = 0;
    let b = hex::decode("0db221c4f57d5d38b968139c06e9132aaf84e8df").unwrap();
    let c = hex::encode(b);
    let params = [json!("account"), json!(c), json!(null), json!(null)];
    let params2 = [json!(null)];
    /*
        {
      "jsonrpc": "2.0",
      "id": "",
      "result": {
        "response": {
          "value": "AAAAAAAAAAAAAAAAAAAAAAAAeiLByLEia/aSXAAAAAAADbIhxPV9XTi5aBOcBukTKq+E6N8="
        }
      }
    }
        */
    println!("{:?}", params);
    let mut client = jsonrpc::client::Client::new("http://localhost:26657".to_owned(), None, None);
    let request = client.build_request("abci_query", &params);
    client.send_request(&request).and_then(|res| {
        // serde_json::value::Value
        {
            let m = res.result.clone().unwrap();
            let res = m.get("response").unwrap();
            let mut v = res.get("value").unwrap().as_str().unwrap();

            // let s= StakedState::decode(&mut v.as_bytes());
            let mut dataraw = v.as_bytes();
            let mut data = base64::decode(dataraw).unwrap();
            let account: Option<StakedState> = StakedState::decode(&mut data.as_slice());

            println!("OK={:?}", data);
            println!("OK={:?}", v);
            println!("OK={:?}", account);
        }
        Ok(res)
    });
    /*
    let request = client.build_request("validators", &params);
    client.send_request(&request).and_then(|res| {
        println!("{:?}", res);
        Ok(res)
    });*/
}
fn main() {
    let tendermint_url = "http://localhost:26657/";
    let storage = MemoryStorage::default();
    let signer = DefaultSigner::new(storage.clone());

    let wallet_client = DefaultWalletClient::builder()
        .with_wallet(storage)
        .build()
        .unwrap();
    let tendermint_client = RpcClient::new(&tendermint_url);
    let network_ops_client =
        DefaultNetworkOpsClient::new(&wallet_client, &signer, &tendermint_client);
    let e = hex::decode("0db221c4f57d5d38b968139c06e9132aaf84e8df").unwrap();
    let g = StakedStateAddress::try_from(e.as_slice()).unwrap();
    println!("StakeStateAddress {:?}", g);
    let nonce = network_ops_client.get_staked_state_nonce(g).unwrap();
    println!("Nonce={}", nonce);
}
