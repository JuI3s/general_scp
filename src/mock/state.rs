use rand::Rng;

use crate::{herder::herder::HerderDriver, scp::nomination_protocol::NominationValue};

// Just hold a vector u8 integers.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct MockState(Vec<[u8; 32]>);

impl MockState {
    pub fn empty() -> Self {
        MockState(Default::default())
    }

    pub fn random() -> Self {
        let mut vec: Vec<[u8; 32]> = Default::default();
        for _ in 0..3 {
            let mut e = [0u8; 32];
            rand::thread_rng().fill(&mut e[..]);
            vec.push(e);
        }

        // Generate a random sample containing a vector of size 3.
        Self(vec)
    }
}

impl Default for MockState {
    fn default() -> Self {
        Self::random()
    }
}

impl NominationValue for MockState {}

pub struct MockStateDriver {}

impl HerderDriver<MockState> for MockStateDriver {
    fn combine_candidates(
        &self,
        candidates: &std::collections::BTreeSet<std::sync::Arc<MockState>>,
    ) -> Option<MockState> {
        let mut state = MockState::default();

        for candidate in candidates {
            for ele in &candidate.0 {
                state.0.push(*ele);
            }
        }

        Some(state)
    }

    fn emit_envelope(&self, envelope: &crate::scp::scp_driver::SCPEnvelope<MockState>) {}

    fn extract_valid_value(&self, value: &MockState) -> Option<MockState> {
        Some(value.clone())
    }

    fn get_quorum_set(
        &self,
        statement: &crate::scp::statement::SCPStatement<MockState>,
    ) -> Option<crate::application::quorum::HQuorumSet> {
        todo!()
    }
}
