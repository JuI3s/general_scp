use std::{
    collections::BTreeMap,
    f32::consts::E,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use syn::token::Mut;

use crate::application::quorum::{HQuorumSet, QuorumSet, QuorumSlice};

use super::{
    nomination_protocol::NominationValue,
    scp::NodeID,
    scp_driver::HSCPEnvelope,
    statement::{HSCPStatement, SCPStatement},
};

pub type HLocalNode<N> = Arc<Mutex<LocalNode<N>>>;
pub struct LocalNode<N>
where
    N: NominationValue + 'static,
{
    pub is_validator: bool,
    pub quorum_set: QuorumSet,
    pub node_id: NodeID,
    phantom: PhantomData<N>,
}

impl<N> LocalNode<N>
where
    N: NominationValue,
{
    pub fn new(is_validator: bool, quorum_set: QuorumSet, node_id: NodeID) -> Self {
        Self {
            is_validator: is_validator,
            quorum_set: quorum_set,
            node_id: node_id,
            phantom: PhantomData,
        }
    }

    pub fn get_quorum_set(&self) -> &QuorumSet {
        todo!();
    }

    // This implementation is different from the Stellar implementation because we have different data structures.
    fn is_v_blocking_internal(quorum_set: &QuorumSet, node_set: &Vec<NodeID>) -> bool {
        // TODO: do we need this? Now validators are represented by quorum slices.
        // if quorum_set.threshold == 0 {
        // return false;
        // }
        quorum_set.slices.iter().all(|quorum_slice| {
            node_set.iter().any(|node| {
                quorum_slice
                    .data
                    .iter()
                    .any(|node_data| node_data.node_id == *node)
            })
        })
    }

    pub fn is_v_blocking(
        quorum_set: &QuorumSet,
        envelope_map: &BTreeMap<NodeID, HSCPEnvelope<N>>,
        filter: &impl Fn(&SCPStatement<N>) -> bool,
    ) -> bool {
        let mut nodes: Vec<NodeID> = vec![];
        envelope_map.iter().for_each(|entry| {
            if filter(entry.1.lock().unwrap().get_statement()) {
                nodes.push(entry.0.clone());
            }
        });
        LocalNode::<N>::is_v_blocking_internal(quorum_set, &nodes)
    }

    fn nodes_fill_quorum_slice(quorum_slice: &QuorumSlice, nodes: &Vec<NodeID>) -> bool {
        quorum_slice
            .data
            .iter()
            .all(|node| nodes.contains(&node.node_id))
    }

    fn nodes_fill_one_quorum_slice_in_quorum_set(
        quorum_set: &QuorumSet,
        nodes: &Vec<NodeID>,
    ) -> bool {
        quorum_set
            .slices
            .iter()
            .any(|slice| LocalNode::<N>::nodes_fill_quorum_slice(slice, nodes))
    }

    // `is_quorum_with_node_filter` tests if the filtered nodes V form a quorum
    // (meaning for each v \in V there is q \in Q(v)
    // isQuorumincluded in V and we have quorum on V for qSetHash). `qfun` extracts the
    // SCPQuorumSetPtr from the SCPStatement for its associated node in map
    // (required for transitivity)

    pub fn is_quorum_with_node_filter(
        local_quorum: Option<(&QuorumSet, &NodeID)>,
        envelopes: &BTreeMap<NodeID, HSCPEnvelope<N>>,
        get_quorum_set_predicate: impl Fn(&SCPStatement<N>) -> Option<HQuorumSet>,
        node_filter: impl Fn(&SCPStatement<N>) -> bool,
    ) -> bool {
        // let mut nodes: Vec<NodeID> = vec![];

        let mut nodes: Vec<NodeID> = envelopes
            .iter()
            .map(|entry| {
                let envelope = entry.1.lock().unwrap();

                if node_filter(&envelope.statement) {
                    Some(entry.0.to_owned())
                } else {
                    None
                }
            })
            .take_while(|id| id.is_some())
            .map(|x| x.unwrap())
            .collect();

        if let Some((_, local_node_id)) = local_quorum {
            nodes.push(local_node_id.to_owned());
        }

        // Definition (quorum). A set of nodes 𝑈 ⊆ 𝐕 in FBAS ⟨𝐕,𝐐⟩ is a quorum iff 𝑈 ≠ ∅ and 𝑈 contains a slice for each member—i.e., ∀𝑣 ∈ 𝑈 , ∃𝑞 ∈ 𝐐(𝑣) such that 𝑞 ⊆ 𝑈 .
        let mut ret = if nodes.is_empty() {
            false
        } else {
            nodes.iter().all(|node| {
                if let Some(quorum_set) = get_quorum_set_predicate(
                    envelopes.get(node).unwrap().lock().unwrap().get_statement(),
                ) {
                    LocalNode::<N>::nodes_fill_one_quorum_slice_in_quorum_set(
                        &quorum_set.lock().unwrap(),
                        &nodes,
                    )
                } else {
                    false
                }
            })
        };

        // Check for local node.
        if let Some((local_quorum_set, _)) = local_quorum {
            ret = ret
                && LocalNode::<N>::nodes_fill_one_quorum_slice_in_quorum_set(
                    local_quorum_set,
                    &nodes,
                );
        }

        ret
    }

    pub fn is_quorum(
        local_quorum: Option<(&QuorumSet, &NodeID)>,
        envelopes: &BTreeMap<NodeID, HSCPEnvelope<N>>,
        get_quorum_set_predicate: impl Fn(&SCPStatement<N>) -> Option<HQuorumSet>,
    ) -> bool {
        LocalNode::is_quorum_with_node_filter(
            local_quorum,
            envelopes,
            get_quorum_set_predicate,
            |_| true,
        )
    }
}

impl<N> Default for LocalNode<N>
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
    use std::net::{Ipv4Addr, SocketAddrV4};

    use crate::{
        application::quorum::QuorumNode,
        scp::{
            nomination_protocol::SCPNominationValue,
            scp_driver::{HashValue, SCPEnvelope},
        },
    };

    use super::*;

    fn create_test_node(index: u16) -> (NodeID, QuorumNode) {
        let sock = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080 + index);
        let node_id = "node".to_string() + &index.to_string();
        let node = QuorumNode {
            node_id: node_id.clone(),
            addr: sock,
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

        let mut envelopes: BTreeMap<NodeID, HSCPEnvelope<SCPNominationValue>> = BTreeMap::new();
        let env1 = SCPEnvelope::test_make_scp_envelope_from_quorum(node_id1.to_owned(), &quorum1);
        let env2 = SCPEnvelope::test_make_scp_envelope_from_quorum(node_id2.to_owned(), &quorum2);
        let env3 = SCPEnvelope::test_make_scp_envelope_from_quorum(node_id3.to_owned(), &quorum2);
        let env4 = SCPEnvelope::test_make_scp_envelope_from_quorum(node_id4.to_owned(), &quorum2);

        envelopes.insert(node_id1.to_owned(), Arc::new(Mutex::new(env1)));
        envelopes.insert(node_id2.to_owned(), Arc::new(Mutex::new(env2)));
        envelopes.insert(node_id3.to_owned(), Arc::new(Mutex::new(env3)));
        envelopes.insert(node_id4.to_owned(), Arc::new(Mutex::new(env4)));

        let mut quorum_map: BTreeMap<HashValue, HQuorumSet> = BTreeMap::new();
        quorum_map.insert(quorum1.hash_value(), Arc::new(Mutex::new(quorum1.clone())));
        quorum_map.insert(quorum2.hash_value(), Arc::new(Mutex::new(quorum2.clone())));

        let get_quorum_set_predicate = |st: &SCPStatement<SCPNominationValue>| {
            quorum_map
                .get(&st.quorum_set_hash_value())
                .map(|val| val.clone())
        };

        assert_eq!(
            LocalNode::is_quorum(None, &envelopes, get_quorum_set_predicate),
            true
        );
        envelopes.remove(&node_id2);
        envelopes.remove(&node_id3);
        envelopes.remove(&node_id4);
        assert_eq!(
            LocalNode::is_quorum(None, &envelopes, get_quorum_set_predicate),
            false
        );
    }
}
