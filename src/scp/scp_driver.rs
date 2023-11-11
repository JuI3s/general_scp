use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, Weak}, os::fd::RawFd,
};

use weak_self_derive::WeakSelf;

use crate::{
    application::work_queue::{ClockEvent, HWorkQueue},
    herder::herder::Herder,
    scp::ballot_protocol::SCPPhase,
    utils::weak_self::WeakSelf,
};

use super::{
    ballot_protocol::{
        BallotProtocol, BallotProtocolState, HBallotProtocolState, SCPBallot, SCPStatement,
    },
    local_node::{HLocalNode, LocalNode},
    nomination_protocol::NominationValue,
    scp::NodeID,
    slot::{HSlot, Slot, SlotIndex},
};

pub type HSCPDriver = Arc<Mutex<dyn SCPDriver>>;

// #[derive(WeakSelf)]
pub struct SlotDriver {
    pub slot_index: u64,
    pub local_node: HLocalNode,
    pub timer: HSlotTimer,
}

pub enum ValidationLevel {
    InvalidValue,
    MaybeValidValue,
    FullyValidatedValue,
    VoteToNominate,
}

pub type HSCPEnvelope = Arc<Mutex<SCPEnvelope>>;
pub struct SCPEnvelope {}

impl SCPEnvelope {
    pub fn get_statement(&self) -> &SCPStatement {
        todo!()
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
    fn value_externalized(slot_index: u64, value: &NominationValue);

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

impl SlotDriver {
    fn get_prepare_candidates(hint: &SCPStatement) -> BTreeSet<SCPBallot> {
        todo!()
    }

    fn get_local_node(&self) -> &LocalNode {
        todo!();
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
            let ratify_filter =  move |st: &SCPStatement| {
                accepted_predicate(st) && voted_predicate(st)
            };
            if LocalNode::is_quorum(self.get_local_node().get_quorum_set(), envelopes, ratify_filter) {
                return true;
            } {
                false
            }
        }
    }
}

// pub trait WeakSelf {
//     fn get_weak_self(&mut self) -> Weak<Mutex<&mut Self>>;
// }

impl SCPDriver for SlotDriver {
    fn nominating_value(self: &Arc<Self>, value: &NominationValue) {}

    fn validate_value(
        slot_index: u64,
        value: &NominationValue,
        nomination: bool,
    ) -> ValidationLevel {
        ValidationLevel::MaybeValidValue
    }

    fn emit_envelope(envelope: &SCPEnvelope) {}

    fn value_externalized(slot_index: u64, value: &NominationValue) {
        todo!()
    }

    fn sign_envelope(envelope: &SCPEnvelope) {
        todo!()
    }
}

impl BallotProtocol for SlotDriver {
    fn externalize(&mut self) {
        todo!()
    }

    fn recv_ballot_envelope(&mut self) {
        todo!()
    }

    fn attempt_accept_prepared(
        self: &Arc<Self>,
        state_handle: HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let state = state_handle.lock().unwrap();
        if state.phase != SCPPhase::PhasePrepare && state.phase != SCPPhase::PhaseConfirm {
            return false;
        }

        let candidates = SlotDriver::get_prepare_candidates(hint);

        // see if we can accept any of the candidates, starting with the highest
        for candidate in &candidates {
            if state.phase == SCPPhase::PhaseConfirm {
                match state.prepared.lock().unwrap().as_ref() {
                    Some(prepared_ballot) => {
                        if prepared_ballot.less_and_compatible(&candidate) {
                            continue;
                        }
                    }
                    None => {
                        panic!("In PhaseConfirm (attempt_accept_prepared) but prepared ballot is None.\n");
                    }
                }
            }

            // if we already prepared this ballot, don't bother checking again

            // if ballot <= p' ballot is neither a candidate for p nor p'
            if state
                .prepared_prime
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|prepared_prime_ballot| {
                    candidate.less_and_compatible(prepared_prime_ballot)
                })
            {
                continue;
            }

            if state
                .prepared
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|prepared_ballot| candidate.less_and_compatible(prepared_ballot))
            {
                continue;
            }

            // There is a chance it increases p'
        }
        todo!()
    }

    fn set_accept_prepared(state: &mut BallotProtocolState, prepared: &SCPBallot) -> bool {
        todo!()
    }

    fn attempt_confirm_prepared(state: &mut BallotProtocolState, hint: &SCPStatement) {
        todo!()
    }

    fn set_confirm_prepared(
        state: &mut BallotProtocolState,
        newC: &SCPBallot,
        newH: &SCPBallot,
    ) -> bool {
        todo!()
    }

    fn attempt_accept_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool {
        todo!()
    }

    fn set_accept_commit(state: &mut BallotProtocolState, c: &SCPBallot, h: &SCPBallot) -> bool {
        todo!()
    }

    fn attempt_confirm_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool {
        todo!()
    }

    fn set_confirm_commit(
        state: &mut BallotProtocolState,
        acceptCommitLow: &SCPBallot,
        acceptCommitHigh: &SCPBallot,
    ) -> bool {
        todo!()
    }

    fn attemptBump() -> bool {
        todo!()
    }
}
