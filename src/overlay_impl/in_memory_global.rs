use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use syn::token::Ref;

use crate::{herder::herder::HerderDriver, overlay::{message::{MessageController, SCPMessage}, peer::PeerID}, scp::nomination_protocol::NominationValue};

use super::{
    in_memory_peer::InMemoryPeerNode,
};

pub struct InMemoryGlobalState<N>
where
    N: NominationValue,
{
    pub msg_peer_id_queue: VecDeque<PeerID>,
    pub peer_msg_queues: HashMap<PeerID, Rc<RefCell<MessageController<N>>>>,
}

impl<N> InMemoryGlobalState<N>
where
    N: NominationValue,
{
    pub fn new() -> Rc<RefCell<Self>> {
        let state = Self {
            peer_msg_queues: Default::default(),
            msg_peer_id_queue: Default::default(),
        };
        Rc::new(RefCell::new(state))
    }

    pub fn send_message(&mut self, peer_id: PeerID, msg: SCPMessage<N>) {
        self.msg_peer_id_queue.push_back(peer_id.clone());
        self.peer_msg_queues
            .get(&peer_id)
            .unwrap()
            .as_ref()
            .borrow_mut()
            .add_message(msg);
    }

    pub fn process_messages<H: HerderDriver<N> + 'static>(
        global_state: &Rc<RefCell<Self>>,
        peers: &mut HashMap<PeerID, Rc<RefCell<InMemoryPeerNode<N, H>>>>,
    ) -> usize {
        let mut num_msg_processed = 0;

        loop {
            let peer_id = global_state
                .as_ref()
                .borrow_mut()
                .msg_peer_id_queue
                .pop_front();
            if peer_id.is_none() {
                break;
            }
            let peer_id = peer_id.unwrap();

            peers
                .get(&peer_id)
                .unwrap()
                .as_ref()
                .borrow_mut()
                .process_one_message();

            num_msg_processed += 1;
        }

        num_msg_processed
    }
}
