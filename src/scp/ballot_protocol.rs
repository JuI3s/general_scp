use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::scp::scp_driver::SCPDriver;

use super::{
    nomination_protocol::{HNominationValue, NominationValue},
    scp::{NodeID, SCP},
    scp_driver::{HSCPEnvelope, SlotDriver},
};

pub enum StatementType<'a> {
    Prepare(&'a SCPBallot),
    Confirm(&'a SCPBallot),
    Externalize(&'a SCPBallot),
}
pub struct SCPStatement {}

impl SCPStatement {
    pub fn get_pledge_type<'a>(&'a self) -> StatementType<'a> {
        todo!()
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct SCPBallot {
    counter: u32,
    value: NominationValue,
}

impl SCPBallot {
    pub fn compatible(&self, other: &SCPBallot) -> bool {
        self.value == other.value
    }

    pub fn less_and_incompatible(&self, other: &SCPBallot) -> bool {
        self <= other && !self.compatible(other)
    }

    pub fn less_and_compatible(&self, other: &SCPBallot) -> bool {
        self <= other && self.compatible(other)
    }
}

impl Default for SCPBallot {
    fn default() -> Self {
        Self {
            counter: Default::default(),
            value: Default::default(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum SCPPhase {
    PhasePrepare,
    PhaseConfirm,
    PhaseExternalize,
    PhaseNum,
}

pub trait BallotProtocol {
    fn externalize(&mut self);
    fn recv_ballot_envelope(&mut self);

    // `attempt*` methods are called by `advanceSlot` internally call the
    //  the `set*` methods.
    //   * check if the specified state for the current slot has been
    //     reached or not.
    //   * idempotent
    //  input: latest statement received (used as a hint to reduce the
    //  space to explore)
    //  output: returns true if the state was updated

    // `set*` methods progress the slot to the specified state
    //  input: state specific
    //  output: returns true if the state was updated.

    // step 1 and 5 from the SCP paper
    fn attempt_accept_prepared(
        self: &Arc<Self>,
        state: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool;
    // prepared: ballot that should be prepared
    fn set_accept_prepared(
        self: &Arc<Self>,
        state: &HBallotProtocolState,
        ballot: &SCPBallot,
    ) -> bool;

    // step 2+3+8 from the SCP paper
    // ballot is the candidate to record as 'confirmed prepared'
    fn attempt_confirm_prepared(state: &mut BallotProtocolState, hint: &SCPStatement);
    // newC, newH : low/high bounds prepared confirmed
    fn set_confirm_prepared(
        state: &mut BallotProtocolState,
        newC: &SCPBallot,
        newH: &SCPBallot,
    ) -> bool;

    // step (4 and 6)+8 from the SCP paper
    fn attempt_accept_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool;
    // new values for c and h
    fn set_accept_commit(state: &mut BallotProtocolState, c: &SCPBallot, h: &SCPBallot) -> bool;

    // step 7+8 from the SCP paper
    fn attempt_confirm_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool;
    fn set_confirm_commit(
        state: &mut BallotProtocolState,
        acceptCommitLow: &SCPBallot,
        acceptCommitHigh: &SCPBallot,
    ) -> bool;

    // step 9 from the SCP paper
    fn attemptBump() -> bool;
}

pub type HBallot = Arc<Mutex<Option<SCPBallot>>>;

pub type HBallotProtocolState = Arc<Mutex<BallotProtocolState>>;
pub struct BallotProtocolState {
    pub heard_from_quorum: bool,

    pub current_ballot: HBallot,
    pub prepared: HBallot,
    pub prepared_prime: HBallot,
    pub high_ballot: HBallot,
    pub commit: HBallot,

    pub latest_envelopes: BTreeMap<NodeID, HSCPEnvelope>,
    pub phase: SCPPhase,
    pub value_override: HNominationValue,

    pub current_message_level: usize,

    // last envelope generated by this node
    pub last_envelope: HSCPEnvelope,

    // last envelope emitted by this node
    pub last_envelope_emitted: HSCPEnvelope,
}

impl BallotProtocolState {
    fn set_prepared(&mut self, ballot: &SCPBallot) -> bool {
        let mut did_work = false;

        match self.prepared.lock().unwrap().as_ref() {
            Some(ref mut prepared_ballot) => {
                if *prepared_ballot < ballot {
                    // as we're replacing p, we see if we should also replace p'
                    if !prepared_ballot.compatible(ballot) {
                        self.prepared_prime = Arc::new(Mutex::new(Some(prepared_ballot.clone())));
                    }
                    *prepared_ballot = ballot;
                    did_work = true;
                } else if *prepared_ballot > ballot {
                    // check if we should update only p', this happens
                    // either p' was None
                    // or p' gets replaced by ballot
                    //      (p' < ballot and ballot is incompatible with p)
                    // note, the later check is here out of paranoia as this function is
                    // not called with a value that would not allow us to make progress

                    if self.prepared_prime.lock().unwrap().is_none()
                        || (self.prepared_prime.lock().unwrap().as_ref().expect("") < ballot
                            && !self
                                .prepared
                                .lock()
                                .unwrap()
                                .as_ref()
                                .expect("")
                                .compatible(ballot))
                    {
                        *self
                            .prepared_prime
                            .lock()
                            .unwrap()
                            .as_ref()
                            .as_mut()
                            .expect("") = ballot;
                        did_work = true;
                    }
                }
            }
            None => {
                self.prepared_prime = Arc::new(Mutex::new(None));
                did_work = true;
            }
        };
        did_work
    }
}

impl Default for BallotProtocolState {
    fn default() -> Self {
        Self {
            heard_from_quorum: Default::default(),
            current_ballot: Default::default(),
            prepared: Default::default(),
            prepared_prime: Default::default(),
            high_ballot: Default::default(),
            commit: Default::default(),
            latest_envelopes: Default::default(),
            phase: SCPPhase::PhasePrepare,
            value_override: Default::default(),
            current_message_level: Default::default(),
            last_envelope: Default::default(),
            last_envelope_emitted: Default::default(),
        }
    }
}

impl SlotDriver {
    fn get_prepare_candidates(hint: &SCPStatement) -> BTreeSet<SCPBallot> {
        todo!()
    }
}

impl SlotDriver {
    fn emit_current_state_statement(self: &Arc<Self>, state: &mut BallotProtocolState) {
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
        state_handle: &HBallotProtocolState,
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
            if self.federated_accept(
                |st| {
                    let ballot = &candidate;
                    match st.get_pledge_type() {
                        crate::scp::ballot_protocol::StatementType::Prepare(prepare) => {
                            ballot.less_and_compatible(prepare)
                        }
                        crate::scp::ballot_protocol::StatementType::Confirm(confirm) => {
                            ballot.less_and_compatible(confirm)
                        }
                        crate::scp::ballot_protocol::StatementType::Externalize(externalize) => {
                            ballot.compatible(externalize)
                        }
                    }
                },
                |st| {
                    let ballot = &candidate;
                    todo!()
                },
                &state.latest_envelopes,
            ) {
                return self.set_accept_prepared(state_handle, candidate);
            }
        }
        false
    }

    fn set_accept_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        ballot: &SCPBallot,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        let mut did_work = state.set_prepared(ballot);

        if state.commit.lock().unwrap().is_some() && state.high_ballot.lock().unwrap().is_some() {
            if state.prepared.lock().unwrap().as_ref().is_some_and(|prepared_ballot| {
                    state.high_ballot.lock().unwrap().as_ref().expect("").less_and_incompatible(prepared_ballot)
                }
                || state.prepared_prime.lock().unwrap().as_ref().is_some_and(|prepared_prime_ballot|{
                    state.high_ballot.lock().unwrap().as_ref().expect("").less_and_incompatible(prepared_prime_ballot)
                })
                ) {
                    state.commit = Arc::new(Mutex::new(None));
                    did_work = true
                }
        }
        if did_work {
            self.accepted_ballot_prepared(&self.slot_index, ballot);
            self.emit_current_state_statement(&mut state);
        }
        did_work
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
