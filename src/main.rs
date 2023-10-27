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
        app_config::AppConfig, application::Application, command_line::Cli, config::Config,
        quorum::QuorumSet,
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

    println!("{}", sock.to_string());
    let mut quorum_set1 = HashSet::new();
    quorum_set1.insert(sock.to_string());
    let mut quorum_set2 = HashSet::new();
    quorum_set2.insert(sock.to_string());
    quorum_set2.insert(sock2.to_string());
    // let mut quorum = HashSet::new();
    // quorum.insert(Box::new(quorum_set1));
    // quorum.insert(Box::new(quorum_set2));
    let data_str = "{\"127.0.0.1:17\"}";
    println!("{:?}", quorum_set1);

    let rpc_gateway: std::sync::Arc<std::sync::Mutex<TestRpcGateway>> =
        TestRpcGateway::new_test_rpc_gateway();

    // let cfg = AppConfig::make_test_config(Some(rpc_gateway.clone()));

    // let cfg = Config::new();

    // let toml = toml::to_string(&cfg).unwrap();
    // println!("{}", toml);
    //
    // fs::write("config.toml", toml).expect("Could not write to file!");
    //
    let filename = format!("config.toml");
    let cfg = Config::from_toml_file(&filename);
    println!("{:?}", cfg);

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
