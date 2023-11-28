use std::{ops::Deref, time::Duration};

use crate::scp::{
    nomination_protocol::{HNominationValue, NominationValue, NominationValueSet},
    scp::SCPEnvelope,
};

pub trait HerderDriver {
    fn combine_candidates(&self, candidates: &NominationValueSet) -> Option<NominationValue>;
    fn emit_envelope(&self, envelope: &SCPEnvelope);
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
