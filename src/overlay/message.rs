use std::{cell::RefCell, collections::VecDeque, rc::Rc, str};

use bincode::de;
use serde::Serialize;

use crate::{
    crypto::types::Blake2Hashable,
    scp::{envelope::SCPEnvelope, nomination_protocol::NominationValue},
};

#[derive(Clone, Serialize)]
pub enum SCPMessage<N>
where
    N: NominationValue,
{
    SCP(SCPEnvelope<N>),
    Hello(HelloEnvelope),
}

impl<N> Blake2Hashable for SCPMessage<N> where N: NominationValue {}

impl<N> SCPMessage<N>
where
    N: NominationValue,
{
    pub fn is_boardcast_msg(&self) -> bool {
        true
    }
}

#[derive(Serialize, Clone)]
pub struct HelloEnvelope {}

pub struct MessageController<N>
where
    N: NominationValue,
{
    pub messages: VecDeque<SCPMessage<N>>,
}

impl<N> MessageController<N>
where
    N: NominationValue,
{
    pub fn new() -> Rc<RefCell<Self>> {
        let msg_queue = Self {
            messages: Default::default(),
        }
        Rc::new(RefCell::new(msg_queue))
    }

    pub fn add_message(&mut self, msg: SCPMessage<N>) {
        self.messages.push_back(msg);
    }


    pub fn pop(&mut self ) -> Option<SCPMessage<N>> {

        self.messages.pop_front()
    }
}
