use crate::{
    application::work_queue::WorkScheduler,
    herder::herder::HerderDriver,
    scp::{local_node::HLocalNode, scp_driver::SlotDriver, slot::SlotIndex},
};

use super::{scp_driver::MockSCPDriver, state::MockState};

pub struct MockHerder {
    pub scp_driver: MockSCPDriver,
    pub local_node: HLocalNode<MockState>,
    pub scheduler: WorkScheduler,
}

impl MockHerder {
    // fn new_slot(&self, slot_index: SlotIndex) -> SlotDriver<MockState> {
    //     SlotDriver::<MockState>::new(slot_index, self.local_node.clone(),
    // self.scheduler.clone(), Default::default(), Default::default(),
    // herder_driver) }
}

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
        // self.scp_driver
    }
}
