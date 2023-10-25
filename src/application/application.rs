use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex, mpsc},
};


use tokio::{sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel}, select};

use crate::{overlay::peer::{HPeer, PeerID}, rpc::args::RpcArg};

use super::{
    clock,
    config::Config,
    work_queue::{WorkQueue},
};

type PendingRequestQueue = UnboundedReceiver<RpcArg>;
type RpcRequestWriteQueue = Arc<Mutex<UnboundedSender<RpcArg>>>;

pub struct Application {
    main_thread_work_queue: Arc<Mutex<WorkQueue>>,
    peers: HashMap<PeerID, HPeer>,
    pending_requests: PendingRequestQueue,
    request_write_queue: RpcRequestWriteQueue,  
}

impl Application {
    pub fn new(cfg: &Config) -> Self {
        let work_queue = Arc::new(Mutex::new(WorkQueue::new(cfg.clock.clone())));

        let (tx, rx) = unbounded_channel::<RpcArg>();
        
        Application {
            main_thread_work_queue: work_queue,
            peers: HashMap::new(),
            pending_requests: rx,
            request_write_queue: Arc::new(Mutex::new(tx))
        }
    }

    pub async fn start(&mut self) {
        print!("Application running...");
        loop {
            select! {
                rpc_call = self.pending_requests.recv() => {
                    unimplemented!();
                },
                else => {
                    self.main_thread_work_queue.lock().unwrap().execute_task();
                }
            }
        }
    }
}
