use std::marker::PhantomData;

use crate::application::quorum::QuorumSet;

use super::local_node::LocalNode;
use super::nomination_protocol::NominationValue;
use super::scp::NodeID;

pub struct LocalNodeBuilder<N>
where
    N: NominationValue + 'static,
{
    is_validator: Option<bool>,
    quorum_set: Option<QuorumSet>,
    node_id: Option<NodeID>,
    phantom: PhantomData<N>,
}

impl<N> Default for LocalNodeBuilder<N>
where
    N: NominationValue + 'static,
{
    fn default() -> Self {
        Self {
            is_validator: Default::default(),
            quorum_set: Default::default(),
            node_id: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<N> LocalNodeBuilder<N>
where
    N: NominationValue + 'static,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_validator(mut self, is_validator: bool) -> Self {
        self.is_validator = Some(is_validator);
        self
    }

    pub fn quorum_set(mut self, quorum_set: QuorumSet) -> Self {
        self.quorum_set = Some(quorum_set);
        self
    }

    pub fn node_id(mut self, node_id: NodeID) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn build(self) -> Result<LocalNode<N>, &'static str> {
        if self.is_validator.is_none() {
            return Err("Missing is_validator.");
        }

        if self.quorum_set.is_none() {
            return Err("Missing quorum set.");
        }

        if self.node_id.is_none() {
            return Err("Missing node id.");
        }

        Ok(LocalNode::<N>::new(
            self.is_validator.unwrap(),
            self.quorum_set.unwrap(),
            self.node_id.unwrap(),
        ))
    }
}
