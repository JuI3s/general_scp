use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream},
};

use super::peer::PeerID;

pub trait Outgate {
    fn add_peer(peer_id: PeerID, addr: SocketAddr);
    fn drop_peer(peer_id: PeerID);
}

pub struct TestOutgate {
    connections: HashMap<PeerID, TcpStream>,
}

impl TestOutgate {
    pub fn new() -> Self {
        TestOutgate {
            connections: HashMap::new(),
        }
    }
}

impl Outgate for TestOutgate {
    fn add_peer(peer_id: PeerID, addr: SocketAddr) {
        todo!()
    }

    fn drop_peer(peer_id: PeerID) {
        todo!()
    }
}
