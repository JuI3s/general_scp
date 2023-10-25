use std::{collections::HashMap, io::StderrLock};

use crate::application::application::RpcRequestWriteQueue;

use super::peer::PeerID;

// When an application is constructed, it needs to register with an RpcGateway struct to receive rpc calls.
pub trait RpcGateway { 
    fn register(&mut self, peer_id: PeerID, write_queue: RpcRequestWriteQueue);
    fn remove(&mut self, peer_id: PeerID);
}

pub struct TestRpcGateway {
    write_queues: HashMap<PeerID, RpcRequestWriteQueue>,
}

impl TestRpcGateway {
    pub fn new() -> Self {
        TestRpcGateway { write_queues: HashMap::new() }
    }
}

impl RpcGateway for TestRpcGateway {
    fn register(&mut self, peer_id: PeerID, write_queue: RpcRequestWriteQueue) {
        self.write_queues.insert(peer_id, write_queue);
    }

    fn remove(&mut self, peer_id: PeerID) {
        self.write_queues.remove(peer_id);
    }
}
