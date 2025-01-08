use crate::application::work_queue::WorkScheduler;
use crate::herder::herder::HerderDriver;
use crate::mock::builder::InMemoryPeerNode;
use crate::overlay::peer_node::PeerNode;
use crate::overlay_impl::in_memory_conn::{InMemoryConn, InMemoryConnBuilder};
use crate::overlay_impl::in_memory_global::InMemoryGlobalState;
use crate::scp::local_node::LocalNodeInfoBuilderFromFile;
use crate::scp::nomination_protocol::NominationValue;
use crate::scp::scp::NodeID;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// Build nodes used for testing. Initiate nodes from quorum sets data stored on file. Use in memory connectoins.
pub struct InMemoryNodeBuilder<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
    pub nodes: HashMap<NodeID, Rc<RefCell<InMemoryPeerNode<N, H>>>>,
    local_node_info_builder: LocalNodeInfoBuilderFromFile,
}

impl<N, H> InMemoryNodeBuilder<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    pub fn new(quorum_dir_path: &str) -> Self {
        let local_node_info_builder = LocalNodeInfoBuilderFromFile::new(quorum_dir_path);

        Self {
            local_node_info_builder,
            global_state: InMemoryGlobalState::new_handle(),
            nodes: Default::default(),
        }
    }

    pub fn build_node_with_herder(
        &mut self,
        node_idx: &str,
        herder: H,
    ) -> Option<PeerNode<N, H, InMemoryConn<N>, InMemoryConnBuilder<N>>> {
        let local_node_info: crate::scp::local_node::LocalNodeInfo<N> =
            self.local_node_info_builder.build_from_file(node_idx)?;

        let conn_builder = InMemoryConnBuilder::new(&self.global_state);
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::new(None)));

        let peer = PeerNode::new(
            node_idx.to_owned(),
            herder,
            conn_builder,
            local_node_info,
            work_scheduler,
        );

        let msg_controller = peer.message_controller.clone();
        self.global_state
            .borrow_mut()
            .peer_msg_queues
            .insert(node_idx.to_owned(), msg_controller);

        Some(peer)
    }

    pub fn build_node(
        &mut self,
        node_idx: &str,
    ) -> Option<PeerNode<N, H, InMemoryConn<N>, InMemoryConnBuilder<N>>> {
        self.build_node_with_herder(node_idx, H::new())
    }
}
