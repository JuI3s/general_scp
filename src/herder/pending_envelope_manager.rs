use crate::{
    application::quorum::QuorumSet,
    scp::{
        envelope::{SCPEnvelope, SCPEnvelopeController},
        nomination_protocol::NominationValue,
        slot::SlotIndex,
    },
};

use super::herder::HerderEnvelopeStatus;

pub trait PendingEnvelopeManager<N>
where
    N: NominationValue,
{
    fn envelope_status(&mut self, envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus;
    fn recv_scp_quorum_set(
        &mut self,
        quorum_set: &QuorumSet,
        envelope_controller: &mut SCPEnvelopeController<N>,
    );
    fn recv_nomination_value(
        &mut self,
        value: &N,
        envelope_controller: &mut SCPEnvelopeController<N>,
    );
    fn pop(&mut self, slot_index: &SlotIndex) -> Option<SCPEnvelope<N>>;
}
