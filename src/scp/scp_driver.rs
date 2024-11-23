use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    env,
    os::fd::RawFd,
    rc::Rc,
    sync::{Arc, Mutex, Weak},
    time::SystemTime,
};

// pub type HashValue = Vec<u8>;
pub type HashValue = [u8; 64];

use serde::{Deserialize, Serialize};
use syn::token::Mut;
use tokio::task;

use crate::{
    application::{
        quorum::{QuorumSet, QuorumSetHash},
        work_queue::{self, ClockEvent, HClockEvent, WorkScheduler},
    },
    crypto::types::{test_default_blake2, Blake2Hashable},
    herder::herder::HerderDriver,
    overlay::overlay_manager::OverlayManager,
    scp::{ballot_protocol::SCPPhase, nomination_protocol::NominationProtocol},
    utils::weak_self::WeakSelf,
};

use super::{
    ballot_protocol::{self, BallotProtocol, BallotProtocolState, HBallotProtocolState, SCPBallot},
    envelope::{self, SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
    local_node::{HLocalNode, LocalNodeInfo},
    nomination_protocol::{
        HLatestCompositeCandidateValue, HNominationProtocolState, HSCPNominationValue,
        NominationProtocolState, NominationValue, SCPNominationValue,
    },
    queue::SlotJobQueue,
    scp::NodeID,
    slot::{HSlot, Slot, SlotIndex},
    statement::SCPStatement,
};

pub type HSCPDriver<N> = Arc<Mutex<dyn SCPDriver<N>>>;

pub enum EnvelopeState {
    Invalid,
    Valid,
}

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
    pub local_node: HLocalNode<N>,
    pub scheduler: Rc<RefCell<WorkScheduler>>,
    pub herder_driver: Rc<RefCell<H>>,
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
            fully_validated: Default::default(),
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

pub type HSCPEnvelope<N> = Arc<Mutex<SCPEnvelope<N>>>;

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
                node_id: node_id.to_owned(),
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
                node_id: node_id.to_owned(),
            }),
            node_id: node_id,
            slot_index: 0,
            signature: test_default_blake2(),
        };
        envelope_controller.add_envelope(env)
    }

    pub fn test_make_scp_envelope_handle(node_id: NodeID) -> HSCPEnvelope<N> {
        Arc::new(Mutex::new(SCPEnvelope::test_make_scp_envelope(node_id)))
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
    fn nominating_value(self: &Arc<Self>, value: &N);
    // `value_externalized` is called at most once per slot when the slot
    // externalize its value.
    fn value_externalized(self: &Arc<Self>, slot_index: u64, value: &N);
    // `accepted_bsallot_prepared` every time a ballot is accepted as prepared
    fn accepted_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>);

    fn accepted_commit(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>);

    fn confirm_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>) {}

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
        local_node: HLocalNode<N>,
        herder_driver: Rc<RefCell<H>>,
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

    pub fn recv_scp_envelvope(
        self: &Arc<Self>,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        env_id: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) {
        let env = envelope_controller.get_envelope(env_id).unwrap();
        println!("Received an envelope: {:?}", env_id);
        match env.get_statement() {
            SCPStatement::Prepare(_) | SCPStatement::Confirm(_) | SCPStatement::Externalize(_) => {
                self.process_ballot_envelope(
                    ballot_state,
                    nomination_state,
                    env,
                    true,
                    envelope_controller,
                );
            }
            SCPStatement::Nominate(st) => {
                self.process_nomination_envelope(
                    nomination_state,
                    ballot_state,
                    &env_id,
                    envelope_controller,
                );
            }
        };
    }

    pub fn bump_state_(
        self: &Arc<Self>,
        nomination_value: &N,
        ballot_state: &mut BallotProtocolState<N>,
        nomination_state: &mut NominationProtocolState<N>,
        force: bool,
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        self.bump_state(
            ballot_state,
            nomination_state,
            nomination_value,
            force,
            envelope_controller,
        )
    }

    pub fn federated_accept(
        &self,
        voted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        accepted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        envelopes: &BTreeMap<NodeID, SCPEnvelopeID>,
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        if LocalNodeInfo::<N>::is_v_blocking_with_predicate(
            &self.local_node.borrow().quorum_set,
            envelopes,
            &accepted_predicate,
            envelope_controller,
        ) {
            true
        } else {
            let ratify_filter =
                move |st: &SCPStatement<N>| accepted_predicate(st) && voted_predicate(st);

            let local_node = self.local_node.borrow();
            if LocalNodeInfo::is_quorum_with_node_filter(
                Some((&local_node.quorum_set, &local_node.node_id)),
                envelopes,
                |st| self.herder_driver.borrow().get_quorum_set(st),
                ratify_filter,
                envelope_controller,
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
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        let local_node = self.local_node.borrow();

        LocalNodeInfo::is_quorum_with_node_filter(
            Some((&local_node.quorum_set, &local_node.node_id)),
            envelopes,
            |st| self.herder_driver.borrow().get_quorum_set(st),
            voted_predicate,
            envelope_controller,
        )
    }

    pub fn create_envelope(
        &self,
        statement: SCPStatement<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> SCPEnvelopeID {
        let env = SCPEnvelope {
            statement,
            node_id: self.local_node.borrow().node_id.clone(),
            slot_index: self.slot_index.clone(),
            signature: test_default_blake2(),
        };
        envelope_controller.add_envelope(env)
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

        let local_node = self.local_node.borrow();

        // Add nodes that we have heard from.
        let mut nodes: Vec<NodeID> = Default::default();

        LocalNodeInfo::<N>::for_all_nodes(&local_node.quorum_set, &mut |node| {
            if Self::get_latest_message(node, ballot_state, nomination_state).is_some() {
                nodes.push(node.to_owned());
            }
            true
        });

        if LocalNodeInfo::<N>::is_v_blocking(&local_node.quorum_set, &nodes) {
            self.slot_state.borrow_mut().got_v_blocking = true;
        }
    }
}

impl<N, H> SCPDriver<N> for SlotDriver<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    fn nominating_value(self: &Arc<Self>, value: &N) {}

    fn validate_value(slot_index: u64, value: &N, nomination: bool) -> ValidationLevel {
        ValidationLevel::MaybeValid
    }

    fn emit_envelope(envelope: &SCPEnvelope<N>) {
        println!("Emitting an envelope");
    }

    fn value_externalized(self: &Arc<Self>, slot_index: u64, value: &N) {
        todo!()
    }

    fn sign_envelope(envelope: &mut SCPEnvelope<N>) {
        // TODO: for now just pretend we're signing...
        envelope.signature = test_default_blake2();
    }

    fn accepted_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>) {}
    fn accepted_commit(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>) {}
    fn confirm_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot<N>) {}
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
}
