use std::{cell::RefCell, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    overlay::peer_node::PeerNode,
    overlay_impl::{
        in_memory_conn::{InMemoryConn, InMemoryConnBuilder},
        in_memory_global::InMemoryGlobalState,
    },
    scp::{
        local_node::LocalNodeInfoBuilderFromFile, local_node_builder::LocalNodeBuilder,
        scp_driver::SlotDriver,
    },
};

use super::state::{MockState, MockStateDriver};

// Build nodes used for testing. Initiate nodes from quorum sets data stored on file. Use in memory connectoins.
pub struct MockNodeBuilder {
    local_node_info_builder: LocalNodeInfoBuilderFromFile,
    global_state: Rc<RefCell<InMemoryGlobalState<MockState>>>,
}

impl MockNodeBuilder {
    pub fn new(quorum_dir_path: &str) -> Self {
        let local_node_info_builder = LocalNodeInfoBuilderFromFile::new(quorum_dir_path);

        Self {
            local_node_info_builder,
            global_state: InMemoryGlobalState::new_handle(),
        }
    }

    pub fn build_node(
        &mut self,
        node_idx: &str,
    ) -> Option<
        PeerNode<
            MockState,
            MockStateDriver,
            InMemoryConn<MockState>,
            InMemoryConnBuilder<MockState>,
        >,
    > {
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

        Some(peer)
    }
}
