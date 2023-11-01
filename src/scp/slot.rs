use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::overlay::peer::PeerID;

use super::{
    ballot_protocol::{BallotProtocol, BallotProtocolState},
    nomination_protocol::{NominationProtocol, NominationProtocolState},
    scp::SCPEnvelope,
};

pub struct Slot {
    index: usize,
    nomination_state: NominationProtocolState,
    ballot_state: BallotProtocolState,
}

pub type HSCPEnvelope = Arc<Mutex<SCPEnvelope>>;

impl Slot {
    pub fn new(index: usize) -> Self {
        Slot {
            index: index,
            nomination_state: NominationProtocolState::default(),
            ballot_state: BallotProtocolState::default(),
        }
    }
}

impl NominationProtocol for Slot {
    fn nominate(&mut self) {
        print!("Nominating a value");
    }

    fn recv_nomination_msg(&mut self) {
        todo!()
    }
}

impl BallotProtocol for Slot {
    fn externalize(&mut self) {
        todo!()
    }

    fn recv_ballot_envelope(&mut self) {
        todo!()
    }
}
