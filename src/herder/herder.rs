use std::{ops::Deref, time::Duration};

use crate::scp::{
    nomination_protocol::{HNominationValue, NominationValue, NominationValueSet},
    scp::SCPEnvelope,
    scp_driver::ValidationLevel,
};

pub trait HerderDriver {
    // Needs to be implemented by the specific consensus protocol for application level checks.
    fn validate_value(&self, value: &NominationValue, nomination: bool) -> ValidationLevel {
        // TODO: evaluates to true for every value for now.
        ValidationLevel::FullyValidated
    } 
    
    fn combine_candidates(&self, candidates: &NominationValueSet) -> Option<NominationValue>;
    fn emit_envelope(&self, envelope: &SCPEnvelope);

    fn nominating_value(&self, value: &NominationValue, slot_index: &u64) {}

    fn extract_valid_value(&self, value: &NominationValue) -> Option<NominationValue> {
        // TODO: assume input value is always valid and just return the input value for now.
        Some(value.to_owned())
    }

    fn compute_timeout(&self, round_number: u64) -> Duration {
        const MAX_TIMEOUT_SECONDS: u64 = 30 * 60;

        if round_number > MAX_TIMEOUT_SECONDS {
            Duration::from_secs(MAX_TIMEOUT_SECONDS)
        } else {
            Duration::from_secs(round_number)
        }
    }
}

struct HerderSCPDriver {}

impl HerderDriver for HerderSCPDriver {
    fn combine_candidates(&self, candidates: &NominationValueSet) -> Option<NominationValue> {
        // For now, just return the first element if there is any...
        // TODO: I think the actual implementation should depend on the specific use cases. For example, the combine_candidates function for a ledger built on top of SCP should be very different than that used for certificate security.e

        candidates.first().map(|val| val.deref().clone())
    }

    fn emit_envelope(&self, envelope: &SCPEnvelope) {
        todo!()
    }
}
