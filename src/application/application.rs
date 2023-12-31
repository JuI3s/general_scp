use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::interval,
};

use crate::{
    overlay::peer::{HPeer, PeerID},
    rpc::args::RpcArg,
};

use super::{app_config::AppConfig, work_queue::EventQueue};

pub type PendingRequestQueue = UnboundedReceiver<RpcArg>;
pub type RpcRequestWriteQueue = Arc<Mutex<UnboundedSender<RpcArg>>>;

pub struct Application {
    local_node_id: PeerID,
    main_thread_work_queue: Arc<Mutex<EventQueue>>,
    peers: HashMap<PeerID, HPeer>,
    pending_requests: PendingRequestQueue,
    config: AppConfig,
}

impl Application {
    pub fn new(config: AppConfig) -> Self {
        let work_queue = Arc::new(Mutex::new(EventQueue::new(config.clock.clone())));

        let (tx, rx) = unbounded_channel::<RpcArg>();
        let rpc_write_queue = Arc::new(Mutex::new(tx));

        // Register rpc gateway.
        config
            .rpc_gateway
            .lock()
            .unwrap()
            .register(config.peer_id, rpc_write_queue);

        Application {
            local_node_id: config.peer_id,
            main_thread_work_queue: work_queue,
            peers: HashMap::new(),
            pending_requests: rx,
            config: config,
        }
    }

    pub async fn start(&mut self) {
        print!("Application running...\n");

        let mut execute_main_work_interval = interval(self.config.clear_work_queue_duration);

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
                _ = execute_main_work_interval.tick() => {
                    print!("Empty work queue\n");
                    self.main_thread_work_queue.lock().unwrap().execute_task();
                },
            }
        }
    }

    fn handle_rpc_call(&mut self, arg: &RpcArg) {
        print!("Handling an rpc arg\n");
    }
}
