use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use std::sync::mpsc::channel;
use crate::server::to_rpc_error;
use client_common::tendermint::Client;
use client_common::Storage;
use client_core::synchronizer::PollingSynchronizer;
use client_core::wallet::syncer::SyncCallback;
use client_core::wallet::syncer::{ObfuscationSyncerConfig, WalletSyncer};
use client_core::wallet::WalletRequest;
use client_core::TransactionObfuscation;
use client_core::wallet::syncer::{ProgressReport};
use std::thread;

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

pub struct SyncRpcImpl<S, C, O>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
{
    config: ObfuscationSyncerConfig<S, C, O>,
    polling_synchronizer: PollingSynchronizer,
    progress_callback: Option<SyncCallback>,
}

impl<S, C, O> SyncRpc for SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
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

impl<S, C, O> SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
{
    pub fn new(
        config: ObfuscationSyncerConfig<S, C, O>,
        progress_callback: Option<SyncCallback>,
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


        let (sender, receiver) = channel();
        let handle = thread::spawn(move || {
            let mut init_block_height = 0;
            let mut final_block_height = 0;            

            for progress_report in receiver.iter() {
                match progress_report {
                    ProgressReport::Init {
                        start_block_height,
                        finish_block_height,
                        ..
                    } => {
                        init_block_height = start_block_height;
                        final_block_height = finish_block_height;
                        
                        println!("Synchronizing: {}~{}", init_block_height, final_block_height);
                    }
                    ProgressReport::Update {
                        current_block_height,
                        ..
                    } => {
                        println!("current: {}", current_block_height);
                    }
                }
            }
        });



        let syncer = WalletSyncer::with_obfuscation_config(
            self.config.clone(),
            Some(sender),
            request.name,
            request.enckey,
            self.progress_callback.clone(),
        )
        .map_err(to_rpc_error)?;
        if reset {
            syncer.reset_state().map_err(to_rpc_error)?;
        }
        syncer.sync().map_err(to_rpc_error)
    }
}

impl<S, C, O> Drop for SyncRpcImpl<S, C, O>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
{
    fn drop(&mut self) {
        self.polling_synchronizer.stop();
    }
}
