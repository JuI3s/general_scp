use std::collections::BTreeMap;

use crate::{
    application::quorum::QuorumSet,
    scp::{nomination_protocol::NominationValue, scp_driver::SCPEnvelope, slot::SlotIndex},
};

use super::{herder::HerderEnvelopeStatus, pending_envelope_manager::PendingEnvelopeManager};

pub struct PendingEnvelopeNoFetchingManager<N>
where
    N: NominationValue,
{
    ready_envelopes: BTreeMap<SlotIndex, Vec<SCPEnvelope<N>>>,
}

impl<N> PendingEnvelopeManager<N> for PendingEnvelopeNoFetchingManager<N>
where
    N: NominationValue,
{
    fn envelope_status(&mut self, _envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus {
        HerderEnvelopeStatus::EnvelopeStatusReady
    }

    fn recv_scp_quorum_set(&mut self, _quorum_set: &QuorumSet) {}

    fn recv_nomination_value(&mut self, _value: &N) {}

    fn pop(&mut self, slot_index: &SlotIndex) -> Option<SCPEnvelope<N>> {
        let slot_envelopes = self.ready_envelopes.get_mut(slot_index)?;
        slot_envelopes.pop()
    }
}
