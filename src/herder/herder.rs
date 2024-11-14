use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use crate::{
    application::quorum::HQuorumSet,
    scp::{
        envelope::{self, SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
        nomination_protocol::{NominationValue, SCPNominationValue, SCPNominationValueSet},
        scp_driver::{HashValue, ValidationLevel},
        slot::SlotIndex,
        statement::{HSCPStatement, SCPStatement},
    },
};

pub enum HerderEnvelopeStatus {
    // for some reason this envelope was discarded - either it was invalid,
    // used unsane qset or was coming from node that is not in quorum
    EnvelopeStatusDiscarded,
    EnvelopeStatusSkippedSelf,
    EnvelopeStatusProcessed,
    EnvelopeStatusFetching,
    EnvelopeStatusReady,
}

pub trait HerderDriver<N>
where
    N: NominationValue,
{
    // Needs to be implemented by the specific consensus protocol for application
    // level checks.
    fn validate_value(&self, value: &N, nomination: bool) -> ValidationLevel {
        // TODO: evaluates to true for every value for now.
        ValidationLevel::FullyValidated
    }

    fn combine_candidates(&self, candidates: &BTreeSet<Arc<N>>) -> Option<N>;
    fn emit_envelope(&self, envelope: &SCPEnvelope<N>);

    fn nominating_value(&self, value: &N, slot_index: &SlotIndex) {}

    fn extract_valid_value(&self, value: &N) -> Option<N>;
    // {
    // TODO: assume input value is always valid and just return the input value for
    // now. Some(value.to_owned())
    // }

    fn get_quorum_set(&self, statement: &SCPStatement<N>) -> Option<HQuorumSet>;

    fn recv_scp_envelope(
        this: &Rc<RefCell<Self>>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
    );

    fn compute_timeout(&self, round_number: u64) -> Duration {
        const MAX_TIMEOUT_SECONDS: u64 = 30 * 60;

        if round_number > MAX_TIMEOUT_SECONDS {
            Duration::from_secs(MAX_TIMEOUT_SECONDS)
        } else {
            Duration::from_secs(round_number)
        }
    }
}

struct HerderSCPDriver {
    quorum_set_map: BTreeMap<HashValue, HQuorumSet>,
}

impl HerderDriver<SCPNominationValue> for HerderSCPDriver {
    fn emit_envelope(&self, envelope: &SCPEnvelope<SCPNominationValue>) {
        todo!()
    }

    fn get_quorum_set(&self, statement: &SCPStatement<SCPNominationValue>) -> Option<HQuorumSet> {
        self.quorum_set_map
            .get(&statement.quorum_set_hash_value())
            .map(|val| val.clone())
    }

    fn extract_valid_value(&self, value: &SCPNominationValue) -> Option<SCPNominationValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        candidates: &BTreeSet<Arc<SCPNominationValue>>,
    ) -> Option<SCPNominationValue> {
        todo!()
    }

    fn recv_scp_envelope(
        this: &Rc<RefCell<Self>>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<SCPNominationValue>,
    ) {
        todo!()
    }
}
