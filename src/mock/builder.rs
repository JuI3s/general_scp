use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    overlay::{message::MessageController, peer_node::PeerNode},
    overlay_impl::{
        in_memory_conn::{InMemoryConn, InMemoryConnBuilder},
        in_memory_global::InMemoryGlobalState,
        tcp_conn::{TCPConn, TCPConnBuilder},
    },
    scp::{local_node::LocalNodeInfoBuilderFromFile, scp::NodeID},
};

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

pub type MockInMemoryPeerNode<'a> = PeerNode<
    'a,
    MockState,
    MockStateDriver,
    InMemoryConn<MockState>,
    InMemoryConnBuilder<MockState>,
>;

pub type MockTCPPeerNode<'a> =
    PeerNode<'a, MockState, MockStateDriver, TCPConn<MockState>, TCPConnBuilder<MockState>>;

pub fn start_tcp_node(node_idx: &str) -> Rc<RefCell<MockTCPPeerNode>> {
    let mut builder = MockTCPNodeBuilder::new(NodeBuilderDir::Test.get_dir_path());
    let peer = builder.build_node(node_idx).unwrap();

    peer
}

pub struct MockTCPNodeBuilder<'a> {
    pub nodes: HashMap<NodeID, Rc<RefCell<MockTCPPeerNode<'a>>>>,
    local_node_info_builder: LocalNodeInfoBuilderFromFile,
}

impl<'a> MockTCPNodeBuilder<'a> {
    pub fn new(quorum_dir_path: &str) -> Self {
        let local_node_info_builder = LocalNodeInfoBuilderFromFile::new(quorum_dir_path);

        Self {
            local_node_info_builder,
            nodes: Default::default(),
        }
    }

    pub fn build_node(&mut self, node_idx: &str) -> Option<Rc<RefCell<MockTCPPeerNode>>> {
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

        let peer_handle = Rc::new(RefCell::new(peer));

        self.nodes.insert(node_idx.to_owned(), peer_handle.clone());

        Some(peer_handle)
    }
}

// Build nodes used for testing. Initiate nodes from quorum sets data stored on file. Use in memory connectoins.
pub struct MockInMemoryNodeBuilder {
    pub global_state: Rc<RefCell<InMemoryGlobalState<MockState>>>,
    pub nodes: HashMap<NodeID, Rc<RefCell<MockInMemoryPeerNode>>>,
    local_node_info_builder: LocalNodeInfoBuilderFromFile,
}

impl MockInMemoryNodeBuilder {
    pub fn new(quorum_dir_path: &str) -> Self {
        let local_node_info_builder = LocalNodeInfoBuilderFromFile::new(quorum_dir_path);

        Self {
            local_node_info_builder,
            global_state: InMemoryGlobalState::new_handle(),
            nodes: Default::default(),
        }
    }

    pub fn build_node(&mut self, node_idx: &str) -> Option<Rc<RefCell<MockInMemoryPeerNode>>> {
        let local_node_info: crate::scp::local_node::LocalNodeInfo<MockState> =
            self.local_node_info_builder.build_from_file(node_idx)?;

        let conn_builder = InMemoryConnBuilder::new(&self.global_state);
        let herder = MockStateDriver::new();
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::new(None)));

        let peer = PeerNode::new(
            node_idx.to_owned(),
            herder,
            conn_builder,
            local_node_info,
            work_scheduler,
        );

        let msg_controller = peer.message_controller.clone();
        let peer_handle = Rc::new(RefCell::new(peer));

        self.nodes.insert(node_idx.to_owned(), peer_handle.clone());

        self.global_state
            .borrow_mut()
            .peer_msg_queues
            .insert(node_idx.to_owned(), msg_controller);

        Some(peer_handle)
    }

    pub fn set_leader(&mut self, node_idx: &NodeID) {
        for peer in self.nodes.values() {
            peer.borrow_mut().add_leader(node_idx);
        }
    }
}

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
