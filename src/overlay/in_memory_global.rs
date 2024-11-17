use std::{cell::RefCell, collections::HashMap, rc::Rc};

use syn::token::Ref;

use crate::{herder::herder::HerderDriver, scp::nomination_protocol::NominationValue};

use super::{message::MessageController, peer::PeerID};

pub struct InMemoryGlobalState<N>
where
    N: NominationValue,
{
    pub peer_msg_queues: HashMap<PeerID, Rc<RefCell<MessageController<N>>>>,
}

impl<N> InMemoryGlobalState<N>
where
    N: NominationValue,
{
    pub fn new() -> Rc<RefCell<Self>> {
        let state = Self {
            peer_msg_queues: Default::default(),
        };
        Rc::new(RefCell::new(state))
    }
}

