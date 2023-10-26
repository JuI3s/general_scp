use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use rust_example::{
    application::{application::Application, config::Config, work_queue::WorkQueue},
    overlay::{
        peer::Peer,
        rpc_gateway::{self, TestRpcGateway},
    },
};
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    let rpc_gateway = TestRpcGateway::new_test_rpc_gateway();

    let cfg = Config::make_test_configt(Some(rpc_gateway.clone()));
    let mut app = Application::new(cfg.clone());

    let handle = tokio::spawn(async move {
        loop {
            thread::sleep(Duration::from_secs(1));
            rpc_gateway.lock().unwrap().send_hello_message(cfg.peer_id);
            rpc_gateway.lock().unwrap().send_hello_message(cfg.peer_id);    
        }
    });

    app.start().await;

    let _ = handle.await;

    // let mut work_queue = WorkQueue::new();
    // let mut peer = Peer::new();
    // peer.incr_one();
    // peer.add_to_queue(&mut work_queue);
    // work_queue.execute_task();
}
