use std::{
    collections::{btree_set, hash_map::DefaultHasher, BTreeSet},
    f32::consts::E,
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use serde::{Deserialize, Serialize};

use crate::scp::{scp::NodeID, scp_driver::HashValue};

// pub type QuorumSet = HashSet<SocketAddr>;
// pub type Quorum = HashSet<QuorumSet>;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumNode {
    pub node_id: NodeID,
    pub addr: SocketAddrV4,
}

// Set of quorum slices for local node.
#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct QuorumSet {
    pub slices: BTreeSet<QuorumSlice>,
    pub threshold: usize,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumSlice {
    pub data: BTreeSet<QuorumNode>,
}

impl QuorumSlice {
    pub fn hash_value(&self) -> HashValue {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}

impl QuorumSet {
    pub fn new(threshold: usize) -> Self {
        QuorumSet {
            slices: BTreeSet::new(),
            threshold: threshold,
        }
    }

    pub fn hash_value(&self) -> HashValue {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    pub fn insert(&mut self, slice: QuorumSlice) {
        self.slices.insert(slice);
    }
}

impl Default for QuorumSet {
    fn default() -> Self {
        Self {
            slices: Default::default(),
            threshold: Default::default(),
        }
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

// TODO:
impl<const N: usize> From<[QuorumSlice; N]> for QuorumSet {
    fn from(slices: [QuorumSlice; N]) -> Self {
        QuorumSet {
            slices: BTreeSet::from(slices),
            threshold: 0,
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
