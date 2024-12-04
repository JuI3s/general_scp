use std::{
    collections::BTreeSet, iter,
};

use general_scp::{
    application::quorum::{make_quorum_node_for_test, QuorumSet, QuorumSlice},
    mock::state::MockState,
    scp::local_node::{LocalNodeInfo, LocalNodeInfoBuilderFromFile},
};


// #[tokio::main]
fn main() {
    // start_local_node_server();

    let node1 = make_quorum_node_for_test(1);
    let node2 = make_quorum_node_for_test(2);

    node1.write_toml();
    node2.write_toml();

    let quorum_slice = QuorumSlice {
        data: BTreeSet::from_iter(vec![node1.clone(), node2.clone()]),
    };
    let quorum_set = QuorumSet {
        slices: BTreeSet::from_iter(iter::once(quorum_slice)),
        threshold: 0,
    };

    let local_node_info1 =
        LocalNodeInfo::<MockState>::new(true, quorum_set.clone(), node1.node_id.clone());

    let local_node_info2 =
        LocalNodeInfo::<MockState>::new(true, quorum_set.clone(), node2.node_id.clone());

    local_node_info1.write_toml("test");
    local_node_info2.write_toml("test");

    let mut local_node_info_builder = LocalNodeInfoBuilderFromFile::new("test");

    let local_node_info1_from_file = local_node_info_builder
        .build_from_file::<MockState>(&node1.node_id)
        .unwrap();
    let local_node_info2_from_file = local_node_info_builder
        .build_from_file::<MockState>(&node2.node_id)
        .unwrap();

    assert!(local_node_info1 == local_node_info1_from_file);
    assert!(local_node_info2 == local_node_info2_from_file);

    // let local_node_info = LocalNodeInfo::new(false, quorum_set, node_id)

    // let mut rng = rand::thread_rng();
    // let components = Components::generate(&mut rng, KeySize::DSA_2048_256);
    // let signing_key = SigningKey::generate(&mut rng, components);
    // let verifying_key = signing_key.verifying_key();

    // let signature = signing_key.sign_digest_with_rng(
    //     &mut rand::thread_rng(),
    //     Sha1::new().chain_update(b"hello world"),
    // );

    // let signing_key_bytes =
    // signing_key.to_pkcs8_pem(LineEnding::LF).unwrap();
    // let verifying_key_bytes =
    // verifying_key.to_public_key_pem(LineEnding::LF).unwrap();

    // let mut file = File::create("public.pem").unwrap();
    // file.write_all(verifying_key_bytes.as_bytes()).unwrap();
    // file.flush().unwrap();

    // let mut file = File::create("signature.der").unwrap();
    // file.write_all(&signature.to_bytes()).unwrap();
    // file.flush().unwrap();

    // let mut file = File::create("private.pem").unwrap();
    // file.write_all(signing_key_bytes.as_bytes()).unwrap();
    // file.flush().unwrap();

    // return;
    // let arg = Cli::parse();
    // println!("{0}", arg.pattern);

    // let cfg = Config::new_test_config();
    // println!("{:?}", cfg.quorum_set);

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
    // work_queue.execute_task();
}
