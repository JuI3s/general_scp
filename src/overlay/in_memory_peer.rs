use std::{arch::global_asm, cell::RefCell, marker::PhantomData, rc::Rc};

use itertools::Itertools;

use crate::{
    application::{
        clock::VirtualClock,
        quorum::{QuorumNode, QuorumSet, QuorumSlice},
        work_queue::WorkScheduler,
    },
    herder::herder::{HerderBuilder, HerderDriver},
    mock::{
        scp_driver::MockSCPDriver,
        state::{MockState, MockStateDriver, MockStateDriverBuilder},
    },
    scp::{local_node::LocalNodeInfo, nomination_protocol::NominationValue},
};

use super::{
    in_memory_conn::{InMemoryConn, InMemoryConnBuilder},
    in_memory_global::InMemoryGlobalState,
    node,
    peer::PeerID,
    peer_node::PeerNode,
};

type TestPeerType =
    PeerNode<MockState, MockStateDriver, InMemoryConn<MockState>, InMemoryConnBuilder<MockState>>;
type TestPeerBuilder = InMemoryPeerBuilder<MockState, MockStateDriver, MockStateDriverBuilder>;

pub struct InMemoryPeerBuilder<N, H, HB>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
    HB: HerderBuilder<N, H>,
{
    global_state: Rc<RefCell<InMemoryGlobalState<N>>>,
    herder_builder: HB,
    phantom: PhantomData<H>,
}

impl<N, H, HB> InMemoryPeerBuilder<N, H, HB>
where
    N: NominationValue,
    H: HerderDriver<N> + Clone,
    HB: HerderBuilder<N, H>,
{
    pub fn new(herder_builder: HB) -> Self {
        let global_state = InMemoryGlobalState::new();
        Self {
            global_state,
            herder_builder,
            phantom: PhantomData,
        }
    }

    pub fn build_node(
        &self,
        local_node_info: LocalNodeInfo<N>,
    ) -> PeerNode<N, H, InMemoryConn<N>, InMemoryConnBuilder<N>> {
        let conn_builder = InMemoryConnBuilder::new(&self.global_state);
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::new(None)));
        PeerNode::new(
            local_node_info.node_id.clone(),
            self.herder_builder.build(),
            conn_builder,
            &self.global_state,
            local_node_info,
            work_scheduler,
        )
    }
}

pub fn create_mock_state_in_memory_peer_builder(
) -> InMemoryPeerBuilder<MockState, MockStateDriver, MockStateDriverBuilder> {
    let herder_builder = MockStateDriverBuilder {};
    InMemoryPeerBuilder::new(herder_builder)
}

pub fn test_data_create_mock_state_local_node_info() -> Vec<LocalNodeInfo<MockState>> {
    let node_1_id = "node1".to_string();
    let node_2_id = "node2".to_string();
    let node_1 = QuorumNode::new(node_1_id, None);
    let node_2 = QuorumNode::new(node_2_id, None);

    let quorum_slice = QuorumSlice::from([node_1.clone(), node_2.clone()]);
    let quorum = QuorumSet::from([quorum_slice]);

    let node_info1 = LocalNodeInfo::new(false, quorum.clone(), node_1.node_id.clone());
    let node_info2 = LocalNodeInfo::new(false, quorum.clone(), node_2.node_id.clone());

    vec![node_info1, node_info2]
}

pub fn test_data_create_mock_in_memory_nodes(
    builder: &TestPeerBuilder,
) -> (TestPeerType, TestPeerType) {
    let node_infos = test_data_create_mock_state_local_node_info();
    node_infos
        .into_iter()
        .map(|node_info| builder.build_node(node_info))
        .next_tuple()
        .unwrap()
}
