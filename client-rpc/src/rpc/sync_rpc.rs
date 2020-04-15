use super::sync_worker::SyncWorker;
use super::sync_worker::WorkerShared;
use crate::server::rpc_error_from_string;
use crate::server::to_rpc_error;
use client_common::tendermint::Client;
use client_common::Storage;
use client_core::synchronizer::PollingSynchronizer;
use client_core::wallet::syncer::ProgressReport;
use client_core::wallet::syncer::{ObfuscationSyncerConfig, WalletSyncer};
use client_core::wallet::WalletRequest;
use client_core::TransactionObfuscation;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub trait CBindingCallback: Send + Sync {
    fn progress(&mut self, current: u64, start: u64, end: u64) -> i32;
    fn set_user(&mut self, user: u64);
    fn get_user(&self) -> u64;
}

#[derive(Clone)]
pub struct CBindingCore {
    pub data: Arc<Mutex<dyn CBindingCallback>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RunSyncResult {
    name: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RunSyncProgressResult {
    pub name: String,
    pub message: String,
    pub percent: f32,
    pub current: u64,
    pub start: u64,
    pub end: u64,
}

#[rpc]
pub trait SyncRpc: Send + Sync {
    #[rpc(name = "sync")]
    fn sync(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "run_sync")]
    fn run_sync(&self, request: WalletRequest) -> Result<RunSyncResult>;

    #[rpc(name = "run_sync_progress")]
    fn run_sync_progress(&self, request: WalletRequest) -> Result<RunSyncProgressResult>;

    #[rpc(name = "sync_all")]
    fn sync_all(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "run_sync_all")]
    fn run_sync_all(&self, request: WalletRequest) -> Result<RunSyncResult>;

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
    progress_callback: Option<CBindingCore>,
    worker: WorkerShared,
}

fn process_sync<S, C, O>(
    config: ObfuscationSyncerConfig<S, C, O>,
    request: WalletRequest,
    reset: bool,
    progress_callback: Option<CBindingCore>,
) -> Result<()>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
{
    let syncer = WalletSyncer::with_obfuscation_config(config, request.name, request.enckey)
        .map_err(to_rpc_error)?;
    if reset {
        syncer.reset_state().map_err(to_rpc_error)?;
    }

    if progress_callback.is_none() {
        return syncer.sync(|_| true).map_err(to_rpc_error);
    }

    let mut init_block_height = 0;
    let mut final_block_height = 0;
    syncer
        .sync(|report: ProgressReport| -> bool {
            match report {
                ProgressReport::Init {
                    start_block_height,
                    finish_block_height,
                    ..
                } => {
                    init_block_height = start_block_height;
                    final_block_height = finish_block_height;
                    if let Some(delegator) = &progress_callback {
                        {
                            let mut user_callback =
                                delegator.data.lock().expect("get cbinding callback");
                            user_callback.progress(0, init_block_height, final_block_height);
                            return true;
                        }
                    }
                    true
                }
                ProgressReport::Update {
                    current_block_height,
                    ..
                } => {
                    if let Some(delegator) = &progress_callback {
                        {
                            let mut user_callback =
                                delegator.data.lock().expect("get cbinding callback");
                            return 1
                                == user_callback.progress(
                                    current_block_height,
                                    init_block_height,
                                    final_block_height,
                                );
                        }
                    }
                    true
                }
            }
        })
        .map_err(to_rpc_error)
}

impl<S, C, O> SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
{
    #[inline]
    fn do_run_sync(&self, request: WalletRequest, reset: bool) -> Result<RunSyncResult> {
        log::info!("run_sync");
        let config = self.config.clone();

        let name = request.name.clone();
        let worker = self.worker.clone();
        let userrequest = request.clone();

        if worker.lock().expect("get sync worker lock").exist(&name) {
            return Err(rpc_error_from_string(
                "wallet already running sync".to_owned(),
            ));
        }

        thread::spawn(move || {
            let tmpworker = worker;

            tmpworker.lock().expect("get sync worker lock").add(&name);
            let node = tmpworker.lock().expect("get sync worker lock").get(&name);
            let usercallback = node.expect("get progress callback");

            let usercallback = Some(CBindingCore { data: usercallback });
            let result = process_sync(config, userrequest, reset, usercallback);
            log::info!("process_sync finished {} {:?}", name, result);
            // notify
            log::info!("wait for notification {}", name);
            std::thread::sleep(std::time::Duration::from_secs(20));
            tmpworker
                .lock()
                .expect("get sync worker lock")
                .remove(&name);
            log::info!("sync thread finished {}", name);
        });

        Ok(RunSyncResult {
            message: "started sync wallet".to_string(),
            name: request.name,
        })
    }
}

impl<S, C, O> SyncRpc for SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
{
    #[inline]
    fn sync(&self, request: WalletRequest) -> Result<()> {
        process_sync(
            self.config.clone(),
            request,
            false,
            self.progress_callback.clone(),
        )
    }

    #[inline]
    fn run_sync_progress(&self, request: WalletRequest) -> Result<RunSyncProgressResult> {
        self.worker
            .lock()
            .expect("get sync worker lock")
            .get_progress(&request.name)
    }

    #[inline]
    fn run_sync(&self, request: WalletRequest) -> Result<RunSyncResult> {
        self.do_run_sync(request, false)
    }

    #[inline]
    fn run_sync_all(&self, request: WalletRequest) -> Result<RunSyncResult> {
        self.do_run_sync(request, true)
    }

    #[inline]
    fn sync_all(&self, request: WalletRequest) -> Result<()> {
        process_sync(
            self.config.clone(),
            request,
            true,
            self.progress_callback.clone(),
        )
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
        progress_callback: Option<CBindingCore>,
    ) -> Self {
        let mut polling_synchronizer = PollingSynchronizer::default();
        polling_synchronizer.spawn(config.clone());

        SyncRpcImpl {
            config,
            polling_synchronizer,
            progress_callback,
            worker: Arc::new(Mutex::new(SyncWorker::new())),
        }
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
