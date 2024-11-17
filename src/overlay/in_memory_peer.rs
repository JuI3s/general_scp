use std::{arch::global_asm, cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    application::{clock::VirtualClock, work_queue::WorkScheduler},
    herder::herder::HerderDriver,
    scp::{local_node::LocalNodeInfo, nomination_protocol::NominationValue},
};

use super::{
    in_memory_conn::{InMemoryConn, InMemoryConnBuilder},
    in_memory_global::InMemoryGlobalState,
    peer::PeerID,
    peer_node::PeerNode,
};

pub struct InMemoryPeerBuilder<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
    herder: H,
}

impl<N, H> InMemoryPeerBuilder<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + Clone,
{
    pub fn new(global_state: Rc<RefCell<InMemoryGlobalState<N>>>, herder: H) -> Self {
        Self {
            global_state,
            herder,
        }
    }

    pub fn build_node(
        &self,
        peer_idx: PeerID,
        local_node_info: LocalNodeInfo<N>,
    ) -> PeerNode<N, H, InMemoryConn<N>, InMemoryConnBuilder<N>> {
        let conn_builder = InMemoryConnBuilder::new(&self.global_state);
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::new(None)));
        PeerNode::new(
            peer_idx,
            self.herder.clone(),
            conn_builder,
            &self.global_state,
            local_node_info,
            work_scheduler,
        )
    }
}
