use std::sync::{Arc, Condvar, Mutex};

use super::{
    clock,
    config::Config,
    work_queue::{self, WorkQueue},
};

pub struct Application {
    main_thread_work_queue: Arc<Mutex<WorkQueue>>,
}

impl Application {
    pub fn new(cfg: &Config) -> Self {
        let work_queue = Arc::new(Mutex::new(WorkQueue::new(cfg.clock.clone())));

        Application {
            main_thread_work_queue: work_queue,
        }
    }
}
