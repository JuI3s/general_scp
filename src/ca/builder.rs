use crate::application::quorum::QuorumSet;
use crate::herder::herder::HerderDriver;
use crate::scp::builder::InMemoryNodeBuilder;
use crate::scp::statement::SCPStatement;
use std::collections::BTreeSet;
use std::sync::Arc;

use super::local_state::LocalCAState;
use super::operation::{CAOperation, SCPCAOperation};

pub struct CAStateDriver(pub LocalCAState);

impl HerderDriver<SCPCAOperation> for CAStateDriver {
    fn combine_candidates(
        &self,
        candidates: &BTreeSet<Arc<SCPCAOperation>>,
    ) -> Option<SCPCAOperation> {
        // TODO: need to filter out conflicting operations
        Some(SCPCAOperation(Vec::from_iter(
            candidates
                .iter()
                .map(|val| val.0.iter())
                .flatten()
                .map(|val| val.to_owned()),
        )))
    }

    fn extract_valid_value(&self, value: &SCPCAOperation) -> Option<SCPCAOperation> {
        None
    }

    fn externalize_value(&mut self, value: &SCPCAOperation) {
        self.0.state.on_scp_operation(value);
    }

    fn new() -> Self {
        panic!()
    }
}

pub type CAInMemoryNodeBuilder = InMemoryNodeBuilder<SCPCAOperation, CAStateDriver>;

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use crate::{
        ca::{builder::CAInMemoryNodeBuilder, operation::SCPCAOperation},
        mock::builder::NodeBuilderDir,
        overlay::peer_node::PeerNode,
        overlay_impl::in_memory_global::InMemoryGlobalState,
        scp::nomination_protocol::NominationProtocolState,
    };

    #[test]
    fn func() {
        let mut builder = CAInMemoryNodeBuilder::new(NodeBuilderDir::Test.get_dir_path());

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

        assert!(InMemoryGlobalState::process_messages(&builder.global_state, &mut nodes) > 0);

        for node in nodes.values() {
            assert_eq!(node.leaders, vec!["node1".to_string()]);
        }

        let node1_nomnination_state: NominationProtocolState<SCPCAOperation> =
            nodes["node1"].get_current_nomination_state(&0).unwrap();
        let node2_nomnination_state = nodes["node1"].get_current_nomination_state(&0).unwrap();

        assert_eq!(
            node1_nomnination_state.round_leaders,
            node2_nomnination_state.round_leaders
        );

        assert_eq!(nodes["node1"].scp_envelope_controller.envs_to_emit.len(), 0);
        assert_eq!(nodes["node2"].scp_envelope_controller.envs_to_emit.len(), 0);

        assert_eq!(node1_nomnination_state.nomination_started, false);
        assert_eq!(node2_nomnination_state.nomination_started, false);

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

        assert!(builder.global_state.borrow().msg_peer_id_queue.len() == 0);
    }
}
