use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt::{Debug, Error},
    rc::Rc,
};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    application::quorum::{HQuorumSet, QuorumSet},
    herder::herder::{HerderBuilder, HerderDriver},
    scp::{
        envelope::SCPEnvelope, nomination_protocol::NominationValue, scp_driver::HashValue,
        slot::SlotIndex,
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

impl Debug for MockState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // https://stackoverflow.com/questions/28991050/how-to-iterate-a-vect-with-the-indexed-position
        let max_hex_to_display = 10;

        f.write_str("0x")?;
        f.write_str(
            &self
                .0
                .as_slice()
                // .as_flattened()
                .iter()
                .flatten()
                .enumerate()
                .take_while(|(idx, __)| *idx < max_hex_to_display)
                .map(|(_, b)| format!("{:02X}", b))
                .collect::<String>(),
        )
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
    // TODO:
    quorum_set_map: BTreeMap<HashValue, QuorumSet>,
}

impl MockStateDriver {
    pub fn new() -> Self {
        Self {
            quorum_set_map: Default::default(),
        }
    }
}

pub struct MockStateDriverBuilder {}

impl MockStateDriverBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

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

impl MockStateDriver {}

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

    fn emit_envelope(&self, envelope: &SCPEnvelope<MockState>) {
        // Emit broadcast envelope to all connected peers.

        todo!("emit_envelope");
    }

    fn extract_valid_value(&self, value: &MockState) -> Option<MockState> {
        Some(value.clone())
    }

    fn get_quorum_set(
        &self,
        statement: &crate::scp::statement::SCPStatement<MockState>,
    ) -> std::option::Option<&QuorumSet> {
        self.quorum_set_map.get(&statement.quorum_set_hash_value())
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

    use std::{
        collections::{BTreeSet, HashMap},
        f64::consts::E,
        sync::Arc,
        vec,
    };

    use syn::token::Default;
    use test_log::test;
    use tracing::info;

    use crate::{
        application::{clock::VirtualClock, quorum::QuorumSet, quorum_manager::QuorumManager},
        mock::{
            self,
            builder::{MockInMemoryNodeBuilder, NodeBuilderDir},
        },
        overlay::{node, peer_node::PeerNode},
        overlay_impl::{
            in_memory_conn::{InMemoryConn, InMemoryConnBuilder},
            in_memory_global::InMemoryGlobalState,
            in_memory_peer::{test_data_create_mock_in_memory_nodes, InMemoryPeerBuilder},
        },
        scp::{
            ballot_protocol::BallotProtocolState,
            envelope::SCPEnvelopeController,
            local_node::LocalNodeInfo,
            local_node_builder::LocalNodeBuilder,
            nomination_protocol::{NominationProtocol, NominationProtocolState},
            scp::NodeID,
            scp_driver::SlotDriver,
            scp_driver_builder::SlotDriverBuilder,
        },
    };

    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::work_queue::WorkScheduler,
        mock::state::{MockState, MockStateDriver},
    };

    use super::*;

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
            .herder_driver(Arc::new(state_driver))
            .timer(Rc::new(RefCell::new(timer_handle)))
            .local_node(Arc::new(local_node))
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
                .herder_driver(Arc::new(MockStateDriver::new()))
                .timer(Rc::new(RefCell::new(timer_handle)))
                .local_node(Arc::new(local_node))
                .build_handle()
                .unwrap();

        let value = Arc::new(MockState::random());
        let prev_value = MockState::random();
        let mut envelope_controller = SCPEnvelopeController::<MockState>::new();

        let mut nomination_state = NominationProtocolState::new(node_id.clone());
        let mut ballot_state = BallotProtocolState::default();
        let mut quorum_manager = QuorumManager::default();

        slot_driver.nominate(
            &mut nomination_state,
            &mut ballot_state,
            value,
            &prev_value,
            &mut envelope_controller,
            &mut quorum_manager,
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
            .local_node(Arc::new(local_node))
            .timer(Rc::new(RefCell::new(timer_handle)))
            .herder_driver(Arc::new(MockStateDriver::new()))
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

        let herder = Rc::new(RefCell::new(MockStateDriver::new()));
        herder
    }

    // fn set_up_test_nodes<'a>() -> (
    //     Rc<
    //         RefCell<
    //             PeerNode<
    //                 'a,
    //                 MockState,
    //                 MockStateDriver,
    //                 InMemoryConn<MockState>,
    //                 InMemoryConnBuilder<MockState>,
    //             >,
    //         >,
    //     >,
    //     Rc<
    //         RefCell<
    //             PeerNode<
    //                 'a,
    //                 MockState,
    //                 MockStateDriver,
    //                 InMemoryConn<MockState>,
    //                 InMemoryConnBuilder<MockState>,
    //             >,
    //         >,
    //     >,
    //     HashMap<
    //         std::string::String,
    //         Rc<
    //             RefCell<
    //                 PeerNode<
    //                     'a,
    //                     MockState,
    //                     MockStateDriver,
    //                     InMemoryConn<MockState>,
    //                     InMemoryConnBuilder<MockState>,
    //                 >,
    //             >,
    //         >,
    //     >,
    //     InMemoryPeerBuilder<MockState, MockStateDriver, mock::state::MockStateDriverBuilder>,
    // ) {
    //     let herder_builder = MockStateDriverBuilder {};
    //     let mut node_builder = InMemoryPeerBuilder::new(herder_builder);
    //     let (node1, node2)= test_data_create_mock_state_local_node_info();
    //     node_infos
    //         .into_iter()
    //         .map(|node_info| builder.build_node(node_info))
    //         .next_tuple()
    //         .unwrap()
    // }
    //     let mut peers = HashMap::new();
    //     peers.insert(node1.borrow().peer_idx.clone(), node1.clone());
    //     peers.insert(node2.borrow().peer_idx.clone(), node2.clone());

    //     (node1, node2, peers, node_builder)
    // }

    // #[test]
    // fn in_memory_pget_latest_messageeer_send_hello_message() {
    //     let (node1, node2, mut peers, node_builder) = set_up_test_nodes();

    //     node1
    //         .borrow_mut()
    //         .send_hello_to_peer(&node2.borrow().peer_idx);

    //     assert_eq!(
    //         InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
    //         2,
    //     );
    // }

    // fn in_memory_peer_nominate() {
    //     let (node1, node2, mut peers, node_builder) = set_up_test_nodes();

    //     node1
    //         .borrow_mut()
    //         .send_hello_to_peer(&node2.borrow().peer_idx);
    //     assert_eq!(
    //         InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
    //         2,
    //     );

    //     node1.borrow_mut().slot_nominate(0);
    //     // node1.borrow_mut().nominate(&node2.borrow().peer_idx, 0);
    //     assert_eq!(
    //         InMemoryGlobalState::process_messages(&node_builder.global_state, &mut peers),
    //         1,
    //     );
    // }

    #[test]
    fn in_memory_peer_send_hello_from_local_node_on_file() {
        let mut builder = MockInMemoryNodeBuilder::new(NodeBuilderDir::Test.get_dir_path());
        let mut node1 = builder.build_node("node1").unwrap();
        let node2 = builder.build_node("node2").unwrap();

        node1.send_hello();

        // assert!(
        // InMemoryGlobalState::process_messages(&builder.global_state, &mut builder.nodes) > 0
        // );
    }

    #[test]
    fn in_memory_peer_nominate_from_local_node_on_file() {
        let mut builder = MockInMemoryNodeBuilder::new(NodeBuilderDir::Test.get_dir_path());
        let mut nodes = BTreeMap::new();
        nodes.insert("node1".to_string(), builder.build_node("node1").unwrap());
        nodes.insert("node2".to_string(), builder.build_node("node2").unwrap());

        PeerNode::add_leader_for_nodes(
            nodes.iter_mut().map(|(_, node)| node),
            &"node1".to_string(),
        );

        for node in nodes.values() {
            assert_eq!(node.leaders, vec!["node1".to_string()]);
        }

        assert!(nodes["node1"].get_current_nomination_state(&0).is_none());
        assert!(nodes["node2"].get_current_nomination_state(&0).is_none());

        nodes.get_mut("node1").unwrap().slot_nominate(0);

        assert_eq!(
            InMemoryGlobalState::process_messages(&builder.global_state, &mut nodes),
            3,
        );

        for node in nodes.values() {
            assert_eq!(node.leaders, vec!["node1".to_string()]);
        }

        let node1_nomnination_state: NominationProtocolState<MockState> =
            nodes["node1"].get_current_nomination_state(&0).unwrap();
        let node2_nomnination_state = nodes["node1"].get_current_nomination_state(&0).unwrap();

        assert_eq!(
            node1_nomnination_state.round_leaders,
            node2_nomnination_state.round_leaders
        );

        assert_eq!(nodes["node1"].scp_envelope_controller.envs_to_emit.len(), 0);
        assert_eq!(nodes["node2"].scp_envelope_controller.envs_to_emit.len(), 0);

        assert_eq!(node1_nomnination_state.nomination_started, true);
        assert_eq!(node2_nomnination_state.nomination_started, true);

        assert_eq!(
            node2_nomnination_state.latest_nominations.len(),
            2,
            "Latest nomination statements from {:?}",
            node2_nomnination_state.latest_nominations.keys()
        );
        assert_eq!(
            node1_nomnination_state.latest_nominations.len(),
            2,
            "Latest nomination statements from {:?}",
            node1_nomnination_state.latest_nominations.keys()
        );
        // assert_eq!(node2_nomnination_state.latest_nominations.len(), 2);

        println!("node1 state: {:?}", node1_nomnination_state);
        println!("node2 state: {:?}", node2_nomnination_state);

        assert!(builder.global_state.borrow().msg_peer_id_queue.len() == 0);
        // assert_eq!(
        //     InMemoryGlobalState::process_messages(&builder.global_state, &mut nodes),
        //     0
        // );
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
