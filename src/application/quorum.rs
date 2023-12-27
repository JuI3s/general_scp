use std::{
    collections::{btree_set, hash_map::DefaultHasher, BTreeSet},
    f32::consts::E,
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::scp::{scp::NodeID, scp_driver::HashValue};

// pub type QuorumSet = HashSet<SocketAddr>;
// pub type Quorum = HashSet<QuorumSet>;

pub type HQuorumSet = Arc<Mutex<QuorumSet>>;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumNode {
    pub node_id: NodeID,
    pub addr: SocketAddrV4,
}

// Set of quorum slices for local node.
#[derive(Debug, Serialize, Deserialize, Hash, Clone)]
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

    pub fn nodes(&self) -> BTreeSet<NodeID> {
        // This method returns a set of all the node_ids in the quorum set.
        let mut nodes = BTreeSet::default();
        for slice in &self.slices {
            for node in &slice.data {
                nodes.insert(node.node_id.to_owned());
            }
        }

        nodes
    }

    pub fn example_quorum_set() -> Self {
        let sock1 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
        let sock2 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081);
        let sock3 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8082);

        let node_id1 = "node1";
        let node_id2 = "node2";
        let node_id3 = "node3";

        let node1 = QuorumNode {
            node_id: node_id1.into(),
            addr: sock1,
        };
        let node2 = QuorumNode {
            node_id: node_id2.into(),
            addr: sock2,
        };
        let node3 = QuorumNode {
            node_id: node_id3.into(),
            addr: sock3,
        };

        let quorum_slice1 = QuorumSlice::from([node1.to_owned(), node2.to_owned()]);
        let quorum_slice2 = QuorumSlice::from([node1.to_owned(), node3.to_owned()]);
        assert_eq!(quorum_slice1.data.len(), 2);
        assert_eq!(quorum_slice2.data.len(), 2);

        let quorum_set = QuorumSet::from([quorum_slice1, quorum_slice2]);
        quorum_set
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_quorum_set() {
        let sock1 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
        let sock2 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081);
        let sock3 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8082);

        let node_id1 = "node1";
        let node_id2 = "node2";
        let node_id3 = "node3";

        let node1 = QuorumNode {
            node_id: node_id1.into(),
            addr: sock1,
        };
        let node2 = QuorumNode {
            node_id: node_id2.into(),
            addr: sock2,
        };
        let node3 = QuorumNode {
            node_id: node_id3.into(),
            addr: sock3,
        };

        let quorum_slice1 = QuorumSlice::from([node1.to_owned(), node2.to_owned()]);
        let quorum_slice2 = QuorumSlice::from([node1.to_owned(), node3.to_owned()]);
        assert_eq!(quorum_slice1.data.len(), 2);
        assert_eq!(quorum_slice2.data.len(), 2);

        let quorum_set = QuorumSet::from([quorum_slice1, quorum_slice2]);
        assert_eq!(quorum_set.slices.len(), 2);
    }
}
