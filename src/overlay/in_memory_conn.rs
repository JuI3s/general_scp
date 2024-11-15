use core::panic;
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use syn::token::LArrow;

use crate::scp::{
    envelope::{self, SCPEnvelopeController},
    nomination_protocol::NominationValue,
};

use super::{
    conn::{PeerConn, PeerConnBuilder},
    in_memory_global::{self, InMemoryGlobalState},
    message::MessageController,
    peer::{PeerID, SCPPeerConnState},
};

// InMemoryConn keeps track of connections with an in-memory peer.
pub struct InMemoryConn<N>
where
    N: NominationValue,
{
    peer_id: PeerID,
    conn_state: SCPPeerConnState,
    in_memory_global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
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
    fn send_message(&mut self, msg: &super::message::SCPMessage<N>) {
        if let Some(peer_msg_queue) = self
            .in_memory_global_state
            .borrow_mut()
            .peer_msg_queues
            .get_mut(&self.peer_id)
        {
            peer_msg_queue.borrow_mut().add_message(msg.clone());
        } else {
            panic!(
                "Envelope controller for peer_id {} does not exist",
                self.peer_id,
            )
        }
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
    fn build(&self, peer_id: &PeerID) -> InMemoryConn<N> {
        InMemoryConn::new(peer_id.to_string(), &self.global_state)
    }
}
