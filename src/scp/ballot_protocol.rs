pub trait BallotProtocol {
    fn externalize(&mut self);
    fn recv_ballot_envelope(&mut self);
}