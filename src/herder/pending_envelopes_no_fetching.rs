use std::collections::BTreeMap;

use crate::{
    application::quorum::QuorumSet,
    scp::{envelope::SCPEnvelope, nomination_protocol::NominationValue, slot::SlotIndex},
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
    fn envelope_status(&mut self, envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus {
        let slot_envelopes = self.ready_envelopes.entry(envelope.slot_index).or_default();
        slot_envelopes.push(envelope.to_owned());

        HerderEnvelopeStatus::EnvelopeStatusReady
    }

    fn pop(&mut self, slot_index: &SlotIndex) -> Option<SCPEnvelope<N>> {
        let slot_envelopes = self.ready_envelopes.get_mut(slot_index)?;
        slot_envelopes.pop()
    }
    
    fn recv_scp_quorum_set(
        &mut self,
        quorum_set: &QuorumSet,
        envelope_controller: &mut crate::scp::envelope::SCPEnvelopeController<N>,
    ) {
        todo!()
    }
    
    fn recv_nomination_value(
        &mut self,
        value: &N,
        envelope_controller: &mut crate::scp::envelope::SCPEnvelopeController<N>,
    ) {
        todo!()
    }
}
