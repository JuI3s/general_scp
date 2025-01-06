use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    application::quorum::{HQuorumSet, QuorumSet},
    scp::{
        envelope::SCPEnvelope,
        nomination_protocol::{NominationValue, SCPNominationValue},
        scp_driver::{HashValue, ValidationLevel},
        slot::SlotIndex,
        statement::SCPStatement,
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

pub trait HerderBuilder<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    fn build(&self) -> H;
}

pub trait HerderDriver<N>
where
    N: NominationValue,
{
    fn new() -> Self;

    // Needs to be implemented by the specific consensus protocol for application
    // level checks.
    fn validate_value(&self, value: &N, nomination: bool) -> ValidationLevel {
        // TODO: evaluates to true for every value for now.
        ValidationLevel::FullyValidated
    }

    fn combine_candidates(&self, candidates: &BTreeSet<Arc<N>>) -> Option<N>;
    fn emit_envelope(&self, envelope: &SCPEnvelope<N>) {}

    fn nominating_value(&self, value: &N, slot_index: &SlotIndex) {}

    fn extract_valid_value(&self, value: &N) -> Option<N>;
    // {
    // TODO: assume input value is always valid and just return the input value for
    // now. Some(value.to_owned())
    // }

    fn compute_timeout(&self, round_number: u64) -> Duration {
        const MAX_TIMEOUT_SECONDS: u64 = 30 * 60;

        if round_number > MAX_TIMEOUT_SECONDS {
            Duration::from_secs(MAX_TIMEOUT_SECONDS)
        } else {
            Duration::from_secs(round_number)
        }
    }
}
