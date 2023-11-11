use std::sync::{Mutex, Arc};

use syn::token::Mut;

use crate::application::quorum::QuorumSet;

use super::scp::NodeID;

pub type HLocalNode = Arc<Mutex<LocalNode>>;
pub struct  LocalNode {
    pub is_validator: bool,
    pub quorum_set: QuorumSet,
    pub node_id: NodeID,
}

impl LocalNode {
    pub fn is_v_blocking() -> bool {
        todo!()
    }

    pub fn is_quorum() -> bool {
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
