use std::{
    collections::{HashSet, BTreeSet},
    fs,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    thread,
    time::Duration,
};

use clap::Parser;
use general_scp::{
    application::{
        app_config::AppConfig,
        application::Application,
        clock::VirtualClock,
        command_line::Cli,
        config::Config,
        quorum::{QuorumSet, QuorumSlice},
    },
    mock::state::{MockState, MockStateDriver},
    overlay::rpc_gateway::TestRpcGateway,
    scp::{
        local_node_builder::LocalNodeBuilder,
        nomination_protocol::{NominationProtocol, NominationProtocolState, self},
        scp::NodeID,
        scp_driver_builder::{SlotDriverBuilder, SlotTimerBuilder}, scp_state_builder::NominationProtocolStateBuilder,
    },
};

use digest::Digest;
use dsa::{Components, KeySize, SigningKey};
use pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding, PrivateKeyInfo};
use sha1::Sha1;
use signature::{RandomizedDigestSigner, SignatureEncoding};
use std::{fs::File, io::Write};

// #[tokio::main]
fn main() {
    // let mut rng = rand::thread_rng();
    // let components = Components::generate(&mut rng, KeySize::DSA_2048_256);
    // let signing_key = SigningKey::generate(&mut rng, components);
    // let verifying_key = signing_key.verifying_key();

    // let signature = signing_key.sign_digest_with_rng(
    //     &mut rand::thread_rng(),
    //     Sha1::new().chain_update(b"hello world"),
    // );

    // let signing_key_bytes = signing_key.to_pkcs8_pem(LineEnding::LF).unwrap();
    // let verifying_key_bytes = verifying_key.to_public_key_pem(LineEnding::LF).unwrap();

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

    let node_id: NodeID = "node1".into();
    let virtual_clock = VirtualClock::new_clock();

    let mut leaders: BTreeSet<NodeID> = BTreeSet::new();
    leaders.insert(node_id.clone());


    let timer_handle = SlotTimerBuilder::new()
        .clock(virtual_clock.clone())
        .build()
        .unwrap();

    let quorum_set = QuorumSet::example_quorum_set();

    let local_node = LocalNodeBuilder::<MockState>::new()
        .is_validator(true)
        .quorum_set(quorum_set)
        .node_id(node_id)
        .build()
        .unwrap();

    let nomination_protocol_state = NominationProtocolStateBuilder::<MockState>::new().round_leaders(leaders).build();
    
    let slot_driver = SlotDriverBuilder::<MockState, MockStateDriver>::new()
        .slot_index(0)
        .herder_driver(Default::default())
        .timer(timer_handle)
        .local_node(local_node)
        .nomination_protocol_state(nomination_protocol_state)
        .build()
        .unwrap();

    let value = Arc::new(MockState::random());
    let prev_value = MockState::random();

    println!("Nominating...");
    slot_driver.nominate(slot_driver.nomination_state(), value, &prev_value);
}
