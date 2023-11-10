#[macro_use]
extern crate weak_self_derive;

use std::{
    collections::HashSet,
    fs,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    thread,
    time::Duration,
};

use clap::Parser;
use rust_example::{
    application::{
        app_config::AppConfig,
        application::Application,
        command_line::Cli,
        config::Config,
        quorum::{QuorumSet, QuorumSlice},
    },
    overlay::rpc_gateway::TestRpcGateway,
};

#[tokio::main]
async fn main() {
    // let arg = Cli::parse();
    // println!("{0}", arg.pattern);

    let cfg = Config::new_test_config();
    println!("{:?}", cfg.quorum_set);

    // let mut app = Application::new(cfg.clone());

    // let handle = tokio::spawn(async move {
    //     loop {
    //         thread::sleep(Duration::from_secs(1));
    //         rpc_gateway.lock().unwrap().send_hello_message(cfg.peer_id);
    //         rpc_gateway.lock().unwrap().send_hello_message(cfg.peer_id);
    //     }
    // });

    // app.start().await;

    // let _ = handle.await;
    // let mut work_queue = WorkQueue::new();
    // let mut peer = Peer::new();
    // peer.incr_one();
    // peer.add_to_queue(&mut work_queue);
    // work_queue.execute_task();
}
