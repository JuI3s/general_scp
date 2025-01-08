use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};

use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::interval,
};

use crate::{
    mock::state::MockStateDriverBuilder,
    overlay::peer::{HPeer, PeerID},
    overlay_impl::tcp_peer::TCPPeerBuilder,
    rpc::args::RpcArg,
    scp::local_node::LocalNodeInfo,
};

use super::{app_config::AppConfig, command::SCPCommand, work_queue::EventQueue};

pub type PendingRequestQueue = UnboundedReceiver<RpcArg>;
pub type RpcRequestWriteQueue = Arc<Mutex<UnboundedSender<RpcArg>>>;

pub fn start_local_node_server() {
    let herder_builder = MockStateDriverBuilder::new();
    let mut tcp_peer_builder = TCPPeerBuilder::new(herder_builder);
    let node_info = LocalNodeInfo::new(false, Default::default(), "node1".to_string());

    let tcp_peer = tcp_peer_builder.build_node(node_info);
    let mut input = String::new();

    loop {
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                if let Some(cmd) = SCPCommand::parse(&input) {
                    match cmd {
                        SCPCommand::Nominate => {
                            tcp_peer.borrow_mut().slot_nominate_with_default_val(0);
                        }
                        SCPCommand::Hello => {}
                    }
                    println!("{:?}", cmd);
                } else {
                    println!("Invalid command.");
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

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
            .register(config.peer_id.clone(), rpc_write_queue);

        Application {
            local_node_id: config.peer_id.clone(),
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
