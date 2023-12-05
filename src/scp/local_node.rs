use std::{
    collections::BTreeMap,
    f32::consts::E,
    sync::{Arc, Mutex},
};

use syn::token::Mut;

use crate::application::quorum::{QuorumSet, QuorumSlice, HQuorumSet};

use super::{
    scp::NodeID,
    scp_driver::HSCPEnvelope,
    statement::{HSCPStatement, SCPStatement},
};

pub type HLocalNode = Arc<Mutex<LocalNode>>;
pub struct LocalNode {
    pub is_validator: bool,
    pub quorum_set: QuorumSet,
    pub node_id: NodeID,
}

impl LocalNode {
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
        envelope_map: &BTreeMap<NodeID, HSCPEnvelope>,
        filter: &impl Fn(&SCPStatement) -> bool,
    ) -> bool {
        let mut nodes: Vec<NodeID> = vec![];
        envelope_map.iter().for_each(|entry| {
            if filter(entry.1.lock().unwrap().get_statement()) {
                nodes.push(entry.0.clone());
            }
        });
        LocalNode::is_v_blocking_internal(quorum_set, &nodes)
    }

    fn nodes_fill_quorum_slice(quorum_slice: &QuorumSlice, nodes: &Vec<NodeID>) -> bool {
        quorum_slice.data.iter().all(|node| {nodes.contains(&node.node_id)})
    }

    fn nodes_fill_one_quorum_slice_in_quorum_set(quorum_set: &QuorumSet, nodes: &Vec<NodeID>) -> bool {
        quorum_set.slices.iter().any(|slice|{
            LocalNode::nodes_fill_quorum_slice(slice, nodes)
        })
    }


    // `is_quorum_with_node_filter` tests if the filtered nodes V form a quorum
    // (meaning for each v \in V there is q \in Q(v)
    // isQuorumincluded in V and we have quorum on V for qSetHash). `qfun` extracts the
    // SCPQuorumSetPtr from the SCPStatement for its associated node in map
    // (required for transitivity)
 
    pub fn is_quorum_with_node_filter(
        local_quorum: Option<(&QuorumSet, &NodeID)>, 
        envelopes: &BTreeMap<NodeID, HSCPEnvelope>,
        get_quorum_set_predicate: impl Fn(&SCPStatement) -> Option<HQuorumSet>,
        node_filter: impl Fn(&SCPStatement) -> bool,
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
                if let Some(quorum_set) = get_quorum_set_predicate(envelopes.get(node).unwrap().lock().unwrap().get_statement()) {
                    LocalNode::nodes_fill_one_quorum_slice_in_quorum_set(&quorum_set.lock().unwrap(), &nodes)
                } else {
                    false
                }
            })
        };

        // Check for local node.
        if let Some((local_quorum_set, _)) = local_quorum {
            ret = ret && LocalNode::nodes_fill_one_quorum_slice_in_quorum_set(local_quorum_set, &nodes);
        }

        ret

    }

    pub fn is_quorum(
        local_quorum: Option<(&QuorumSet, &NodeID)>, 
        envelopes: &BTreeMap<NodeID, HSCPEnvelope>,
        get_quorum_set_predicate: impl Fn(&SCPStatement) -> Option<HQuorumSet>,
    ) -> bool {
        LocalNode::is_quorum_with_node_filter( local_quorum, envelopes, get_quorum_set_predicate, |_| true)
    }
}

impl Default for LocalNode {
    fn default() -> Self {
        Self {
            is_validator: Default::default(),
            quorum_set: Default::default(),
            node_id: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heard_from_quorum() {}
}
