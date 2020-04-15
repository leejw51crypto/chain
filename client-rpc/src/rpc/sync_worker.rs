use super::sync_rpc::CBindingCallback;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

pub struct SyncWorkerNode {
    pub user_data: u64,
    // 0.0 ~ 100.0
    pub progress: f32,
    counter: Instant,
}
impl SyncWorkerNode {
    fn new() -> Self {
        SyncWorkerNode {
            progress: 0.0,
            user_data: 0,
            counter: Instant::now(),
        }
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

        if current == end || self.counter.elapsed().as_millis() > 250 {
            log::info!(
                "sync progress {} percent  {} {}~{}",
                rate,
                current,
                start,
                end
            );
            self.counter = Instant::now();
        } else {
            log::debug!(
                "sync progress {} percent  {} {}~{}",
                rate,
                current,
                start,
                end
            );
        }

        self.progress = rate;

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
            Arc::new(Mutex::new(SyncWorkerNode::new())),
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
    pub fn get_progress(&self, key: &str) -> f32 {
        if self.works.contains_key(key) {
            self.works.get(key).unwrap().lock().unwrap().progress
        } else {
            0.0
        }
    }
    pub fn exist(&self, thread: &str) -> bool {
        self.works.contains_key(thread)
    }
}

pub type WorkerShared = Arc<Mutex<SyncWorker>>;
