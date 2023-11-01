use super::{ballot_protocol::BallotProtocol, nomination_protocol::NominationProtocol};

pub struct Slot {
    index: usize,
}

impl Slot {
    pub fn new(index: usize) -> Self {
        Slot {index: index}
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