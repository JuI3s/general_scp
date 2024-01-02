use crate::{
    application::quorum::QuorumSet,
    scp::{nomination_protocol::NominationValue, scp_driver::SCPEnvelope, slot::SlotIndex},
};

use super::herder::HerderEnvelopeStatus;

pub trait PendingEnvelopeManager<N>
where
    N: NominationValue,
{
    fn envelope_status(&mut self, envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus;
    fn recv_scp_quorum_set(&mut self, quorum_set: &QuorumSet);
    fn recv_nomination_value(&mut self, value: &N);
    fn pop(&mut self, slot_index: &SlotIndex) -> Option<SCPEnvelope<N>>;
}
