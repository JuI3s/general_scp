use std::{
    collections::{btree_set, BTreeSet},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use serde::{Deserialize, Serialize};

// pub type QuorumSet = HashSet<SocketAddr>;
// pub type Quorum = HashSet<QuorumSet>;

pub type QuorumNode = SocketAddrV4;

// Set of quorum slices for local node.
#[derive(Debug, Serialize, Deserialize)]
pub struct QuorumSet {
    slices: BTreeSet<QuorumSlice>,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumSlice {
    data: BTreeSet<QuorumNode>,
}

impl QuorumSet {
    pub fn new() -> Self {
        QuorumSet {
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

impl<const N: usize> From<[QuorumSlice; N]> for QuorumSet {
    fn from(slices: [QuorumSlice; N]) -> Self {
        QuorumSet {
            slices: BTreeSet::from(slices),
        }
    }
}

impl<const N: usize> From<[QuorumNode; N]> for QuorumSlice {
    fn from(arr: [QuorumNode; N]) -> Self {
        QuorumSlice {
            data: BTreeSet::from(arr),
        }
    }
}
