use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    env,
    os::fd::RawFd,
    rc::Rc,
    sync::{Arc, Mutex, Weak},
};

// pub type HashValue = Vec<u8>;
pub type HashValue = [u8; 64];

use serde::{Deserialize, Serialize};
use syn::token::Mut;

use crate::{
    application::{
        quorum::{QuorumSet, QuorumSetHash},
        work_queue::{self, ClockEvent, WorkScheduler},
    },
    crypto::types::{test_default_blake2, Blake2Hashable},
    herder::herder::HerderDriver,
    overlay::overlay_manager::OverlayManager,
    scp::ballot_protocol::SCPPhase,
    utils::weak_self::WeakSelf,
};

use super::{
    ballot_protocol::{BallotProtocol, BallotProtocolState, HBallotProtocolState, SCPBallot},
    local_node::{HLocalNode, LocalNode},
    nomination_protocol::{
        HLatestCompositeCandidateValue, HNominationProtocolState, HSCPNominationValue,
        NominationValue, SCPNominationValue,
    },
    scp::NodeID,
    slot::{HSlot, Slot, SlotIndex},
    statement::SCPStatement,
};

pub type HSCPDriver<N> = Arc<Mutex<dyn SCPDriver<N>>>;

pub enum EnvelopeState {
    Invalid,
    Valid,
}

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
    pub scheduler: WorkScheduler,
    nomination_state_handle: HNominationProtocolState<N>,
    ballot_state_handle: HBallotProtocolState<N>,
    pub herder_driver: Rc<RefCell<H>>,
    pub fully_validated: bool,
    pub got_v_blocking: bool,
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

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SCPEnvelope<N>
where
    N: NominationValue,
{
    pub statement: SCPStatement<N>,
    pub node_id: NodeID,
    pub slot_index: SlotIndex,

    #[serde(with = "serde_bytes")]
    pub signature: HashValue,
}

impl<N> Blake2Hashable for SCPEnvelope<N> where N: NominationValue + Serialize {}

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
                from_self: true,
                quorum_set: None,
            }),
            node_id: node_id,
            slot_index: 0,
            signature: test_default_blake2(),
        }
    }

    pub fn test_make_scp_envelope_from_quorum(node_id: NodeID, quorum_set: &QuorumSet) -> Self {
        SCPEnvelope {
            statement: SCPStatement::Prepare(super::statement::SCPStatementPrepare {
                quorum_set_hash: quorum_set.hash_value(),
                ballot: SCPBallot::default(),
                prepared: Some(SCPBallot::default()),
                prepared_prime: Some(SCPBallot::default()),
                num_commit: 0,
                num_high: 0,
                from_self: true,
                quorum_set: None,
            }),
            node_id: node_id,
            slot_index: 0,
            signature: test_default_blake2(),
        }
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
        timer: WorkScheduler,
        nomination_state_handle: HNominationProtocolState<N>,
        ballot_state_handle: HBallotProtocolState<N>,
        herder_driver: Rc<RefCell<H>>,
    ) -> Self {
        Self {
            slot_index: slot_index,
            local_node: local_node,
            scheduler: timer,
            nomination_state_handle: nomination_state_handle,
            ballot_state_handle: ballot_state_handle,
            herder_driver: herder_driver,
            fully_validated: true,
            got_v_blocking: false,
        }
    }

    pub fn recv_scp_envelvope(self: &Arc<Self>, envelope: &SCPEnvelope<N>) {
        todo!()
    }

    pub fn nomination_state(&self) -> &HNominationProtocolState<N> {
        &self.nomination_state_handle
    }

    pub fn ballot_state(&self) -> HBallotProtocolState<N> {
        self.ballot_state().clone()
    }

    pub fn bump_state_(self: &Arc<Self>, nomination_value: &N, force: bool) -> bool {
        self.bump_state(
            &mut self.ballot_state_handle.lock().unwrap(),
            nomination_value,
            force,
        )
    }

    pub fn get_latest_composite_value(&self) -> HLatestCompositeCandidateValue<N> {
        self.nomination_state_handle
            .lock()
            .unwrap()
            .latest_composite_candidate
            .clone()
    }

    pub fn federated_accept(
        &self,
        voted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        accepted_predicate: impl Fn(&SCPStatement<N>) -> bool,
        envelopes: &BTreeMap<NodeID, HSCPEnvelope<N>>,
    ) -> bool {
        if LocalNode::is_v_blocking_with_predicate(
            self.local_node.borrow().get_quorum_set(),
            envelopes,
            &accepted_predicate,
        ) {
            true
        } else {
            let ratify_filter =
                move |st: &SCPStatement<N>| accepted_predicate(st) && voted_predicate(st);

            let local_node = self.local_node.borrow();
            if LocalNode::is_quorum_with_node_filter(
                Some((local_node.get_quorum_set(), &local_node.node_id)),
                envelopes,
                |st| self.herder_driver.borrow().get_quorum_set(st),
                ratify_filter,
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
        envelopes: &BTreeMap<NodeID, HSCPEnvelope<N>>,
    ) -> bool {
        let local_node = self.local_node.borrow();

        LocalNode::is_quorum_with_node_filter(
            Some((local_node.get_quorum_set(), &local_node.node_id)),
            envelopes,
            |st| self.herder_driver.borrow().get_quorum_set(st),
            voted_predicate,
        )
    }

    pub fn create_envelope(&self, statement: SCPStatement<N>) -> SCPEnvelope<N> {
        SCPEnvelope {
            statement,
            node_id: self.local_node.borrow().node_id.clone(),
            slot_index: self.slot_index.clone(),
            signature: test_default_blake2(),
        }
    }

    fn get_latest_message(&self, node_id: &NodeID) -> Option<HSCPEnvelope<N>> {
        // Return the latest message we have heard from the node with node_id. Start
        // searching in the ballot protocol state and then the nomination protocol
        // state. If nothing is found, return None.

        if let Some(env) = self
            .ballot_state()
            .lock()
            .unwrap()
            .latest_envelopes
            .get(node_id)
        {
            return Some(env.clone());
        }

        if let Some(env) = self
            .nomination_state()
            .lock()
            .unwrap()
            .latest_nominations
            .get(node_id)
        {
            return Some(env.clone());
        }

        None
    }

    pub fn maybe_got_v_blocking(&mut self) {
        // Called when we process an envelope or set state from an envelope and maybe we
        // hear from a v-blocking set for the first time.

        if self.got_v_blocking {
            return;
        }

        let local_node = self.local_node.borrow();

        // Add nodes that we have heard from.
        let mut nodes: Vec<NodeID> = Default::default();

        LocalNode::<N>::for_all_nodes(&local_node.quorum_set, &mut |node| {
            if self.get_latest_message(node).is_some() {
                nodes.push(node.to_owned());
            }
            true
        });

        if LocalNode::<N>::is_v_blocking(&local_node.quorum_set, &nodes) {
            self.got_v_blocking = true;
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
