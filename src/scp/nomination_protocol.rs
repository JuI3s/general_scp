use super::slot::Slot;

pub trait NominationProtocol {
    fn nominate(&mut self);
    fn recv_nomination_msg(&mut self);
}