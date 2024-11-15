use std::{cell::RefCell, rc::Rc};

use crate::{
    herder::herder::HerderDriver,
    overlay::{conn::PeerConn, peer::SCPPeerState},
    scp::scp::NodeID,
};

use super::state::{MockState, MockStateDriver};

pub struct MockPeer {
    id: NodeID,
    state: Rc<RefCell<SCPPeerState>>,
    herder: Rc<RefCell<MockStateDriver>>,
}

impl PeerConn<MockState> for MockPeer {

    fn send_message(&mut self, msg: &crate::overlay::message::SCPMessage<MockState>) {
        todo!()
    }
    
    fn send_hello(&mut self, envelope: crate::overlay::message::HelloEnvelope) {
        self.send_message(&crate::overlay::message::SCPMessage::Hello(envelope))
    }
    
    fn send_scp_msg(&mut self, envelope: crate::scp::envelope::SCPEnvelope<MockState>) {
        self.send_message(&crate::overlay::message::SCPMessage::SCP(envelope))
    }
}
