use std::sync::{Arc, Mutex};

use crate::{
    overlay::{rpc_gateway::{self, RpcGateway, TestRpcGateway, HRpcGateway}, peer::PeerID},
    rpc,
};

use super::clock::{HVirtualClock, VirtualClock};

pub struct Config {
    pub clock: HVirtualClock,
    pub rpc_gateway: HRpcGateway,
    pub peer_id: PeerID, 
}

impl Config {
    pub fn new(clock: HVirtualClock, rpc_gateway: HRpcGateway, peer_id: PeerID) -> Self {
        Config {
            peer_id: peer_id, 
            clock: clock.clone(),
            rpc_gateway: rpc_gateway,
        }
    }

    pub fn new_config() -> Self {
        let clock = VirtualClock::new_clock();
        Config {
            peer_id: "local_node",
            clock: clock,
            rpc_gateway: TestRpcGateway::new_test_rpc_gateway(),
        }
    }
}
