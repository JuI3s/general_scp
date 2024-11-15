use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    herder::{self, herder::HerderDriver},
    scp::{envelope::SCPEnvelopeController, nomination_protocol::NominationValue},
};

use super::{
    conn::PeerConn,
    in_memory_conn::InMemoryConn,
    in_memory_global::InMemoryGlobalState,
    message::{MessageController, SCPMessage},
    peer::{PeerID, SCPPeerState},
};

pub struct InMemoryPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub peer_idx: PeerID,
    pub message_controller: Rc<RefCell<MessageController<N>>>,
    pub peer_conns: BTreeMap<PeerID, InMemoryConn<N>>,
    scp_envelope_controller: SCPEnvelopeController<N>,
    herder: H,
    global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
}

impl<N, H> InMemoryPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub fn new(
        peer_idx: PeerID,
        herder: H,
        global_state: &Rc<RefCell<InMemoryGlobalState<N>>>,
    ) -> Self {
        let msg_queue = MessageController::new();
        global_state
            .borrow_mut()
            .peer_msg_queues
            .insert(peer_idx.to_string(), msg_queue.clone());

        Self {
            peer_idx,
            message_controller: msg_queue,
            herder,
            peer_conns: BTreeMap::new(),
            global_state: global_state.clone(),
            scp_envelope_controller: SCPEnvelopeController::new(),
        }
    }

    pub fn send_message(&mut self, peer_id: &PeerID, msg: &SCPMessage<N>) {
        if let Some(peer_conn) = self.peer_conns.get_mut(peer_id) {
            peer_conn.send_message(msg);
        }
    }

    pub fn add_connection(&mut self, peer_id: &PeerID) {
        let in_memory_conn = InMemoryConn::new(peer_id.clone(), &self.global_state);
        self.peer_conns.insert(peer_id.to_string(), in_memory_conn);
    }

    pub fn process_all_messages(&mut self) {
        while let Some(msg) = self.message_controller.borrow_mut().pop() {
            match msg {
                SCPMessage::SCP(scp_env) => {
                    let env_id = self.scp_envelope_controller.add_envelope(scp_env);
                    self.herder
                        .recv_scp_envelope(&env_id, &mut self.scp_envelope_controller);
                }
                SCPMessage::Hello(hello_env) => todo!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{mock::state::MockState, overlay::in_memory_global::InMemoryGlobalState};

    #[test]
    fn test_in_memory_peer_send_hello() {
        let global_state = InMemoryGlobalState::<MockState>::new();

    }
}
