use super::sync_rpc::{CBindingCallback, RunSyncProgressResult};
use crate::server::rpc_error_from_string;
use jsonrpc_core::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
pub struct SyncWorkerNode {
    pub user_data: u64,
    pub progress: RunSyncProgressResult,
    counter: Instant,
}
impl SyncWorkerNode {
    fn new(name: &str) -> Self {
        let mut ret = SyncWorkerNode {
            progress: RunSyncProgressResult::default(),
            user_data: 0,
            counter: Instant::now(),
        };
        ret.progress.name = name.to_string();
        ret
    }
}

impl CBindingCallback for SyncWorkerNode {
    fn set_user(&mut self, user: u64) {
        self.user_data = user;
    }

    fn get_user(&self) -> u64 {
        self.user_data
    }

    fn progress(&mut self, current: u64, start: u64, end: u64) -> i32 {
        let rate = if current >= start && end > start {
            let gap: f32 = (end - start) as f32;
            ((current - start) as f32) / gap * 100.0
        } else {
            0.0
        };

        let status: String;
        if current == end || self.counter.elapsed().as_millis() > 250 {
            status = format!(
                "sync progress {} percent  {} {}~{}",
                rate, current, start, end
            );
            log::info!("{}", status);
            self.counter = Instant::now();
        } else {
            status = format!(
                "sync progress {} percent  {} {}~{}",
                rate, current, start, end
            );
            log::debug!("{}", status);
        }

        self.progress.percent = rate;
        self.progress.current = current;
        self.progress.start = start;
        self.progress.end = end;
        self.progress.message = status;

        // OK
        1
    }
}
pub type NodeShared = Arc<Mutex<SyncWorkerNode>>;

#[derive(Default)]
pub struct SyncWorker {
    //  works: Vec<JoinHandle<()>>,
    works: HashMap<String, NodeShared>,
}

impl SyncWorker {
    pub fn new() -> Self {
        SyncWorker {
            works: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<NodeShared> {
        if let Some(value) = self.works.get(name) {
            Some(value.clone())
        } else {
            None
        }
    }
    pub fn add(&mut self, newthread: &str) {
        self.works.insert(
            newthread.to_string(),
            Arc::new(Mutex::new(SyncWorkerNode::new(newthread))),
        );
        log::info!("add sync thread {} total {}", newthread, self.works.len());
    }
    pub fn remove(&mut self, removethread: &str) {
        self.works.remove(removethread);
        log::info!(
            "remove sync thread {} total {}",
            removethread,
            self.works.len()
        );
    }
    pub fn get_progress(&self, key: &str) -> Result<RunSyncProgressResult> {
        if let Some(value) = self.works.get(key) {
            Ok(value.lock().unwrap().progress.clone())
        } else {
            Err(rpc_error_from_string(
                "wallet is not running sync".to_owned(),
            ))
        }
    }
    pub fn exist(&self, thread: &str) -> bool {
        self.works.contains_key(thread)
    }
}

pub type WorkerShared = Arc<Mutex<SyncWorker>>;
