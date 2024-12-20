use std::{cell::RefCell, fmt::Debug, marker::PhantomData, rc::Rc};

use crate::{
    application::quorum::QuorumNode,
    overlay::{
        conn::{PeerConn, PeerConnBuilder},
        message::SCPMessage,
        peer::{PeerID, SCPPeerConnState},
    },
    scp::nomination_protocol::NominationValue,
};

use super::in_memory_global::InMemoryGlobalState;

// InMemoryConn keeps track of connections with an in-memory peer.
pub struct InMemoryConn<N>
where
    N: NominationValue,
{
    peer_id: PeerID,
    conn_state: SCPPeerConnState,
    in_memory_global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
}

impl<N: NominationValue> Debug for InMemoryConn<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryConn")
            .field("peer_id", &self.peer_id)
            .finish()
    }
}

impl<N> InMemoryConn<N>
where
    N: NominationValue,
{
    pub fn new(
        peer_id: PeerID,
        in_memory_global_state: &Rc<RefCell<InMemoryGlobalState<N>>>,
    ) -> Self {
        Self {
            peer_id,
            conn_state: SCPPeerConnState::Connecting,
            in_memory_global_state: in_memory_global_state.clone(),
        }
    }
}

impl<N> PeerConn<N> for InMemoryConn<N>
where
    N: NominationValue,
{
    fn send_message(&mut self, msg: &SCPMessage<N>) {
        self.in_memory_global_state
            .borrow_mut()
            .send_message(self.peer_id.clone(), msg.clone());

        // Sanity check
        assert!(self.in_memory_global_state.borrow().msg_peer_id_queue.len() > 0);
        assert!(
            self.in_memory_global_state
                .borrow()
                .peer_msg_queues
                .get(&self.peer_id)
                .unwrap()
                .borrow()
                .messages
                .len()
                > 0
        );

        println!("InMemoryConn::send_message: to {:?}", self.peer_id);
    }

    fn set_state(&mut self, state: SCPPeerConnState) {
        self.conn_state = state
    }
}

pub struct InMemoryConnBuilder<N>
where
    N: NominationValue,
{
    global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
    phantom: PhantomData<N>,
}

impl<N> InMemoryConnBuilder<N>
where
    N: NominationValue,
{
    pub fn new(global_state: &Rc<RefCell<InMemoryGlobalState<N>>>) -> Self {
        Self {
            global_state: global_state.clone(),
            phantom: PhantomData,
        }
    }
}

impl<N> PeerConnBuilder<N, InMemoryConn<N>> for InMemoryConnBuilder<N>
where
    N: NominationValue,
{
    fn build(&self, peer: &QuorumNode) -> InMemoryConn<N> {
        InMemoryConn::new(peer.node_id.to_string(), &self.global_state)
    }
}
