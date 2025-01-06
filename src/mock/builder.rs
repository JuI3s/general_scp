use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    herder::herder::HerderDriver,
    overlay::peer_node::PeerNode,
    overlay_impl::{
        in_memory_conn::{InMemoryConn, InMemoryConnBuilder}
        ,
        tcp_conn::{TCPConn, TCPConnBuilder},
    },
    scp::{
        local_node::LocalNodeInfoBuilderFromFile, nomination_protocol::NominationValue, scp::NodeID,
    },
};
use crate::scp::builder::InMemoryNodeBuilder;
use super::state::{MockState, MockStateDriver};

pub enum NodeBuilderDir {
    Test,
}

impl NodeBuilderDir {
    pub fn get_dir_path(&self) -> &'static str {
        match self {
            NodeBuilderDir::Test => "test",
        }
    }
}

pub type InMemoryPeerNode<N, H> = PeerNode<N, H, InMemoryConn<N>, InMemoryConnBuilder<N>>;

pub type MockInMemoryPeerNode =
    PeerNode<MockState, MockStateDriver, InMemoryConn<MockState>, InMemoryConnBuilder<MockState>>;

pub type MockTCPPeerNode =
    PeerNode<MockState, MockStateDriver, TCPConn<MockState>, TCPConnBuilder<MockState>>;

pub struct MockTCPNodeBuilder {
    pub nodes: HashMap<NodeID, Rc<RefCell<MockTCPPeerNode>>>,
    local_node_info_builder: LocalNodeInfoBuilderFromFile,
}

impl MockTCPNodeBuilder {
    pub fn new(quorum_dir_path: &str) -> Self {
        let local_node_info_builder = LocalNodeInfoBuilderFromFile::new(quorum_dir_path);

        Self {
            local_node_info_builder,
            nodes: Default::default(),
        }
    }

    pub fn build_node(&mut self, node_idx: &str) -> Option<MockTCPPeerNode> {
        let local_node_info = self.local_node_info_builder.build_from_file(node_idx)?;

        let conn_builder = TCPConnBuilder::new();
        let herder = MockStateDriver::new();
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::new(None)));

        let peer = PeerNode::new(
            node_idx.to_owned(),
            herder,
            conn_builder,
            local_node_info,
            work_scheduler,
        );

        Some(peer)
    }
}

pub type MockInMemoryNodeBuilder = InMemoryNodeBuilder<MockState, MockStateDriver>;

#[cfg(test)]
mod tests {
    use crate::mock::builder::NodeBuilderDir;

    #[test]
    fn test_mock_node_builder_ok() {
        let mut builder = super::MockInMemoryNodeBuilder::new(NodeBuilderDir::Test.get_dir_path());

        let node1 = builder.build_node("node1");
        assert!(node1.is_some());

        let node2 = builder.build_node("node2");
        assert!(node2.is_some());

        let node3 = builder.build_node("node3");
        assert!(node3.is_none());
    }
}
