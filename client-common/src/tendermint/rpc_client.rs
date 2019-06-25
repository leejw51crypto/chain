#![cfg(feature = "rpc")]

use failure::ResultExt;
use jsonrpc::client::Client as JsonRpcClient;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::tendermint::types::*;
use crate::tendermint::Client;
use crate::{ErrorKind, Result};

/// Tendermint RPC Client
#[derive(Clone)]
pub struct RpcClient {
    url: String,
}

impl RpcClient {
    /// Creates a new instance of `RpcClient`
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
        }
    }

    fn call<T>(&self, name: &str, params: &[Value]) -> Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        // jsonrpc does not handle Hyper connection reset properly. The current
        // inefficient workaround is to create a new client on every call.
        // https://github.com/apoelstra/rust-jsonrpc/issues/26
        let client = JsonRpcClient::new(self.url.to_owned(), None, None);
        let request = client.build_request(name, params);

        let response = client.send_request(&request).context(ErrorKind::RpcError)?;

        let result = response.result::<T>().context(ErrorKind::RpcError)?;

        Ok(result)
    }
}

impl Client for RpcClient {
    fn genesis(&self) -> Result<Genesis> {
        self.call("genesis", Default::default())
    }

    fn status(&self) -> Result<Status> {
        self.call("status", Default::default())
    }

    fn block(&self, height: u64) -> Result<Block> {
        let params = [json!(height.to_string())];
        self.call("block", &params)
    }

    fn block_results(&self, height: u64) -> Result<BlockResults> {
        let params = [json!(height.to_string())];
        self.call("block_results", &params)
    }

    fn broadcast_transaction(&self, transaction: &[u8]) -> Result<()> {
        let params = [json!(transaction)];
        self.call::<serde_json::Value>("broadcast_tx_sync", &params)
            .map(|_| ())
    }

    fn get_account(&self, staked_state_address: &[u8]) -> Result<Account> {
        let params = [
            json!({"path":"account"}),
            json!({ "data": hex::encode(staked_state_address) }),
        ];

        self.call("abci_query", &params)
    }
}
