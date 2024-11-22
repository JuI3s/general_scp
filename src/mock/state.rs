use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    application::{quorum::HQuorumSet, work_queue::WorkScheduler},
    crypto::types::Blake2Hashable,
    herder::herder::{HerderBuilder, HerderDriver},
    scp::{
        self,
        ballot_protocol::HBallotProtocolState,
        envelope::{MakeEnvelope, SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
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

#[derive(Clone)]
pub struct MockStateDriver {
    quorum_set_map: BTreeMap<HashValue, HQuorumSet>,
}

impl MockStateDriver {
    pub fn new() -> Self {
        Self {
            quorum_set_map: Default::default(),
        }
    }
}

pub struct MockStateDriverBuilder {}

impl HerderBuilder<MockState, MockStateDriver> for MockStateDriverBuilder {
    fn build(&self) -> MockStateDriver {
        MockStateDriver::new()
    }
}

impl Into<Rc<RefCell<MockStateDriver>>> for MockStateDriver {
    fn into(self) -> Rc<RefCell<MockStateDriver>> {
        RefCell::new(self).into()
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
}

#[cfg(test)]
mod tests {

    use env_logger;
    use log::debug;
    use scp::{
        ballot_protocol::BallotProtocolState, envelope::SCPEnvelopeController,
        nomination_protocol::NominationProtocolState,
    };
    use std::{
        collections::{BTreeSet, HashMap},
        sync::Mutex,
    };

    use crate::{
        application::{clock::VirtualClock, quorum::QuorumSet, work_queue::EventQueue},
        overlay::{
            in_memory_global::InMemoryGlobalState,
            in_memory_peer::{
                test_data_create_mock_in_memory_nodes, test_data_create_mock_state_local_node_info,
                InMemoryPeerBuilder,
            },
            loopback_peer::{LoopbackPeer, LoopbackPeerConnection},
            message::SCPMessage,
            node,
        },
        scp::{
            local_node::LocalNodeInfo, local_node_builder::LocalNodeBuilder, scp::NodeID,
            scp_driver_builder::SlotDriverBuilder,
        },
    };

    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::work_queue::WorkScheduler,
        mock::state::{MockState, MockStateDriver},
        overlay::message::HelloEnvelope,
    };

    use super::*;

    use backtrace::Backtrace;

    #[test]
    fn slot_driver_builder() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let timer_handle = WorkScheduler::new(None);
        let quorum_set = QuorumSet::example_quorum_set();

        let local_node = LocalNodeBuilder::<MockState>::new()
            .is_validator(true)
            .quorum_set(quorum_set)
            .node_id(node_id)
            .build()
            .unwrap();

        let state_driver = MockStateDriver::new();

        let slot_driver = SlotDriverBuilder::<MockState, MockStateDriver>::new()
            .slot_index(0)
            .herder_driver(Rc::new(RefCell::new(state_driver)))
            .timer(Rc::new(RefCell::new(timer_handle)))
            .local_node(local_node)
            .build()
            .unwrap();
    }

    #[test]
    fn nominate() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let timer_handle = WorkScheduler::new(None);

        let quorum_set = QuorumSet::example_quorum_set();

        let local_node = LocalNodeBuilder::<MockState>::new()
            .is_validator(true)
            .quorum_set(quorum_set)
            .node_id(node_id.clone())
            .build()
            .unwrap();

        let slot_driver: Arc<SlotDriver<MockState, MockStateDriver>> =
            SlotDriverBuilder::<MockState, MockStateDriver>::new()
                .slot_index(0)
                .herder_driver(Rc::new(RefCell::new(MockStateDriver::new())))
                .timer(Rc::new(RefCell::new(timer_handle)))
                .local_node(local_node)
                .build_handle()
                .unwrap();

        let value = Arc::new(MockState::random());
        let prev_value = MockState::random();
        let mut envelope_controller = SCPEnvelopeController::<MockState>::new();

        let mut nomination_state = NominationProtocolState::new(node_id.clone());
        let mut ballot_state = BallotProtocolState::default();

        slot_driver.nominate(
            &mut nomination_state,
            &mut ballot_state,
            value,
            &prev_value,
            &mut envelope_controller,
        );
    }

    #[test]
    fn build_mock_herder() {
        let node_id: NodeID = "node1".into();
        let virtual_clock = VirtualClock::new_clock();

        let mut leaders: BTreeSet<NodeID> = BTreeSet::new();
        leaders.insert(node_id.clone());

        let timer_handle = WorkScheduler::new(None);
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
            .timer(Rc::new(RefCell::new(timer_handle)))
            .herder_driver(Rc::new(RefCell::new(MockStateDriver::new())))
            .build()
            .unwrap();

        // slot_driver.recv_scp_envelvope(envelope)
    }

    fn create_test_herder(node_index: u64) -> Rc<RefCell<MockStateDriver>> {
        let node_id: NodeID = "node".to_string() + node_index.to_string().as_ref();
        let virtual_clock = VirtualClock::new_clock();

        let mut leaders: BTreeSet<NodeID> = BTreeSet::new();
        leaders.insert(node_id.clone());

        let timer_handle = WorkScheduler::new(None);
        let quorum_set = QuorumSet::example_quorum_set();

        let local_node: Rc<RefCell<LocalNodeInfo<MockState>>> =
            LocalNodeBuilder::<MockState>::new()
                .is_validator(true)
                .quorum_set(quorum_set)
                .node_id(node_id)
                .build()
                .unwrap();

        let herder = Rc::new(RefCell::new(MockStateDriver::new()));
        herder
    }

    fn set_up_test_nodes() -> (
        Rc<
            RefCell<
                crate::overlay::peer_node::PeerNode<
                    MockState,
                    MockStateDriver,
                    crate::overlay::in_memory_conn::InMemoryConn<MockState>,
                    crate::overlay::in_memory_conn::InMemoryConnBuilder<MockState>,
                >,
            >,
        >,
        Rc<
            RefCell<
                crate::overlay::peer_node::PeerNode<
                    MockState,
                    MockStateDriver,
                    crate::overlay::in_memory_conn::InMemoryConn<MockState>,
                    crate::overlay::in_memory_conn::InMemoryConnBuilder<MockState>,
                >,
            >,
        >,
        HashMap<
            String,
            Rc<
                RefCell<
                    crate::overlay::peer_node::PeerNode<
                        MockState,
                        MockStateDriver,
                        crate::overlay::in_memory_conn::InMemoryConn<MockState>,
                        crate::overlay::in_memory_conn::InMemoryConnBuilder<MockState>,
                    >,
                >,
            >,
        >,
        InMemoryPeerBuilder<MockState, MockStateDriver, MockStateDriverBuilder>,
    ) {
        let herder_builder = MockStateDriverBuilder {};
        let mut node_builder = InMemoryPeerBuilder::new(herder_builder);
        let (node1, node2) = test_data_create_mock_in_memory_nodes(&mut node_builder);
        let mut peers = HashMap::new();
        peers.insert(node1.borrow().peer_idx.clone(), node1.clone());
        peers.insert(node2.borrow().peer_idx.clone(), node2.clone());

        (node1, node2, peers, node_builder)
    }

    #[test]
    fn in_memory_pget_latest_messageeer_send_hello_message() {
        let (node1, node2, mut peers, node_builder) = set_up_test_nodes();

        node1.borrow_mut().send_hello(&node2.borrow().peer_idx);

        assert_eq!(
            InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
            2,
        );
    }

    #[test]
    fn in_memory_peer_nominate() {
        let (node1, node2, mut peers, node_builder) = set_up_test_nodes();

        node1.borrow_mut().send_hello(&node2.borrow().peer_idx);
        assert_eq!(
            InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
            2,
        );

        node1.borrow_mut().slot_nominate(0);
        // node1.borrow_mut().nominate(&node2.borrow().peer_idx, 0);
        assert_eq!(
            InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
            1,
        );
    }

    //     #[test]
    //     fn loopback_peer_nominate() {
    //         env_logger::init();

    //         let herder1 = create_test_herder(1);
    //         let herder2 = create_test_herder(2);

    //         let work_scheduler = Rc::new(RefCell::new(WorkScheduler::default()));
    //         let connection = LoopbackPeerConnection::<MockState, MockStateDriver>::new(
    //             &work_scheduler,
    //             herder1,
    //             herder2,
    //         );

    //         let value = MockState::random();
    //         let prev_value = MockState::random();

    //         // Make a nomination statement.

    //         let envelope: SCPEnvelope<MockState> = connection
    //             .initiator
    //             .borrow()
    //             .new_nomination_envelope(0, value);

    //         println!("{:?}", envelope);
    //         connection.initiator.borrow_mut().send_scp_msg(envelope);

    //         LoopbackPeer::process_in_queue(
    //             &connection.acceptor,
    //             &mut connection.acceptor_envs.borrow_mut(),
    //         );

    //         todo!();

    //         // println!("{:?}", bt);

    //         // connection.initiator.borrow_mut().send_scp_msg(envelope);
    //         // Creating nomination envelope and pass to loopback peer.
    //     }
}
