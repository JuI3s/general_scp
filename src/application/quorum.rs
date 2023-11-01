use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

// pub type QuorumSet = HashSet<SocketAddr>;
// pub type Quorum = HashSet<QuorumSet>;

pub type QuorumNode = SocketAddrV4;

#[derive(Debug)]
pub struct Quorum {
    slices: BTreeSet<QuorumSlice>,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone)]
pub struct QuorumSlice {
    data: BTreeSet<QuorumNode>,
}

impl Quorum {
    pub fn new() -> Self {
        Quorum {
            slices: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, slice: QuorumSlice) {
        self.slices.insert(slice);
    }
}

impl QuorumSlice {
    pub fn new() -> Self {
        QuorumSlice {
            data: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, sock: QuorumNode) {
        self.data.insert(sock);
    }
}
