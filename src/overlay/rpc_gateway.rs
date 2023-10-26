use std::{
    collections::HashMap,
    hash::Hash,
    io::StderrLock,
    sync::{Arc, Mutex},
};

use crate::{application::application::RpcRequestWriteQueue, rpc::args::RpcArg};

use super::peer::{Peer, PeerID};

pub type HRpcGateway = Arc<Mutex<dyn RpcGateway>>;
// When an application is constructed, it needs to register with an RpcGateway struct to receive rpc calls.
pub trait RpcGateway {
    fn register(&mut self, peer_id: PeerID, write_queue: RpcRequestWriteQueue);
    fn remove(&mut self, peer_id: PeerID);
    fn listen(&mut self);
}

// Used for testing.
pub struct TestRpcGateway {
    write_queues: HashMap<PeerID, RpcRequestWriteQueue>,
}

impl TestRpcGateway {
    pub fn new() -> Self {
        let mut ret = TestRpcGateway {
            write_queues: HashMap::new(),
        };
        ret.listen();
        ret
    }

    pub fn new_test_rpc_gateway() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(TestRpcGateway {
            write_queues: HashMap::new(),
        }))
    }

    pub fn send_hello_message(&mut self, peer_id: PeerID) {
        // Used for testing
        print!("Rpc gateway sending a hello message for testing\n");

        match self.write_queues.get(peer_id) {
            Some(peer_work_queue) => {
                let hello_msg = RpcArg::example_arg();
                let _ = peer_work_queue.lock().unwrap().send(hello_msg);
            }
            None => {}
        }
    }
}

impl RpcGateway for TestRpcGateway {
    fn register(&mut self, peer_id: PeerID, write_queue: RpcRequestWriteQueue) {
        self.write_queues.insert(peer_id, write_queue);
    }

    fn remove(&mut self, peer_id: PeerID) {
        self.write_queues.remove(peer_id);
    }

    fn listen(&mut self) {}
}
