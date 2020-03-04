use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use crate::server::to_rpc_error;
use client_common::tendermint::Client;
use client_common::Storage;
use client_core::synchronizer::PollingSynchronizer;
use client_core::wallet::syncer::SyncCallback;
use client_core::wallet::syncer::{ObfuscationSyncerConfig, WalletSyncer};
use client_core::wallet::WalletRequest;
use client_core::TransactionObfuscation;

#[rpc]
pub trait SyncRpc: Send + Sync {
    #[rpc(name = "sync")]
    fn sync(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_all")]
    fn sync_all(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_unlockWallet")]
    fn sync_unlock_wallet(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_stop")]
    fn sync_stop(&self, request: WalletRequest) -> Result<()>;
}

pub struct SyncRpcImpl<'a,S, C, O,F>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
    F: Fn(u64, u64, u64)->i32,
{
    config: ObfuscationSyncerConfig<S, C, O>,
    polling_synchronizer: PollingSynchronizer,
    progress_callback: &'a Option<SyncCallback<F>>,
}

impl<'a,S, C, O,F> SyncRpc for SyncRpcImpl<'a,S, C, O,F>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
    F: Fn(u64, u64, u64)->i32 + 'static + Send + Sync,
{
    #[inline]
    fn sync(&self, request: WalletRequest) -> Result<()> {
        self.do_sync(request, false)
    }

    #[inline]
    fn sync_all(&self, request: WalletRequest) -> Result<()> {
        self.do_sync(request, true)
    }

    #[inline]
    fn sync_unlock_wallet(&self, request: WalletRequest) -> Result<()> {
        self.polling_synchronizer
            .add_wallet(request.name, request.enckey);
        Ok(())
    }

    #[inline]
    fn sync_stop(&self, request: WalletRequest) -> Result<()> {
        self.polling_synchronizer.remove_wallet(&request.name);
        Ok(())
    }
}

impl<'a,S, C, O,F> SyncRpcImpl<'a,S, C, O,F>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
    F: Fn(u64, u64, u64)->i32 + 'static,

{
    pub fn new(
        config: ObfuscationSyncerConfig<S, C, O>,
        progress_callback: Option<SyncCallback<F>>,
    ) -> Self {
        let mut polling_synchronizer = PollingSynchronizer::default();
        polling_synchronizer.spawn(config.clone());

        SyncRpcImpl {
            config,
            polling_synchronizer,
            progress_callback,
        }
    }

    fn do_sync(&self, request: WalletRequest, reset: bool) -> Result<()> {
        let syncer = WalletSyncer::with_obfuscation_config(
            self.config.clone(),
            None,
            request.name,
            request.enckey,
            &self.progress_callback,
        )
        .map_err(to_rpc_error)?;
        if reset {
            syncer.reset_state().map_err(to_rpc_error)?;
        }
        syncer.sync().map_err(to_rpc_error)
    }
}

impl<'a,S, C, O,F> Drop for SyncRpcImpl<'a,S, C, O,F>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
    F: Fn(u64, u64, u64)->i32 ,
{
    fn drop(&mut self) {
        self.polling_synchronizer.stop();
    }
}
