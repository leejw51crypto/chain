


use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use jsonrpc_core::Result;


#[derive(Default)]
struct SyncWorkerNode {
    pub progress: f32,

}
impl SyncWorkerNode {
    fn new()->Self {
        SyncWorkerNode {
            progress:0.0,

        }
    }
}


#[derive(Default)]
pub struct SyncWorker {
    //  works: Vec<JoinHandle<()>>,
    works: HashMap<String, SyncWorkerNode>,
}

impl SyncWorker {
    pub fn new() -> Self {
        SyncWorker {
            works: HashMap::new(),
        }
    }
    pub fn add(&mut self, newthread: &str) {
        self.works
            .insert(newthread.to_string(), SyncWorkerNode::new());
        println!("add current={}", self.works.len());
    }
    pub fn remove(&mut self, removethread: &str) {
        self.works.remove(removethread);
        println!("remove current={}", self.works.len());
    }
    pub fn get_progress(&self,key:&str) -> f32 {
        if self.works.contains_key(key) {
            return self.works.get(key).unwrap().progress;
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
