use std::time::Duration;

use serde::Deserialize;

use crate::overlay::{
    peer::PeerID,
    rpc_gateway::{HRpcGateway, TestRpcGateway},
};

use super::clock::{HVirtualClock, VirtualClock};

#[derive(Clone)]
pub struct AppConfig {
    pub clock: HVirtualClock,
    pub rpc_gateway: HRpcGateway,
    pub peer_id: PeerID,
    pub clear_work_queue_duration: Duration,
}

impl AppConfig {
    pub fn new(
        clock: HVirtualClock,
        rpc_gateway: HRpcGateway,
        peer_id: PeerID,
        clear_work_queue_duration: Duration,
    ) -> Self {
        AppConfig {
            peer_id: peer_id,
            clock: clock.clone(),
            rpc_gateway: rpc_gateway,
            clear_work_queue_duration: clear_work_queue_duration,
        }
    }

    pub fn make_test_config(rpc_gateway: Option<HRpcGateway>) -> Self {
        let gateway = if let Some(_gateway) = rpc_gateway {
            _gateway
        } else {
            TestRpcGateway::new_test_rpc_gateway()
        };

        let clock = VirtualClock::new_clock();
        AppConfig {
            peer_id: "local_node".to_owned(),
            clock: clock,
            rpc_gateway: gateway,
            clear_work_queue_duration: Duration::from_secs(1),
        }
    }

    pub fn from_config_file() -> Self {
        todo!();
    }
}
