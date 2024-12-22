use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet},
    fs::{self, create_dir},
    hash::{Hash, Hasher},
    io::Write,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use blake2::Digest;

use serde::{Deserialize, Serialize};

use crate::{
    crypto::types::{Blake2Hash, Blake2Hashable},
    overlay::peer::PeerID,
    scp::{
        envelope::{SCPEnvelopeController, SCPEnvelopeID},
        local_node::LocalNodeInfo,
        nomination_protocol::NominationValue,
        scp::NodeID,
        scp_driver::HashValue,
        statement::SCPStatement,
    },
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

pub fn is_v_blocking(quorum_set: &QuorumSet, node_set: &Vec<NodeID>) -> bool {
    quorum_set.slices.iter().all(|quorum_slice| {
        node_set.iter().any(|node| {
            quorum_slice
                .data
                .iter()
                .any(|node_data| node_data.node_id == *node)
        })
    })
}

pub fn accept_predicate<N: NominationValue>(value: &N, statement: &SCPStatement<N>) -> bool {
    statement.as_nomination_statement().accepted.contains(value)
}

pub fn has_voted_predicate<N: NominationValue>(value: &N, statement: &SCPStatement<N>) -> bool {
    statement.as_nomination_statement().votes.contains(value)
        || statement.as_nomination_statement().accepted.contains(value)
}

pub fn is_quorum(slice: &QuorumSlice, quorum_set: &QuorumSet) -> bool {
    quorum_set.slices.iter().any(|quorum_slice| {
        slice
            .data
            .iter()
            .all(|node| quorum_slice.data.contains(node))
    })
}

pub fn nodes_fill_quorum_slice(quorum_slice: &QuorumSlice, nodes: &Vec<NodeID>) -> bool {
    /// Check if the nodes contain the entire quorum slice.
    quorum_slice
        .data
        .iter()
        .all(|node| nodes.contains(&node.node_id))
}

pub fn nodes_fill_one_quorum_slice_in_quorum_set(
    quorum_set: &QuorumSet,
    nodes: &Vec<NodeID>,
) -> bool {
    quorum_set
        .slices
        .iter()
        .any(|slice| nodes_fill_quorum_slice(slice, nodes))
}

// `is_quorum_with_node_filter` tests if the filtered nodes V form a quorum
// (meaning for each v \in V there is q \in Q(v)
// isQuorumincluded in V and we have quorum on V for qSetHash). `qfun` extracts
// the SCPQuorumSetPtr from the SCPStatement for its associated node in map
// (required for transitivity)
pub fn is_quorum_with_node_filter<'a>(
    local_quorum: Option<(&QuorumSet, &NodeID)>,
    get_quorum_set: impl Fn(&'a NodeID) -> Option<HQuorumSet>,
    nodes: &'a Vec<NodeID>,
) -> bool {
    // TODO: do not need input from self?
    // if let Some((_, local_node_id)) = local_quorum {
    //     nodes.push(local_node_id.to_owned());
    // }
    println!("nodes: {:?}", nodes);

    // Definition (quorum). A set of nodes 𝑈 ⊆ 𝐕 in FBAS ⟨𝐕,𝐐⟩ is a quorum iff 𝑈 ≠ ∅
    // and 𝑈 contains a slice for each member—i.e., ∀𝑣 ∈ 𝑈 , ∃𝑞 ∈ 𝐐(𝑣) such that 𝑞 ⊆
    // 𝑈 .

    let mut ret = if nodes.is_empty() {
        false
    } else {
        nodes.iter().all(|node| {
            // let env_id = envelopes.get(node).unwrap();
            // let env = envelope_controller.get_envelope(env_id).unwrap();
            // let statement = env.get_statement();

            if let Some(quorum_set) = get_quorum_set(node) {
                nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set.lock().unwrap(), &nodes)
            } else {
                false
            }
        })
    };

    // Check for local node.
    if let Some((local_quorum_set, _)) = local_quorum {
        ret = ret && nodes_fill_one_quorum_slice_in_quorum_set(local_quorum_set, &nodes);
    }

    ret
}

#[cfg(test)]
mod tests {
    use crate::{
        mock::state::MockState,
        scp::local_node::{LocalNodeInfo, LocalNodeInfoBuilderFromFile},
    };

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

    #[test]
    fn test_is_v_blocking() {
        let mut builder = LocalNodeInfoBuilderFromFile::new("test");
        let node1_info: LocalNodeInfo<MockState> = builder.build_from_file("node1").unwrap();

        let node_sets = vec![
            vec!["node1".to_string()],
            vec!["node1".to_string(), "node2".to_string()],
            vec!["node2".to_string()],
            vec![
                "node1".to_string(),
                "node2".to_string(),
                "node3".to_string(),
            ],
        ];

        for node_set in node_sets {
            let is_v_blocking = is_v_blocking(&node1_info.quorum_set, &node_set);
            assert!(is_v_blocking);
        }

        assert!(is_v_blocking(
            &node1_info.quorum_set,
            &vec!["node3".to_string()]
        ));
    }

    #[test]
    fn test_nodes_fill_quorum_slice() {
        let quorum_slice =
            QuorumSlice::from([make_quorum_node_for_test(1), make_quorum_node_for_test(2)]);
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        let is_quorum = nodes_fill_quorum_slice(&quorum_slice, &nodes);
        assert!(is_quorum);

        let nodes = vec!["node1".to_string()];
        let is_quorum = nodes_fill_quorum_slice(&quorum_slice, &nodes);
        assert!(!is_quorum);
    }

    #[test]
    fn test_nodes_fill_one_quorum_slice_in_quorum_set() {
        // [node1, node2]
        let quorum_slice1 =
            QuorumSlice::from([make_quorum_node_for_test(1), make_quorum_node_for_test(2)]);
        // [node1, node3]
        let quorum_slice2 =
            QuorumSlice::from([make_quorum_node_for_test(1), make_quorum_node_for_test(3)]);
        let quorum_set = QuorumSet::from([quorum_slice1, quorum_slice2]);

        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let is_quorum = nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set, &nodes);
        assert!(is_quorum);

        let nodes = vec!["node1".to_string(), "node3".to_string()];
        let is_quorum = nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set, &nodes);
        assert!(is_quorum);

        // [node1, node2, node3]
        let nodes = vec![
            "node1".to_string(),
            "node2".to_string(),
            "node3".to_string(),
        ];
        let fill_one_quorum_set = nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set, &nodes);
        assert!(fill_one_quorum_set);

        // [node1]
        let mut nodes = vec!["node1".to_string()];
        let fill_one_quorum_set = nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set, &nodes);
        assert!(!fill_one_quorum_set);

        for node in vec!["node2".to_string(), "node3".to_string()] {
            nodes.push(node);
            let fill_one_quorum_set =
                nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set, &nodes);
            assert!(fill_one_quorum_set);
        }
    }

    #[test]
    fn test_a_quorum_has_responded() {}
}
