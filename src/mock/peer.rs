use std::{cell::RefCell, rc::Rc};

use crate::{
    herder::herder::HerderDriver,
    overlay::peer::{PeerConn, SCPPeerState},
    scp::scp::NodeID,
};

use super::state::{MockState, MockStateDriver};

pub struct MockPeer {
    id: NodeID,
    state: Rc<RefCell<SCPPeerState>>,
    herder: Rc<RefCell<MockStateDriver>>,
}

impl PeerConn<MockState, MockStateDriver> for MockPeer {

    fn peer_state(&mut self) -> &std::rc::Rc<std::cell::RefCell<SCPPeerState>> {
        &self.state
    }

    fn overlay_manager(
        &self,
    ) -> &std::rc::Rc<
        std::cell::RefCell<
            dyn crate::overlay::overlay_manager::OverlayManager<
                MockState,
                MockStateDriver,
                HP = std::rc::Rc<std::cell::RefCell<Self>>,
                P = Self,
            >,
        >,
    > {
        todo!()
    }

    fn send_message(&mut self, msg: &crate::overlay::message::SCPMessage<MockState>) {
        todo!()
    }

    fn herder(&self) -> Rc<RefCell<MockStateDriver>> {
        self.herder.clone()
    }
}
