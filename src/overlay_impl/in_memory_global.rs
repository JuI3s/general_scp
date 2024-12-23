use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    herder::herder::HerderDriver,
    overlay::{
        message::{MessageController, SCPMessage},
        peer::PeerID,
    },
    scp::nomination_protocol::NominationValue,
};

use super::in_memory_peer::InMemoryPeerNode;

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
    pub fn new_handle() -> Rc<RefCell<Self>> {
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

        println!("send_message to: {:?}", peer_id);
    }

    pub fn process_messages<H: HerderDriver<N> + 'static>(
        global_state: &Rc<RefCell<Self>>,
        peers: &mut BTreeMap<PeerID, InMemoryPeerNode<N, H>>,
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
            println!("process msg sent to: {:?}", peer_id);

            let processed_new_msg = peers
                .get_mut(&peer_id)
                .unwrap()
                .borrow_mut()
                .process_one_message();

            assert!(processed_new_msg);

            num_msg_processed += 1;
        }

        num_msg_processed
    }
}
