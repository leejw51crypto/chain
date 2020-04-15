


use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use jsonrpc_core::Result;

use super::sync_rpc::{CBindingCore,CBindingCallback};
#[derive(Default)]
pub struct SyncWorkerNode {
    pub user_data: u64,
    // 0.0 ~ 100.0
    pub progress: f32,

}
impl SyncWorkerNode {
    fn new()->Self {
        SyncWorkerNode {
            progress:0.0,
            user_data:0,
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
        let mut gap:f32= 0.0;
        let mut rate:f32 =0.0;
        if (end>start) {
            gap= (end- start) as f32;
            rate= ((current-start) as f32)/ gap*100.0;
        }
        self.progress=rate;
        // OK
        0 
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
    pub fn add(&mut self, newthread: &str) {
        self.works
            .insert(newthread.to_string(), Arc::new(Mutex::new(SyncWorkerNode::new())));
        println!("add current={}", self.works.len());
    }
    pub fn remove(&mut self, removethread: &str) {
        self.works.remove(removethread);
        println!("remove current={}", self.works.len());
    }
    pub fn get_progress(&self,key:&str) -> f32 {
        if self.works.contains_key(key) {
            return self.works.get(key).unwrap().lock().unwrap().progress;
        }
        else {
            return 1.0 ;
        }
        
    }
    pub fn exist(&self, thread: &str) -> bool {
        self.works.contains_key(thread)
    }
}

pub type WorkerShared = Arc<Mutex<SyncWorker>>;
