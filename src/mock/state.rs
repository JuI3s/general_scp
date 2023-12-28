use std::{collections::BTreeMap, sync::Arc};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    application::quorum::HQuorumSet,
    herder::herder::HerderDriver,
    scp::{
        ballot_protocol::HBallotProtocolState,
        nomination_protocol::{HNominationProtocolState, NominationProtocol, NominationValue},
        scp_driver::{HashValue, SlotDriver},
    },
};

// Just hold a vector u8 integers.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
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

pub struct MockStateDriver {
    quorum_set_map: BTreeMap<HashValue, HQuorumSet>,
}

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
        self.quorum_set_map
            .get(&statement.quorum_set_hash_value())
            .map(|val| val.clone())
    }

    fn validate_value(
        &self,
        value: &MockState,
        nomination: bool,
    ) -> crate::scp::scp_driver::ValidationLevel {
        // TODO: evaluates to true for every value for now.
        crate::scp::scp_driver::ValidationLevel::FullyValidated
    }

    fn nominating_value(&self, value: &MockState, slot_index: &u64) {}

    fn compute_timeout(&self, round_number: u64) -> std::time::Duration {
        const MAX_TIMEOUT_SECONDS: u64 = 30 * 60;

        if round_number > MAX_TIMEOUT_SECONDS {
            std::time::Duration::from_secs(MAX_TIMEOUT_SECONDS)
        } else {
            std::time::Duration::from_secs(round_number)
        }
    }
}

impl MockStateDriver {}

impl Default for MockStateDriver {
    fn default() -> Self {
        Self {
            quorum_set_map: Default::default(),
        }
    }
}

impl MockStateDriver {}

#[cfg(test)]
mod tests {

    use std::sync::Mutex;

    use crate::{
        application::{clock::VirtualClock, quorum::QuorumSet, work_queue::EventQueue},
        scp::{
            local_node::LocalNode,
            local_node_builder::LocalNodeBuilder,
            scp::NodeID,
            scp_driver_builder::{SlotDriverBuilder, SlotTimerBuilder},
        },
    };

    use super::*;

    #[test]
    fn slot_driver_builder() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let timer_handle = SlotTimerBuilder::new()
            .clock(virtual_clock.clone())
            .build()
            .unwrap();

        let quorum_set = QuorumSet::example_quorum_set();

        let local_node = LocalNodeBuilder::<MockState>::new()
            .is_validator(true)
            .quorum_set(quorum_set)
            .node_id(node_id)
            .build()
            .unwrap();

        let slot_driver = SlotDriverBuilder::<MockState, MockStateDriver>::new()
            .slot_index(0)
            .herder_driver(Default::default())
            .timer(timer_handle)
            .local_node(local_node)
            .build()
            .unwrap();
    }

    #[test]
    fn nominate() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let timer_handle = SlotTimerBuilder::new()
            .clock(virtual_clock.clone())
            .build()
            .unwrap();

        let quorum_set = QuorumSet::example_quorum_set();

        let local_node = LocalNodeBuilder::<MockState>::new()
            .is_validator(true)
            .quorum_set(quorum_set)
            .node_id(node_id)
            .build()
            .unwrap();

        let slot_driver = SlotDriverBuilder::<MockState, MockStateDriver>::new()
            .slot_index(0)
            .herder_driver(Default::default())
            .timer(timer_handle)
            .local_node(local_node)
            .build()
            .unwrap();

        let value = Arc::new(MockState::random());
        let prev_value = MockState::random();
        slot_driver.nominate(slot_driver.nomination_state().clone(), value, &prev_value);
    }
}
