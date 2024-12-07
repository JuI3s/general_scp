use std::{
    collections::{btree_set, hash_map::DefaultHasher, BTreeSet},
    f32::consts::E,
    fs::{self, create_dir},
    hash::{Hash, Hasher},
    io::Write,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use blake2::{Blake2b512, Blake2s256, Digest};

use serde::{Deserialize, Serialize};

use crate::{
    crypto::types::{Blake2Hash, Blake2Hashable},
    overlay::peer::PeerID,
    scp::{scp::NodeID, scp_driver::HashValue},
    utils::config::test_data_dir,
};

// pub type QuorumSet = HashSet<SocketAddr>;
// pub type Quorum = HashSet<QuorumSet>;
pub type QuorumSetHash = Blake2Hash;
pub type HQuorumSet = Arc<Mutex<QuorumSet>>;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumNode {
    pub node_id: NodeID,
    pub ip_addr: Option<SocketAddrV4>,
}

impl From<PeerID> for QuorumNode {
    fn from(value: PeerID) -> Self {
        Self {
            node_id: value,
            ip_addr: None,
        }
    }
}

const BASE_PORT: u16 = 8080;
pub fn make_quorum_node_for_test(node_idx: u16) -> QuorumNode {
    let node_id = format!("node{}", node_idx);

    let ip_addr = Some(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        BASE_PORT + node_idx,
    ));

    QuorumNode { node_id, ip_addr }
}

impl QuorumNode {
    const TEST_DATA_DIR: &'static str = "quorum_node";

    pub fn new(node_id: NodeID, ip_addr: Option<SocketAddrV4>) -> QuorumNode {
        QuorumNode { node_id, ip_addr }
    }

    pub fn write_toml(&self) {
        let path = test_data_dir()
            .join(Self::TEST_DATA_DIR)
            .join(self.node_id.clone());
        let _ = create_dir(path.parent().unwrap());

        let toml = toml::to_string(self).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(toml.as_bytes()).unwrap();
    }

    pub fn from_toml(node_id: &NodeID) -> Option<Self> {
        let path = test_data_dir().join(Self::TEST_DATA_DIR).join(node_id);

        let toml_str = fs::read_to_string(path).ok()?;
        let node = toml::from_str(&toml_str).ok()?;
        node
    }
}

// Set of quorum slices for local node.
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QuorumSet {
    pub slices: BTreeSet<QuorumSlice>,
    pub threshold: usize,
}

impl Blake2Hashable for QuorumSet {
    fn to_blake2(&self) -> crate::crypto::types::Blake2Hash {
        let mut hasher = blake2::Blake2b512::new();
        let encoded: Vec<u8> = bincode::serialize(&self).unwrap();
        hasher.update(encoded);
        hasher.finalize().into()
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Deserialize, Serialize)]
pub struct QuorumSlice {
    pub data: BTreeSet<QuorumNode>,
}

impl QuorumSlice {
    pub fn hash_value(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}

impl QuorumSet {
    pub fn new(threshold: usize) -> Self {
        QuorumSet {
            slices: BTreeSet::new(),
            threshold,
        }
    }

    pub fn nodes(&self) -> BTreeSet<QuorumNode> {
        // This method returns a set of all the node_ids in the quorum set.
        let mut nodes = BTreeSet::default();
        for slice in &self.slices {
            for node in &slice.data {
                nodes.insert(node.to_owned());
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
            ip_addr: Some(sock1),
        };
        let node2 = QuorumNode {
            node_id: node_id2.into(),
            ip_addr: Some(sock2),
        };
        let node3 = QuorumNode {
            node_id: node_id3.into(),
            ip_addr: Some(sock3),
        };

        let quorum_slice1 = QuorumSlice::from([node1.to_owned(), node2.to_owned()]);
        let quorum_slice2 = QuorumSlice::from([node1.to_owned(), node3.to_owned()]);
        assert_eq!(quorum_slice1.data.len(), 2);
        assert_eq!(quorum_slice2.data.len(), 2);

        let quorum_set = QuorumSet::from([quorum_slice1, quorum_slice2]);
        quorum_set
    }

    pub fn hash_value(&self) -> HashValue {
        self.to_blake2()
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
            ip_addr: Some(sock1),
        };
        let node2 = QuorumNode {
            node_id: node_id2.into(),
            ip_addr: Some(sock2),
        };
        let node3 = QuorumNode {
            node_id: node_id3.into(),
            ip_addr: Some(sock3),
        };

        let quorum_slice1 = QuorumSlice::from([node1.to_owned(), node2.to_owned()]);
        let quorum_slice2 = QuorumSlice::from([node1.to_owned(), node3.to_owned()]);
        assert_eq!(quorum_slice1.data.len(), 2);
        assert_eq!(quorum_slice2.data.len(), 2);

        let quorum_set = QuorumSet::from([quorum_slice1, quorum_slice2]);
        assert_eq!(quorum_set.slices.len(), 2);
    }
}
