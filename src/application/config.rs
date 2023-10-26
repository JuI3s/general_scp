use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    overlay::{
        peer::PeerID,
        rpc_gateway::{self, HRpcGateway, RpcGateway, TestRpcGateway},
    },
    rpc,
};

use super::clock::{HVirtualClock, VirtualClock};

#[derive(Clone)]
pub struct Config {
    pub clock: HVirtualClock,
    pub rpc_gateway: HRpcGateway,
    pub peer_id: PeerID,
    pub clear_work_queue_duration: Duration,
}

impl Config {
    pub fn new(
        clock: HVirtualClock,
        rpc_gateway: HRpcGateway,
        peer_id: PeerID,
        clear_work_queue_duration: Duration,
    ) -> Self {
        Config {
            peer_id: peer_id,
            clock: clock.clone(),
            rpc_gateway: rpc_gateway,
            clear_work_queue_duration: clear_work_queue_duration,
        }
    }

    pub fn make_test_configt(rpc_gateway: Option<HRpcGateway>) -> Self {
        let gateway = if let Some(_gateway) = rpc_gateway {
            _gateway
        } else {
            TestRpcGateway::new_test_rpc_gateway()
        };

        let clock = VirtualClock::new_clock();
        Config {
            peer_id: "local_node",
            clock: clock,
            rpc_gateway: gateway,
            clear_work_queue_duration: Duration::from_secs(1),
        }
    }
}
