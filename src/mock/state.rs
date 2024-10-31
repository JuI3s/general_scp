use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    application::{quorum::HQuorumSet, work_queue::WorkScheduler},
    crypto::types::Blake2Hashable,
    herder::herder::HerderDriver,
    scp::{
        self,
        ballot_protocol::HBallotProtocolState,
        envelope::{MakeEnvelope, SCPEnvelope},
        local_node::{self, HLocalNode},
        nomination_protocol::{HNominationProtocolState, NominationProtocol, NominationValue},
        scp_driver::{HashValue, SlotDriver},
        scp_driver_builder::SlotDriverBuilder,
        slot::SlotIndex,
        statement::{MakeStatement, SCPStatementNominate},
    },
};

use super::scp_driver::MockSCPDriver;

// Just hold a vector u8 integers.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize, Debug)]
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
    pub scp_driver: MockSCPDriver,
    pub local_node: HLocalNode<MockState>,
    pub scheduler: WorkScheduler,
}

impl Into<Rc<RefCell<MockStateDriver>>> for MockStateDriver {
    fn into(self) -> Rc<RefCell<MockStateDriver>> {
        RefCell::new(self).into()
    }
}

impl MakeStatement<MockState> for MockStateDriver {
    fn new_nominate_statement(&self) -> crate::scp::statement::SCPStatementNominate<MockState> {
        SCPStatementNominate::<MockState>::new(&self.local_node.borrow().quorum_set)
    }
}

impl MakeEnvelope<MockState> for MockStateDriver {
    fn new_nomination_envelope(&self, slot_index: usize) -> SCPEnvelope<MockState> {
        let statement = self.new_nominate_statement();
        SCPEnvelope::<MockState>::new(
            scp::statement::SCPStatement::Nominate(statement),
            self.local_node.borrow().node_id.clone(),
            slot_index.try_into().unwrap(),
            self.signature_for_testing(),
        )
    }
}

impl MockStateDriver {
    pub fn new(local_node: HLocalNode<MockState>, schedular: WorkScheduler) -> Rc<RefCell<Self>> {
        Self {
            quorum_set_map: Default::default(),
            scp_driver: Default::default(),
            local_node: local_node,
            scheduler: schedular,
        }
        .into()
    }

    fn signature_for_testing(&self) -> [u8; 64] {
        [0; 64]
    }

    pub fn new_slot(
        this: &Rc<RefCell<Self>>,
        slot_index: SlotIndex,
    ) -> Option<SlotDriver<MockState, MockStateDriver>> {
        SlotDriverBuilder::<MockState, MockStateDriver>::new()
            .slot_index(slot_index)
            .local_node(this.borrow().local_node.clone())
            .timer(this.borrow().scheduler.clone())
            .herder_driver(this.to_owned())
            .build()
            .ok()
    }

    fn get_or_create_slot(
        this: &Rc<RefCell<Self>>,
        slot_index: &SlotIndex,
    ) -> Arc<SlotDriver<MockState, MockStateDriver>> {
        let slot = this.borrow_mut().scp_driver.slots.get(slot_index).cloned();
        if slot.is_some() {
            return slot.unwrap();
        }

        println!("create slot");

        let slot: Arc<SlotDriver<MockState, MockStateDriver>> =
            Self::new_slot(this, slot_index.to_owned()).unwrap().into();
        this.borrow_mut()
            .scp_driver
            .slots
            .insert(slot_index.to_owned(), slot.clone());
        slot
    }
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

    fn emit_envelope(&self, envelope: &SCPEnvelope<MockState>) {}

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

    fn nominating_value(&self, value: &MockState, slot_index: &SlotIndex) {}

    fn compute_timeout(&self, round_number: SlotIndex) -> std::time::Duration {
        const MAX_TIMEOUT_SECONDS: SlotIndex = 30 * 60;

        if round_number > MAX_TIMEOUT_SECONDS {
            std::time::Duration::from_secs(MAX_TIMEOUT_SECONDS)
        } else {
            std::time::Duration::from_secs(round_number)
        }
    }

    fn recv_scp_envelope(this: &Rc<RefCell<Self>>, envelope: &SCPEnvelope<MockState>) {
        let slot = Self::get_or_create_slot(this, &envelope.slot_index);
        // slot.
        slot.recv_scp_envelvope(envelope);
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::BTreeSet, sync::Mutex};

    use crate::{
        application::{clock::VirtualClock, quorum::QuorumSet, work_queue::EventQueue},
        overlay::loopback_peer::{LoopbackPeer, LoopbackPeerConnection},
        scp::{
            local_node::LocalNode,
            local_node_builder::LocalNodeBuilder,
            scp::NodeID,
            scp_driver_builder::{SlotDriverBuilder, SlotTimerBuilder},
        },
    };

    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::work_queue::WorkScheduler,
        mock::state::{MockState, MockStateDriver},
        overlay::message::HelloEnvelope,
        overlay::peer::SCPPeer,
    };

    use super::*;

    use backtrace::Backtrace;

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

        let state_driver = MockStateDriver::new(local_node.clone(), timer_handle.clone());

        let slot_driver = SlotDriverBuilder::<MockState, MockStateDriver>::new()
            .slot_index(0)
            .herder_driver(state_driver)
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
            .herder_driver(MockStateDriver::new(
                local_node.clone(),
                timer_handle.clone(),
            ))
            .timer(timer_handle)
            .local_node(local_node)
            .build_handle()
            .unwrap();

        let value = Arc::new(MockState::random());
        let prev_value = MockState::random();
        slot_driver.nominate(slot_driver.nomination_state().clone(), value, &prev_value);
    }

    #[test]
    fn build_mock_herder() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let mut leaders: BTreeSet<NodeID> = BTreeSet::new();
        leaders.insert(node_id.clone());

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
            .local_node(local_node.clone())
            .timer(timer_handle.clone())
            .herder_driver(MockStateDriver::new(
                local_node.clone(),
                timer_handle.clone(),
            ))
            .build()
            .unwrap();

        // slot_driver.recv_scp_envelvope(envelope)
    }

    fn create_test_herder(node_index: u64) -> Rc<RefCell<MockStateDriver>> {
        let node_id: NodeID = "node".to_string() + node_index.to_string().as_ref();
        let virtual_clock = VirtualClock::new_clock();

        let mut leaders: BTreeSet<NodeID> = BTreeSet::new();
        leaders.insert(node_id.clone());

        let timer_handle = SlotTimerBuilder::new()
            .clock(virtual_clock.clone())
            .build()
            .unwrap();

        let quorum_set = QuorumSet::example_quorum_set();

        let local_node: Rc<RefCell<LocalNode<MockState>>> = LocalNodeBuilder::<MockState>::new()
            .is_validator(true)
            .quorum_set(quorum_set)
            .node_id(node_id)
            .build()
            .unwrap();

        let herder = MockStateDriver::new(local_node.clone(), timer_handle.clone());
        herder
    }

    #[test]
    fn loopback_peer_send_hello_message() {
        let herder1 = create_test_herder(1);
        let herder2 = create_test_herder(2);

        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::default()));
        let connection = LoopbackPeerConnection::<MockState, MockStateDriver>::new(
            &work_scheduler,
            herder1,
            herder2,
        );
        let msg = HelloEnvelope {};

        connection.initiator.borrow_mut().send_hello(msg.clone());

        assert_eq!(connection.acceptor.borrow_mut().in_queue.len(), 1);
        LoopbackPeer::<MockState, MockStateDriver>::process_in_queue(&connection.acceptor);
        assert_eq!(connection.acceptor.borrow_mut().in_queue.len(), 0);

        connection.initiator.borrow_mut().send_hello(msg.clone());
        connection.initiator.borrow_mut().send_hello(msg.clone());
        assert_eq!(connection.initiator.borrow_mut().in_queue.len(), 0);

        work_scheduler.borrow().excecute_main_thread_tasks();
        assert_eq!(connection.acceptor.borrow_mut().in_queue.len(), 0);
    }

    #[test]
    fn loopback_peer_nominate() {
        let herder1 = create_test_herder(1);
        let herder2 = create_test_herder(2);

        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::default()));
        let connection = LoopbackPeerConnection::<MockState, MockStateDriver>::new(
            &work_scheduler,
            herder1,
            herder2,
        );

        let value = Arc::new(MockState::random());
        let prev_value = MockState::random();

        // Make a nomination statement.
        let nominate_statement = connection.initiator.borrow().new_nominate_statement();

        let envelope: SCPEnvelope<MockState> =
            connection.initiator.borrow().new_nomination_envelope(0);

        println!("{:?}", envelope);
        connection.initiator.borrow_mut().send_scp_msg(envelope);

        let bt = Backtrace::new();

        LoopbackPeer::process_in_queue(&connection.acceptor);

        // println!("{:?}", bt);

        // connection.initiator.borrow_mut().send_scp_msg(envelope);
        // Creating nomination envelope and pass to loopback peer.
    }
}
