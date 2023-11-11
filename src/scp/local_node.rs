use std::{
    collections::BTreeMap,
    f32::consts::E,
    sync::{Arc, Mutex},
};

use syn::token::Mut;

use crate::application::quorum::QuorumSet;

use super::{ballot_protocol::SCPStatement, scp::NodeID, scp_driver::HSCPEnvelope};

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
            if !node_set.iter().any(|node| {
                quorum_slice
                    .data
                    .iter()
                    .any(|node_data| node_data.node_id == *node)
            }) {
                return false;
            }
            true
        })
    }

    pub fn is_v_blocking(
        quorum_set: &QuorumSet,
        envelope_map: &BTreeMap<NodeID, HSCPEnvelope>,
        filter: &impl Fn(&SCPStatement) -> bool,
    ) -> bool {
        let mut nodes: Vec<NodeID> = vec![];
        envelope_map.iter().for_each(|ele| {
            if filter(ele.1.lock().unwrap().get_statement()) {
                nodes.push(ele.0.clone());
            }
        });
        LocalNode::is_v_blocking_internal(quorum_set, &nodes)
    }

    pub fn is_quorum(quorum_set: &QuorumSet, envelopes: &BTreeMap<NodeID, HSCPEnvelope>, ratify_predicate: impl Fn(&SCPStatement) -> bool) -> bool {
        todo!()
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
