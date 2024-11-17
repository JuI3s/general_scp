use std::{
    collections::{BTreeSet, HashSet},
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
        nomination_protocol::{self, NominationProtocol, NominationProtocolState},
        scp::NodeID,
        scp_driver_builder::SlotDriverBuilder,
        scp_state_builder::NominationProtocolStateBuilder,
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
