use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Condvar, Mutex},
};

use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

use crate::{
    overlay::peer::{HPeer, PeerID},
    rpc::args::RpcArg,
};

use super::{clock, config::Config, work_queue::WorkQueue};

pub type PendingRequestQueue = UnboundedReceiver<RpcArg>;
pub type RpcRequestWriteQueue = Arc<Mutex<UnboundedSender<RpcArg>>>;

pub struct Application {
    local_node_id: PeerID,
    main_thread_work_queue: Arc<Mutex<WorkQueue>>,
    peers: HashMap<PeerID, HPeer>,
    pending_requests: PendingRequestQueue,
}

impl Application {
    pub fn new(cfg: &Config) -> Self {
        let work_queue = Arc::new(Mutex::new(WorkQueue::new(cfg.clock.clone())));

        let (tx, rx) = unbounded_channel::<RpcArg>();
        let rpc_write_queue = Arc::new(Mutex::new(tx));
        
        // Register rpc gateway.
        cfg.rpc_gateway.lock().unwrap().register(cfg.peer_id, rpc_write_queue);
        
        Application {
            local_node_id: cfg.peer_id, 
            main_thread_work_queue: work_queue,
            peers: HashMap::new(),
            pending_requests: rx,
        }
    }

    pub async fn start(&mut self) {
        print!("Application running...");
        loop {
            select! {
                rpc_call = self.pending_requests.recv() => {
                    match rpc_call { 
                        Some(arg) => {
                            self.handle_rpc_call(&arg);
                        },
                        None => {},
                    }
                },
                else => {
                    self.main_thread_work_queue.lock().unwrap().execute_task();
                }
            }
        }
    }
    
    fn handle_rpc_call(&mut self, arg: &RpcArg) {
        print!("Handling an rpc arg");
    }
}
