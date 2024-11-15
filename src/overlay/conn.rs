use crate::scp::{envelope::SCPEnvelope, nomination_protocol::NominationValue};

use super::message::{HelloEnvelope, SCPMessage};

pub trait PeerConn<N>
where
    N: NominationValue,
    Self: Sized,
{
    // Implemented by struct implementing the trait.
    fn send_message(&mut self, msg: &SCPMessage<N>);

    fn send_hello(&mut self, envelope: HelloEnvelope) {
        self.send_message(&SCPMessage::Hello(envelope))
    }

    fn send_scp_msg(&mut self, envelope: SCPEnvelope<N>) {
        self.send_message(&SCPMessage::SCP(envelope))
    }
}
