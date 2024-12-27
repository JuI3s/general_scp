use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{Arc, Mutex},
};

// pub type HashValue = Vec<u8>;
pub type HashValue = [u8; 64];

use log::{debug, info};
use serde::Serialize;
use tracing::field::debug;

use crate::{
    application::{
        quorum::{accept_predicate, is_v_blocking, nodes_form_quorum, QuorumSet},
        quorum_manager::{self, QuorumManager},
        work_queue::{HClockEvent, WorkScheduler},
    },
    crypto::types::{test_default_blake2, Blake2Hashable},
    herder::herder::HerderDriver,
    scp::{
        local_node::extract_nodes_from_statement_with_filter,
        nomination_protocol::NominationProtocol,
    },
    utils::test::pretty_print_scp_env_id,
};

use super::{
    ballot_protocol::{BallotProtocol, BallotProtocolState, SCPBallot},
    envelope::{EnvMap, SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
    local_node::{HLocalNode, LocalNodeInfo},
    nomination_protocol::{NominationProtocolState, NominationValue},
    queue::SlotJobQueue,
    scp::{EnvelopeState, NodeID},
    slot::SlotIndex,
    statement::{SCPStatement, SCPStatementNominate},
};

pub type HSCPDriver<N> = Arc<Mutex<dyn SCPDriver<N>>>;

#[derive(PartialEq, Eq)]
pub enum ValidationLevel {
    Invalid,
    MaybeValid,
    VoteToNominate,
    FullyValidated,
}
// #[derive(WeakSelf)]
pub struct SlotDriver<N, H>
where
    N: NominationValue + 'static,
    H: HerderDriver<N>,
{
    pub slot_index: SlotIndex,
    pub local_node: Arc<LocalNodeInfo<N>>,
    pub scheduler: Rc<RefCell<WorkScheduler>>,
    pub herder_driver: Arc<H>,
    pub slot_state: RefCell<SlotState>,
    pub task_queue: Rc<RefCell<SlotJobQueue<N, H>>>,
}

#[derive(PartialEq, Eq, Hash)]
pub enum SlotStateTimer {
    BallotProtocol,
    NominationProtocol,
}

pub struct SlotState {
    pub fully_validated: bool,
    pub got_v_blocking: bool,
    pub ballot_timer: HashMap<SlotStateTimer, HClockEvent>,
}

impl Default for SlotState {
    fn default() -> Self {
        Self {
            fully_validated: true,
            got_v_blocking: Default::default(),
            ballot_timer: Default::default(),
        }
    }
}

impl SlotState {
    pub fn stop_timer(&mut self, timer_type: &SlotStateTimer) {
        if let Some(old_timer) = self.ballot_timer.get(timer_type) {
            old_timer.replace(None);
        }
    }

    pub fn restart_timer(&mut self, timer_type: SlotStateTimer, event: HClockEvent) {
        debug_assert!(event.borrow().is_some());

        // cancel old eventp
        self.stop_timer(&timer_type);

        self.ballot_timer.insert(timer_type, event);
    }
}

impl<N, H> Into<Rc<RefCell<SlotDriver<N, H>>>> for SlotDriver<N, H>
where
    N: NominationValue + 'static,
    H: HerderDriver<N>,
{
    fn into(self) -> Rc<RefCell<SlotDriver<N, H>>> {
        RefCell::new(self).into()
    }
}

pub type HSCPEnvelope<N> = Arc<SCPEnvelope<N>>;

impl<N> Blake2Hashable for SCPEnvelope<N> where N: NominationValue + Serialize {}

impl<N> Into<Arc<Mutex<SCPEnvelope<N>>>> for SCPEnvelope<N>
where
    N: NominationValue,
{
    fn into(self) -> Arc<Mutex<SCPEnvelope<N>>> {
        Mutex::new(self).into()
    }
}

impl<N> SCPEnvelope<N>
where
    N: NominationValue,
{
    pub fn new(
        statement: SCPStatement<N>,
        node_id: NodeID,
        slot_index: SlotIndex,
        signature: HashValue,
    ) -> Self {
        Self {
            statement: statement,
            node_id: node_id,
            slot_index: slot_index,
            signature: signature,
        }
    }

    pub fn to_handle(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn get_statement(&self) -> &SCPStatement<N> {
        &self.statement
    }

    // Used only for testing
    // TODO: is there any way I can enable the functions below only during testing?
    pub fn test_make_scp_envelope(node_id: NodeID) -> Self {
        SCPEnvelope {
            statement: SCPStatement::Prepare(super::statement::SCPStatementPrepare {
                quorum_set_hash: [0; 64],
                ballot: SCPBallot::default(),
                prepared: Some(SCPBallot::default()),
                prepared_prime: Some(SCPBallot::default()),
                num_commit: 0,
                num_high: 0,
                quorum_set: None,
            }),
            node_id: node_id,
            slot_index: 0,
            signature: test_default_blake2(),
        }
    }

    pub fn test_make_scp_envelope_from_quorum(
        node_id: NodeID,
        quorum_set: &QuorumSet,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> SCPEnvelopeID {
        let env = SCPEnvelope {
            statement: SCPStatement::Prepare(super::statement::SCPStatementPrepare {
                quorum_set_hash: quorum_set.hash_value(),
                ballot: SCPBallot::default(),
                prepared: Some(SCPBallot::default()),
                prepared_prime: Some(SCPBallot::default()),
                num_commit: 0,
                num_high: 0,
                quorum_set: None,
            }),
            node_id: node_id,
            slot_index: 0,
            signature: test_default_blake2(),
        };
        envelope_controller.add_envelope(env)
    }
}

// TODO: I think I don't need this trait
pub trait SCPDriver<N>
where
    N: NominationValue,
{
    fn validate_value(slot_index: u64, value: &N, nomination: bool) -> ValidationLevel;

    // Inform about events happening within the consensus algorithm.

    // ``nominating_value`` is called every time the local instance nominates a new
    // value.
    fn nominating_value(&self, value: &N);
    // `value_externalized` is called at most once per slot when the slot
    // externalize its value.
    fn value_externalized(&self, slot_index: u64, value: &N);
    // `accepted_bsallot_prepared` every time a ballot is accepted as prepared
    fn accepted_ballot_prepared(&self, slot_index: &u64, ballot: &SCPBallot<N>);

    fn accepted_commit(&self, slot_index: &u64, ballot: &SCPBallot<N>);

    fn confirm_ballot_prepared(&self, slot_index: &u64, ballot: &SCPBallot<N>) {}

    // the following methods are used for monitoring of the SCP subsystem most
    // implementation don't really need to do anything with these.

    fn emit_envelope(envelope: &SCPEnvelope<N>);

    fn sign_envelope(envelope: &mut SCPEnvelope<N>);
}

impl<N, H> SlotDriver<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    pub fn new(
        slot_index: SlotIndex,
        local_node: Arc<LocalNodeInfo<N>>,
        herder_driver: Arc<H>,
        task_queue: Rc<RefCell<SlotJobQueue<N, H>>>,
        scheduler: Rc<RefCell<WorkScheduler>>,
    ) -> Self {
        Self {
            slot_index,
            local_node,
            herder_driver,
            slot_state: Default::default(),
            task_queue,
            scheduler,
        }
    }

    fn try_accept_value(
        &self,
        value: &N,
        nomination_state: &mut NominationProtocolState<N>,
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        // Logic for accepting a nomination value as described in the paper (copied below) https://johnpconley.com/wp-content/uploads/2021/01/stellar-consensus-protocol.pdf. Returns true if the value was accepted, false otherwise.
        //
        //  (1) There exists a quorum 𝑈 such that 𝑣 ∈ 𝑈 and each member of 𝑈 either voted for 𝑎 or claims to accept 𝑎, or
        //  (2) Each member of a 𝑣-blocking set claims to accept 𝑎.

        false
    }

    pub fn state_may_have_changed(
        &self,
        statement: &SCPStatementNominate<N>,
        nomination_state: &mut NominationProtocolState<N>,
        envelope_controller: &SCPEnvelopeController<N>,
        quorum_manager: &QuorumManager,
    ) -> bool {
        // TODO: Need to check if we need to accept the statement.
        debug!(
            "state_may_have_changed, node: {:?}, statement votes: {:?}, statement accepts: {:?}",
            self.node_idx(),
            statement.votes,
            statement.accepted
        );

        let modified = statement.votes.iter().any(|vote| {
            println!("state_may_have_changed cur vote: {:?}", vote);
            if nomination_state.accepted.contains(vote) {
                println!("state_may_have_changed vote already accepted {:?}", vote);
                return false;
            }
            println!("Node idx: {:?}", self.node_idx());
            println!(
                "latest nominations: {:?}",
                nomination_state.latest_nominations
            );

            if self.federated_accept(
                |st| st.as_nomination_statement().votes.contains(vote),
                |st| accept_predicate(vote, st),
                &nomination_state.latest_nominations,
                &envelope_controller.envelopes,
                quorum_manager
            ) {
                match self.herder_driver.validate_value(vote, true) {
                    ValidationLevel::FullyValidated => {
                        debug!(
                            "state_may_have_changed STATE CHANGE: nodes {:?} fully validates value {:?}",
                            self.node_idx(),
                            vote
                        );

                        let value = Arc::new(vote.clone());
                        nomination_state.accepted.insert(value.clone());
                        nomination_state.votes.insert(value.clone());
                        return true;
                    }
                    _ => {
                        if let Some(value) = self.herder_driver.extract_valid_value(vote) {
                            nomination_state.accepted.insert(Arc::new(value.clone()));
                            nomination_state.votes.insert(Arc::new(value.clone()));
                            return true;
                        }
                    }
                }
            }
            false
        });
        modified
    }

    pub fn node_idx(&self) -> NodeID {
        self.local_node.node_id.clone()
    }

    pub fn recv_scp_envelvope(
        &self,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        env_id: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
    ) -> EnvelopeState {
        let is_ballot = {
            let env = envelope_controller.get_envelope(env_id).unwrap();
            info!(
                "recv_scp_envelvope: node {:?} receives an envelope: {:?}",
                self.node_idx(),
                pretty_print_scp_env_id(&env_id)
            );

            let is_ballot = match env.get_statement() {
                SCPStatement::Prepare(_)
                | SCPStatement::Confirm(_)
                | SCPStatement::Externalize(_) => true,
                SCPStatement::Nominate(st) => false,
            };
            is_ballot
        };

        if is_ballot {
            self.process_ballot_envelope(
                ballot_state,
                nomination_state,
                env_id,
                true,
                &mut envelope_controller.envelopes,
                &mut envelope_controller.envs_to_emit,
                &quorum_manager,
            )
        } else {
            self.process_nomination_envelope(
                nomination_state,
                ballot_state,
                &env_id,
                envelope_controller,
                quorum_manager,
            )
        }
    }

    pub fn federated_accept(
        &self,
        voted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        accepted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        envelopes: &BTreeMap<NodeID, SCPEnvelopeID>,
        env_map: &EnvMap<N>,
        quorum_manager: &QuorumManager,
    ) -> bool {
        println!(
            "federated_accept: local node {:?}, envelopes {:?}",
            self.node_idx(),
            envelopes
        );
        let ratify_filter =
            move |st: &SCPStatement<N>| accepted_predicate(st) || voted_predicate(st);

        if LocalNodeInfo::<N>::is_v_blocking_with_predicate(
            &self.local_node.quorum_set,
            envelopes,
            &ratify_filter,
            env_map,
        ) {
            println!(
                "federated_accept: node {:?} is_v_blocking_with_predicate returns true",
                self.local_node.node_id
            );

            true
        } else {
            println!(
                "federated_accept: node {:?} is_v_blocking_with_predicate returns false",
                self.local_node.node_id
            );

            let nodes =
                extract_nodes_from_statement_with_filter(envelopes, &env_map, ratify_filter);

            println!("nodes in federated accept: {:?}", nodes);

            if nodes_form_quorum(
                |node| {
                    if node == self.local_node.node_id.as_str() {
                        Some(&self.local_node.quorum_set)
                    } else {
                        let env_id = envelopes.get(node).unwrap();
                        let env = env_map.0.get(env_id).unwrap();
                        let statement = env.get_statement();
                        quorum_manager.get_quorum_set(statement)
                    }
                },
                &nodes,
            ) {
                return true;
            }
            {
                false
            }
        }
    }

    pub fn federated_ratify(
        &self,
        voted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        envelopes: &BTreeMap<NodeID, SCPEnvelopeID>,
        env_map: &EnvMap<N>,
        quorum_manager: &QuorumManager,
    ) -> bool {
        // Definition of ratify (under Ratification): https://stellar.org/blog/thought-leadership/on-worldwide-consensus

        println!("envelopes before: {:?}", envelopes);
        env_map.display_for_env_ids(envelopes.values().into_iter());

        let nodes = extract_nodes_from_statement_with_filter(envelopes, &env_map, voted_predicate);

        debug!(
            "node {:?} tries to federated_ratify nodes: {:?}, local_quorum_set: {:?}",
            self.node_idx(),
            nodes,
            self.local_node.quorum_set
        );

        nodes_form_quorum(
            |node| {
                if node == self.local_node.node_id.as_str() {
                    Some(&self.local_node.quorum_set)
                } else {
                    let env_id = envelopes.get(node).unwrap();
                    let env = env_map.0.get(env_id).unwrap();
                    let st = env.get_statement();
                    quorum_manager.get_quorum_set(st)
                }
            },
            &nodes,
        )
    }

    pub fn create_envelope(
        &self,
        statement: SCPStatement<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> SCPEnvelopeID {
        /// Create an envelope and add it to the queue of envelopes to be emitted.

        debug!(
            "create_envelope: node {:?} creates envelope with statement: {:?}",
            self.node_idx(),
            statement
        );

        let env = SCPEnvelope {
            statement,
            node_id: self.local_node.node_id.clone(),
            slot_index: self.slot_index.clone(),
            signature: test_default_blake2(),
        };

        let env_id = envelope_controller.add_envelope(env);
        envelope_controller.add_env_to_emit(&env_id);
        debug!(
            "create_envelope: node {:?} creates cur_env_id: {:?}",
            self.node_idx(),
            pretty_print_scp_env_id(&env_id)
        );
        env_id
    }

    fn get_latest_message(
        node_id: &NodeID,
        ballot_state: &BallotProtocolState<N>,
        nomination_state: &NominationProtocolState<N>,
    ) -> Option<SCPEnvelopeID> {
        // Return the latest message we have heard from the node with node_id. Start
        // searching in the ballot protocol state and then the nomination protocol
        // state. If nothing is found, return None.

        if let Some(env) = ballot_state.latest_envelopes.get(node_id) {
            return Some(env.clone());
        }

        if let Some(env) = nomination_state.latest_nominations.get(node_id) {
            return Some(env.clone());
        }

        None
    }

    pub fn maybe_got_v_blocking(
        &mut self,
        nomination_state: &NominationProtocolState<N>,
        ballot_state: &BallotProtocolState<N>,
    ) {
        // Called when we process an envelope or set state from an envelope and maybe we
        // hear from a v-blocking set for the first time.

        if self.slot_state.borrow().got_v_blocking {
            return;
        }

        // Add nodes that we have heard from.
        let mut nodes: Vec<NodeID> = Default::default();

        LocalNodeInfo::<N>::for_all_nodes(&self.local_node.quorum_set, &mut |node| {
            if Self::get_latest_message(node, ballot_state, nomination_state).is_some() {
                nodes.push(node.to_owned());
            }
            true
        });

        if is_v_blocking(&self.local_node.quorum_set, &nodes) {
            self.slot_state.borrow_mut().got_v_blocking = true;
        }
    }
}

impl<N, H> SCPDriver<N> for SlotDriver<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    fn nominating_value(&self, value: &N) {}

    fn validate_value(slot_index: u64, value: &N, nomination: bool) -> ValidationLevel {
        ValidationLevel::FullyValidated
        // ValidationLevel::MaybeValid
    }

    fn emit_envelope(envelope: &SCPEnvelope<N>) {
        println!("Emitting an envelope");
    }

    fn value_externalized(&self, slot_index: u64, value: &N) {
        debug!(
            "node {:?} externalized value {:?}",
            self.local_node.node_id, value
        );
        // todo!();
    }

    fn sign_envelope(envelope: &mut SCPEnvelope<N>) {
        // TODO: for now just pretend we're signing...
        envelope.signature = test_default_blake2();
    }

    fn accepted_ballot_prepared(&self, slot_index: &u64, ballot: &SCPBallot<N>) {}
    fn accepted_commit(&self, slot_index: &u64, ballot: &SCPBallot<N>) {}
    fn confirm_ballot_prepared(&self, slot_index: &u64, ballot: &SCPBallot<N>) {}
}

#[cfg(test)]
mod tests {
    use crate::mock::state::MockState;

    use super::*;

    #[test]
    fn mock_scp_envelope_to_blake2() {
        let env1 = SCPEnvelope::<MockState>::test_make_scp_envelope("1".into());
        let env2 = SCPEnvelope::<MockState>::test_make_scp_envelope("2".into());
        let hash_1 = env1.to_blake2();
        let hash_2 = env2.to_blake2();
        assert_ne!(hash_1, hash_2);
        assert_eq!(hash_1, env1.to_blake2());
    }

    #[test]
    fn test_federated_accept() {
        todo!()
    }
}
