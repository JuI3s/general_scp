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
        quorum::{Quorum, QuorumSlice},
    },
    overlay::rpc_gateway::TestRpcGateway,
};

#[tokio::main]
async fn main() {
    // let arg = Cli::parse();
    // println!("{0}", arg.pattern);

    let ip_addr = Ipv4Addr::new(127, 0, 0, 1);
    let sock = SocketAddrV4::new(ip_addr, 17);
    let sock2 = SocketAddrV4::new(ip_addr, 18);

    let mut qset1 = QuorumSlice::new();
    let mut qset2 = QuorumSlice::new();
    let mut q = Quorum::new();
    qset1.insert(sock.clone());
    qset1.insert(sock2.clone());

    qset2.insert(sock.clone());
    q.insert(qset1.clone());
    q.insert(qset2.clone());

    println!("{:?}", qset1);
    println!("{:?}", qset2);
    println!("{:?}", q);

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
