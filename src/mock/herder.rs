use crate::herder::herder::HerderDriver;

use super::state::MockState;

pub struct MockHerder {}

impl HerderDriver<MockState> for MockHerder {
    fn combine_candidates(
        &self,
        candidates: &std::collections::BTreeSet<std::sync::Arc<MockState>>,
    ) -> Option<MockState> {
        todo!()
    }

    fn emit_envelope(&self, envelope: &crate::scp::scp_driver::SCPEnvelope<MockState>) {
        todo!()
    }

    fn extract_valid_value(&self, value: &MockState) -> Option<MockState> {
        todo!()
    }

    fn get_quorum_set(
        &self,
        statement: &crate::scp::statement::SCPStatement<MockState>,
    ) -> Option<crate::application::quorum::HQuorumSet> {
        todo!()
    }

    fn recv_scp_envelope(&mut self, envelope: &crate::scp::scp_driver::SCPEnvelope<MockState>) {
        todo!()
    }
}
