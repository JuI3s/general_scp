use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, create_dir_all},
    io::Write,
    iter::{self},
    marker::PhantomData,
    path::PathBuf,
    rc::Rc,
};

use serde_derive::{Deserialize, Serialize};

use crate::{
    application::quorum::{
        is_v_blocking, nodes_fill_quorum_slice, nodes_form_quorum, HQuorumSet, QuorumNode,
        QuorumSet, QuorumSlice,
    },
    mock::state::MockState,
    utils::config::test_data_dir,
};

use super::{
    envelope::{SCPEnvelopeController, SCPEnvelopeID},
    nomination_protocol::NominationValue,
    scp::NodeID,
    statement::SCPStatement,
};

pub type HLocalNode<N> = Rc<RefCell<LocalNodeInfo<N>>>;

pub struct LocalNodeInfoBuilderFromFile {
    nodes: BTreeMap<NodeID, QuorumNode>,
    quorum_dir: PathBuf,
}

impl LocalNodeInfoBuilderFromFile {
    pub fn new(quorum_dir_path: &str) -> Self {
        let quorum_dir = test_data_dir()
            .join(LocalNodeInfo::<MockState>::TEST_DATA_DIR)
            .join(quorum_dir_path);

        Self {
            nodes: BTreeMap::new(),
            quorum_dir,
        }
    }

    fn build_local_info_from_toml<N: NominationValue>(
        &mut self,
        toml_info: LocalNodeInfoToml,
    ) -> Option<LocalNodeInfo<N>> {
        for node_id in toml_info
            .quorum_set
            .iter()
            .flatten()
            .chain(iter::once(&toml_info.node_id))
        {
            if !self.nodes.contains_key(node_id) {
                let node = QuorumNode::from_toml(node_id)?;
                self.nodes.insert(node_id.to_owned(), node);
            }
        }

        let slices = toml_info
            .quorum_set
            .iter()
            .map(|node_slice| QuorumSlice {
                data: node_slice
                    .iter()
                    .map(|node_id| self.nodes.get(node_id).unwrap().to_owned())
                    .collect(),
            })
            .collect();

        let quorum_set = QuorumSet {
            slices,
            threshold: 0,
        };

        let local_node_info = LocalNodeInfo::<N>::from_toml_info(toml_info, quorum_set);

        Some(local_node_info)
    }

    pub fn build_from_file<N: NominationValue>(
        &mut self,
        node_id: &str,
    ) -> Option<LocalNodeInfo<N>> {
        let path = self.quorum_dir.join(node_id);
        let toml_str = fs::read_to_string(path).ok()?;
        let node_toml = toml::from_str(&toml_str).ok()?;

        let node_info = self.build_local_info_from_toml(node_toml)?;
        Some(node_info)
    }
}

#[derive(Serialize, Deserialize)]
pub struct LocalNodeInfoToml {
    pub is_validator: bool,
    pub quorum_set: BTreeSet<BTreeSet<NodeID>>,
    pub node_id: NodeID,
}

impl<N> From<LocalNodeInfo<N>> for LocalNodeInfoToml
where
    N: NominationValue,
{
    fn from(local_node_info: LocalNodeInfo<N>) -> Self {
        let quorum_set = local_node_info
            .quorum_set
            .slices
            .iter()
            .map(|slice| {
                slice
                    .data
                    .iter()
                    .map(|node_data| node_data.node_id.clone())
                    .collect()
            })
            .collect();

        Self {
            is_validator: local_node_info.is_validator,
            quorum_set,
            node_id: local_node_info.node_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct LocalNodeInfo<N>
where
    N: NominationValue + 'static,
{
    pub is_validator: bool,
    pub quorum_set: QuorumSet,
    pub node_id: NodeID,
    phantom: PhantomData<N>,
}

impl<N> Into<Rc<RefCell<LocalNodeInfo<N>>>> for LocalNodeInfo<N>
where
    N: NominationValue,
{
    fn into(self) -> Rc<RefCell<LocalNodeInfo<N>>> {
        RefCell::new(self).into()
    }
}

impl<N> LocalNodeInfo<N>
where
    N: NominationValue,
{
    const TEST_DATA_DIR: &'static str = "node_info";

    pub fn new(is_validator: bool, quorum_set: QuorumSet, node_id: NodeID) -> Self {
        Self {
            is_validator,
            quorum_set,
            node_id,
            phantom: PhantomData,
        }
    }

    pub fn from_toml_info(toml_info: LocalNodeInfoToml, quorum_set: QuorumSet) -> Self {
        Self {
            is_validator: toml_info.is_validator,
            quorum_set,
            node_id: toml_info.node_id,
            phantom: PhantomData,
        }
    }

    pub fn write_toml(&self, dir_name: &str) {
        let path = test_data_dir()
            .join(Self::TEST_DATA_DIR)
            .join(dir_name)
            .join(self.node_id.clone());
        let _ = create_dir_all(path.parent().unwrap());
        let local_node_info_toml = LocalNodeInfoToml::from(self.clone());

        let toml = toml::to_string(&local_node_info_toml).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(toml.as_bytes()).unwrap();
    }

    pub fn for_all_nodes(
        quorum_set: &QuorumSet,
        predicate: &mut impl FnMut(&NodeID) -> bool,
    ) -> bool {
        // This function applies the predicate to each node_id in the quorum set. If the
        // predicate evalutes false on any node, return false immediatley. Otherwise,
        // return true.nnnn

        let nodes = quorum_set.nodes();

        for node in &nodes {
            if !predicate(&node.node_id) {
                return false;
            }
        }

        true
    }

    pub fn is_v_blocking_with_predicate(
        quorum_set: &QuorumSet,
        envelope_map: &BTreeMap<NodeID, SCPEnvelopeID>,
        filter: &impl Fn(&SCPStatement<N>) -> bool,
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        let mut nodes: Vec<NodeID> = vec![];
        envelope_map.iter().for_each(|entry| {
            let env = envelope_controller.get_envelope(entry.1).unwrap();
            println!(
                "env nomination_values: {:?}",
                env.get_statement().get_nomination_values()
            );
            println!(
                "env st votes: {:?}",
                env.get_statement().as_nomination_statement().votes
            );
            println!(
                "env st accepted: {:?}",
                env.get_statement().as_nomination_statement().accepted
            );

            if filter(env.get_statement()) {
                nodes.push(entry.0.clone());
            }
        });
        // TODO: fix this
        println!(
            "is_v_blocking_with_predicate nodes: {:?}, quorum_set: {:?}, envelope_map: {:?}",
            nodes, quorum_set, envelope_map,
        );
        is_v_blocking(quorum_set, &nodes)
    }
}

pub fn extract_nodes_from_statement_with_filter<N: NominationValue>(
    envelopes: &BTreeMap<NodeID, SCPEnvelopeID>,
    envelope_controller: &SCPEnvelopeController<N>,
    node_filter: impl Fn(&SCPStatement<N>) -> bool,
) -> Vec<NodeID> {
    println!(
        "extract_nodes_from_statement_with_filter envelopes: {:?}",
        envelopes
    );

    let nodes: Vec<NodeID> = envelopes
        .iter()
        .map(|entry| {
            let envelope = envelope_controller.get_envelope(entry.1).unwrap();

            if node_filter(&envelope.statement) {
                Some(entry.0.to_owned())
            } else {
                None
            }
        })
        .take_while(|id| id.is_some())
        .map(|x| x.unwrap())
        .collect();
    nodes
}

impl<N> Default for LocalNodeInfo<N>
where
    N: NominationValue,
{
    fn default() -> Self {
        Self {
            is_validator: Default::default(),
            quorum_set: Default::default(),
            node_id: Default::default(),
            phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        arch::aarch64::veor3q_s16,
        net::{Ipv4Addr, SocketAddrV4},
        sync::{Arc, Mutex},
    };

    use crate::{
        application::quorum::QuorumNode,
        scp::{
            envelope::SCPEnvelope, nomination_protocol::SCPNominationValue, scp_driver::HashValue,
        },
    };

    use super::*;

    fn create_test_node(index: u16) -> (NodeID, QuorumNode) {
        let sock = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080 + index);
        let node_id = "node".to_string() + &index.to_string();
        let node = QuorumNode {
            node_id: node_id.clone(),
            ip_addr: Some(sock),
        };
        (node_id, node)
    }

    #[test]
    fn quorum_test_1() {
        // V1's quorum slice is not a quorum without 'v4'.
        //              ┌────┐
        //     ┌───────▶│ 4  │◀──────────┐
        //     │        └────┘           │
        //     │                         │
        //     │                         │
        //     ▼                         ▼
        //  ┌────┐                    ┌────┐
        //  │ 2  │◀──────────────────▶│ 3  │
        //  └────┘                    └────┘
        //     ▲                         ▲
        //     │                         │
        //     │         ┌────┐          │
        //     └─────────│ 1  │──────────┘
        //               └────┘

        let mut env_controller = SCPEnvelopeController::<SCPNominationValue>::new();

        let (node_id1, node1) = create_test_node(1);
        let (node_id2, node2) = create_test_node(2);
        let (node_id3, node3) = create_test_node(3);
        let (node_id4, node4) = create_test_node(4);

        let quorum_slice1 =
            QuorumSlice::from([node1.to_owned(), node2.to_owned(), node3.to_owned()]);
        let quorum_slice2 =
            QuorumSlice::from([node2.to_owned(), node3.to_owned(), node4.to_owned()]);

        let quorum1 = QuorumSet::from([quorum_slice1]);
        let quorum2 = QuorumSet::from([quorum_slice2.clone()]);

        let mut envelopes = BTreeMap::new();
        let env1 = SCPEnvelope::test_make_scp_envelope_from_quorum(
            node_id1.to_owned(),
            &quorum1,
            &mut env_controller,
        );
        let env2 = SCPEnvelope::test_make_scp_envelope_from_quorum(
            node_id2.to_owned(),
            &quorum2,
            &mut env_controller,
        );
        let env3 = SCPEnvelope::test_make_scp_envelope_from_quorum(
            node_id3.to_owned(),
            &quorum2,
            &mut env_controller,
        );
        let env4 = SCPEnvelope::test_make_scp_envelope_from_quorum(
            node_id4.to_owned(),
            &quorum2,
            &mut env_controller,
        );

        envelopes.insert(node_id1.to_owned(), env1);
        envelopes.insert(node_id2.to_owned(), env2);
        envelopes.insert(node_id3.to_owned(), env3);
        envelopes.insert(node_id4.to_owned(), env4);

        let mut quorum_map: BTreeMap<HashValue, QuorumSet> = BTreeMap::new();
        quorum_map.insert(quorum1.hash_value(), quorum1.clone());
        quorum_map.insert(quorum2.hash_value(), quorum2.clone());

        {
            let get_quorum_set_predicate = |node_id| {
                let env_id = envelopes.get(node_id).clone().unwrap();
                let env = env_controller.get_envelope(env_id).unwrap();
                let st: &SCPStatement<SCPNominationValue> = env.get_statement();

                quorum_map.get(&st.quorum_set_hash_value())
            };

            let nodes =
                extract_nodes_from_statement_with_filter(&envelopes, &env_controller, |_| true);
            assert_eq!(nodes_form_quorum(get_quorum_set_predicate, &nodes), true);
        }

        envelopes.remove(&node_id2);
        envelopes.remove(&node_id3);
        envelopes.remove(&node_id4);

        {
            let get_quorum_set_predicate = |node_id| {
                let env_id = envelopes.get(node_id).clone().unwrap();
                let env = env_controller.get_envelope(env_id).unwrap();
                let st: &SCPStatement<SCPNominationValue> = env.get_statement();

                quorum_map.get(&st.quorum_set_hash_value())
            };

            let nodes =
                extract_nodes_from_statement_with_filter(&envelopes, &env_controller, |_| true);
            assert_eq!(nodes_form_quorum(get_quorum_set_predicate, &nodes), false);
        }
    }
}
