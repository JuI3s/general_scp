use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    os::fd::RawFd,
    sync::{Arc, Mutex, Weak},
};

pub type HashValue = u64;

use syn::token::Mut;

use crate::{
    application::{
        quorum::QuorumSet,
        work_queue::{ClockEvent, HWorkQueue},
    },
    herder::herder::HerderDriver,
    scp::ballot_protocol::SCPPhase,
    utils::weak_self::WeakSelf,
};

use super::{
    ballot_protocol::{BallotProtocol, BallotProtocolState, HBallotProtocolState, SCPBallot},
    local_node::{HLocalNode, LocalNode},
    nomination_protocol::{
        HLatestCompositeCandidateValue, HNominationProtocolState, HNominationValue, NominationValue,
    },
    scp::NodeID,
    slot::{HSlot, Slot, SlotIndex},
    statement::SCPStatement,
};

pub type HSCPDriver = Arc<Mutex<dyn SCPDriver>>;

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
pub struct SlotDriver<T>
where
    T: HerderDriver,
{
    pub slot_index: u64,
    pub local_node: HLocalNode,
    pub timer: HSlotTimer,
    nomination_state_handle: HNominationProtocolState,
    ballot_state_handle: HBallotProtocolState,
    pub herder_driver: T,
}

pub type HSCPEnvelope = Arc<Mutex<SCPEnvelope>>;
pub struct SCPEnvelope {
    pub statement: SCPStatement,
    pub node_id: NodeID,
    pub slot_index: SlotIndex,
    pub signature: HashValue,
}

impl SCPEnvelope {
    pub fn get_statement(&self) -> &SCPStatement {
        &self.statement
    }

    // Used only for testing
    // TODO: is there any way I can enable the functions below only during testing?
    pub fn test_make_scp_envelope(node_id: NodeID) -> Self {
        SCPEnvelope {
            statement: SCPStatement::Prepare(super::statement::SCPStatementPrepare {
                quorum_set_hash: 0,
                ballot: SCPBallot::default(),
                prepared: Some(SCPBallot::default()),
                prepared_prime: Some(SCPBallot::default()),
                num_commit: 0,
                num_high: 0,
                from_self: true,
            }),
            node_id: node_id,
            slot_index: 0,
            signature: 0,
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
            }),
            node_id: node_id,
            slot_index: 0,
            signature: 0,
        }
    }

    pub fn test_make_scp_envelope_handle(node_id: NodeID) -> HSCPEnvelope {
        Arc::new(Mutex::new(SCPEnvelope::test_make_scp_envelope(node_id)))
    }
}

pub trait SCPDriver {
    fn validate_value(
        slot_index: u64,
        value: &NominationValue,
        nomination: bool,
    ) -> ValidationLevel;

    // Inform about events happening within the consensus algorithm.

    // ``nominating_value`` is called every time the local instance nominates a new value.
    fn nominating_value(self: &Arc<Self>, value: &NominationValue);
    // `value_externalized` is called at most once per slot when the slot externalize its value.
    fn value_externalized(self: &Arc<Self>, slot_index: u64, value: &NominationValue);
    // `accepted_bsallot_prepared` every time a ballot is accepted as prepared
    fn accepted_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot);

    fn accepted_commit(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot);

    fn confirm_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot) {}

    // the following methods are used for monitoring of the SCP subsystem most implementation don't really need to do anything with these.

    fn emit_envelope(envelope: &SCPEnvelope);

    fn sign_envelope(envelope: &SCPEnvelope);
}

pub type HSlotTimer = Arc<Mutex<SlotTimer>>;
pub struct SlotTimer {
    work_queue: HWorkQueue,
}

impl SlotTimer {
    pub fn add_task(&mut self, callback: ClockEvent) {
        self.work_queue.lock().unwrap().add_task(callback);
    }
}

impl<T: HerderDriver + 'static> SlotDriver<T> {
    pub fn bump_state_(self: &Arc<Self>, nomination_value: &NominationValue, force: bool) -> bool {
        self.bump_state(
            &mut self.ballot_state_handle.lock().unwrap(),
            nomination_value,
            force,
        )
    }

    fn get_local_node(&self) -> &LocalNode {
        todo!();
    }

    pub fn get_latest_composite_value(&self) -> HLatestCompositeCandidateValue {
        self.nomination_state_handle
            .lock()
            .unwrap()
            .latest_composite_candidate
            .clone()
    }

    pub fn federated_accept(
        &self,
        voted_predicate: impl Fn(&SCPStatement) -> bool,
        accepted_predicate: impl Fn(&SCPStatement) -> bool,
        envelopes: &BTreeMap<NodeID, HSCPEnvelope>,
    ) -> bool {
        if LocalNode::is_v_blocking(
            self.get_local_node().get_quorum_set(),
            envelopes,
            &accepted_predicate,
        ) {
            true
        } else {
            let ratify_filter =
                move |st: &SCPStatement| accepted_predicate(st) && voted_predicate(st);
            if LocalNode::is_quorum_with_node_filter(
                Some((
                    self.get_local_node().get_quorum_set(),
                    &self.get_local_node().node_id,
                )),
                envelopes,
                |st| self.herder_driver.get_quorum_set(st),
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
        voted_predicate: impl Fn(&SCPStatement) -> bool,
        envelopes: &BTreeMap<NodeID, HSCPEnvelope>,
    ) -> bool {
        LocalNode::is_quorum_with_node_filter(
            Some((
                self.get_local_node().get_quorum_set(),
                &self.get_local_node().node_id,
            )),
            envelopes,
            |st| self.herder_driver.get_quorum_set(st),
            voted_predicate,
        )
    }

    fn sign_envelope(&self) -> HashValue {
        todo!()
    }

    pub fn create_envelope(&self, statement: SCPStatement) -> SCPEnvelope {
        SCPEnvelope {
            statement,
            node_id: self.local_node.lock().unwrap().node_id.clone(),
            slot_index: self.slot_index.clone(),
            signature: self.sign_envelope(),
        }
    }
}

impl<T: HerderDriver> SCPDriver for SlotDriver<T> {
    fn nominating_value(self: &Arc<Self>, value: &NominationValue) {}

    fn validate_value(
        slot_index: u64,
        value: &NominationValue,
        nomination: bool,
    ) -> ValidationLevel {
        ValidationLevel::MaybeValid
    }

    fn emit_envelope(envelope: &SCPEnvelope) {}

    fn value_externalized(self: &Arc<Self>, slot_index: u64, value: &NominationValue) {
        todo!()
    }

    fn sign_envelope(envelope: &SCPEnvelope) {
        todo!()
    }

    fn accepted_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot) {}
    fn accepted_commit(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot) {}
    fn confirm_ballot_prepared(self: &Arc<Self>, slot_index: &u64, ballot: &SCPBallot) {}
}
