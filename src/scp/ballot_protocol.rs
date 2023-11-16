use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::scp::{scp_driver::SCPDriver, nomination_protocol::NominationProtocol};

use super::{
    nomination_protocol::{HNominationValue, NominationValue, HNominationProtocolState},
    scp::{NodeID, SCP},
    scp_driver::{HSCPEnvelope, Hash, SlotDriver},
};

pub trait ToBallot {
    fn to_ballot(&self) -> SCPBallot;
}

pub enum SCPStatement {
    Prepare(SCPStatementPrepare),
    Confirm(SCPStatementConfirm),
    Externalize(SCPStatementExternalize),
}

pub struct SCPStatementPrepare {
    quorum_set_hash: Hash,
    ballot: SCPBallot,
    prepared: Option<SCPBallot>,
    prepared_prime: Option<SCPBallot>,
    num_commit: u32,
    num_high: u32,
}

pub struct SCPStatementConfirm {
    quorum_set_hash: Hash,
    ballot: SCPBallot,
    num_prepared: u32,
    num_commit: u32,
    num_high: u32,
}

pub struct SCPStatementExternalize {
    commit_quorum_set_hash: Hash,
    commit: SCPBallot,
    num_high: u32,
}

// TODO: Probably make this generic using macros?
impl ToBallot for SCPStatementPrepare {
    fn to_ballot(&self) -> SCPBallot {
        SCPBallot {
            counter: self.num_high,
            value: self.ballot.value.clone(),
        }
    }
}

impl ToBallot for SCPStatementConfirm {
    fn to_ballot(&self) -> SCPBallot {
        SCPBallot {
            counter: self.num_high,
            value: self.ballot.value.clone(),
        }
    }
}

impl ToBallot for SCPStatementExternalize {
    fn to_ballot(&self) -> SCPBallot {
        SCPBallot {
            counter: self.num_high,
            value: self.commit.value.clone(),
        }
    }
}

pub struct SPCStatementCommit {}

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct SCPBallot {
    counter: u32,
    value: NominationValue,
}

impl SCPBallot {
    pub fn make_ballot(other: &SCPBallot) -> Self {
        SCPBallot {
            counter: other.counter,
            value: other.value.clone(),
        }
    }

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
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool;
    // prepared: ballot that should be prepared
    fn set_accept_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        ballot: &SCPBallot,
    ) -> bool;

    // step 2+3+8 from the SCP paper
    // ballot is the candidate to record as 'confirmed prepared'
    fn attempt_confirm_prepared(
        self: &Arc<Self>,
        state: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool;
    // newC, newH : low/high bounds prepared confirmed
    fn set_confirm_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        newC: &SCPBallot,
        newH: &SCPBallot,
    ) -> bool;

    // step (4 and 6)+8 from the SCP paper
    fn attempt_accept_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool;
    // new values for c and h
    fn set_accept_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        c: &SCPBallot,
        h: &SCPBallot,
    ) -> bool;

    // step 7+8 from the SCP paper
    fn attempt_confirm_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool;
    fn set_confirm_commit(
        self: &Arc<Self>,
        ballot_state_handle: &HBallotProtocolState,
        nomination_state_handle: &HNominationProtocolState,
        acceptCommitLow: &SCPBallot,
        acceptCommitHigh: &SCPBallot,
    ) -> bool;

    // step 9 from the SCP paper
    fn attempt_bump() -> bool;
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
    pub value_override: Arc<Mutex<Option<NominationValue>>>,

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

struct BallotProtocolUtils {}

impl BallotProtocolUtils {
    fn get_prepare_candidates(hint: &SCPStatement) -> BTreeSet<SCPBallot> {
        todo!()
    }

    fn has_prepared_ballot(ballot: &SCPBallot, statement: &SCPStatement) -> bool {
        match statement {
            SCPStatement::Prepare(st) => {
                st.prepared
                    .as_ref()
                    .is_some_and(|prepared| ballot.less_and_compatible(&prepared))
                    || st
                        .prepared_prime
                        .as_ref()
                        .is_some_and(|prepared_prime| ballot.less_and_compatible(&prepared_prime))
            }
            SCPStatement::Confirm(st) => ballot.less_and_compatible(&SCPBallot {
                counter: st.num_prepared,
                value: st.ballot.value.clone(),
            }),
            SCPStatement::Externalize(st) => ballot.compatible(&st.commit),
        }
    }
}

impl SlotDriver {
    fn emit_current_state_statement(self: &Arc<Self>, state: &mut BallotProtocolState) {
        todo!()
    }

    // helper to perform step (8) from the paper
    fn update_current_if_needed(
        self: &Arc<Self>,
        state: &mut BallotProtocolState,
        h: &SCPBallot,
    ) -> bool {
        todo!();
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

        let candidates = BallotProtocolUtils::get_prepare_candidates(hint);

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
                    match st {
                        SCPStatement::Prepare(st) => ballot.less_and_compatible(&st.ballot),
                        SCPStatement::Confirm(st) => ballot.less_and_compatible(&st.ballot),
                        SCPStatement::Externalize(st) => ballot.compatible(&st.commit),
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

    fn attempt_confirm_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let state = state_handle.lock().unwrap();
        if state.phase != SCPPhase::PhasePrepare {
            return false;
        }

        let prepared_ballot_opt = state.prepared.lock().unwrap();

        if let Some(prepared_ballot) = prepared_ballot_opt.as_ref() {
            let mut candidates = BallotProtocolUtils::get_prepare_candidates(hint);

            if let Some(new_high) = candidates.iter().find(|&candidate| {
                if state
                    .high_ballot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|hb| hb >= candidate)
                {
                    false
                } else {
                    let ratified =
                        |st: &SCPStatement| BallotProtocolUtils::has_prepared_ballot(candidate, st);
                    self.federated_ratify(ratified, &state.latest_envelopes)
                }
            }) {
                let b = match state.current_ballot.lock().unwrap().as_ref() {
                    Some(ballot) => ballot.clone(),
                    None => SCPBallot::default(),
                };

                // now, look for newC (left as 0 if no update) step (3) from the paper.
                let mut new_commit = SCPBallot::default();

                if state.commit.lock().unwrap().is_none()
                    && !state
                        .prepared
                        .lock()
                        .unwrap()
                        .as_ref()
                        .is_some_and(|prepared| new_high.less_and_incompatible(prepared))
                    && !state.prepared_prime.lock().unwrap().as_ref().is_some_and(
                        |prepared_prime| new_high.less_and_incompatible(prepared_prime),
                    )
                {
                    // TODO: maybe rewrite this logic in a more functional programming way...
                    for candidate in &candidates {
                        if candidate < &b {
                            break;
                        }

                        if !candidate.less_and_compatible(new_high) {
                            continue;
                        }

                        let voted_predicate = |st: &SCPStatement| {
                            BallotProtocolUtils::has_prepared_ballot(candidate, st)
                        };

                        if self.federated_ratify(voted_predicate, &state.latest_envelopes) {
                            new_commit = candidate.clone();
                        } else {
                            break;
                        }
                    }
                }
                return self.set_confirm_prepared(&state_handle, &new_commit, new_high);
            }
            false
        } else {
            false
        }
    }

    fn set_confirm_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        new_commit: &SCPBallot,
        new_high: &SCPBallot,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        *state.value_override.lock().unwrap() = Some(new_high.value.clone());

        let mut did_work = false;

        if !state
            .current_ballot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|current_ballot| !current_ballot.compatible(new_high))
        {
            if let Some(high_ballot) = state.high_ballot.lock().unwrap().as_mut() {
                *high_ballot = SCPBallot::make_ballot(new_high);
                did_work = true;
            }

            if new_commit.counter != 0 {
                *state.commit.lock().unwrap() = Some(SCPBallot::make_ballot(new_commit));
                did_work = true;
            }

            if did_work {
                self.confirm_ballot_prepared(&self.slot_index, new_high);
            }
        }

        // always perform step (8) with the computed value of h
        did_work = did_work || self.update_current_if_needed(&mut state, new_high);

        if did_work {
            self.emit_current_state_statement(&mut state);
        }

        did_work
    }

    fn attempt_accept_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        if state.phase != SCPPhase::PhasePrepare && state.phase != SCPPhase::PhaseConfirm {
            return false;
        }

        let ballot = match hint {
            SCPStatement::Prepare(st) => {
                if st.num_commit != 0 {
                    st.to_ballot()
                } else {
                    return false;
                }
            }
            SCPStatement::Confirm(st) => st.to_ballot(),
            SCPStatement::Externalize(st) => st.to_ballot(),
        };

        if state.phase == SCPPhase::PhaseConfirm
            && !state
                .high_ballot
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|high_ballot| high_ballot.compatible(&ballot))
        {
            return false;
        }
        todo!()
    }

    fn set_accept_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        commit: &SCPBallot,
        high: &SCPBallot,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        let mut did_work = false;

        *state.value_override.lock().unwrap() = Some(high.value.clone());

        if !state
            .high_ballot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|high_ballot| high_ballot == high)
            || !state
                .commit
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|commit_ballot| commit_ballot == commit)
        {
            *state.commit.lock().unwrap() = Some(commit.clone());
            *state.high_ballot.lock().unwrap() = Some(high.clone());
            did_work = true;
        }

        if state.phase == SCPPhase::PhasePrepare {
            state.phase = SCPPhase::PhaseConfirm;
            if state
                .current_ballot
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|current_ballot| !high.less_and_compatible(current_ballot))
            {
                // Bump to ballot
                did_work = true;
                todo!();
            }
        }

        if did_work {
            self.update_current_if_needed(&mut state, high);
            self.accepted_commit(&self.slot_index, high);
            self.emit_current_state_statement(&mut state);
        }

        did_work
    }

    fn attempt_confirm_commit(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        if state.phase != SCPPhase::PhaseConfirm {
            return false;
        }

        if state.high_ballot.lock().unwrap().is_none() || state.commit.lock().unwrap().is_none() {
            return false;
        }

        let ballot = match hint {
            SCPStatement::Prepare(st) => {
                return false;
            }
            SCPStatement::Confirm(st) => st.to_ballot(),
            SCPStatement::Externalize(st) => st.to_ballot(),
        };

        if !state
            .commit
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|commit| commit.compatible(&ballot))
        {
            return false;
        }

        todo!()
    }

    fn set_confirm_commit(
        self: &Arc<Self>,
        ballot_state_handle: &HBallotProtocolState,
        nomination_state_handle: &HNominationProtocolState,
        accept_commit_low: &SCPBallot,
        accept_commit_high: &SCPBallot,
    ) -> bool {
        let mut state = ballot_state_handle.lock().unwrap();

        *state.commit.lock().unwrap() = Some(accept_commit_low.clone());
        *state.high_ballot.lock().unwrap() = Some(accept_commit_high.clone());
        self.update_current_if_needed(&mut state, accept_commit_high);
    
        state.phase = SCPPhase::PhaseExternalize;

        self.emit_current_state_statement(&mut state);

        let mut nomination_state = nomination_state_handle.lock().unwrap(); 
        self.stop_nomination(&mut nomination_state);

        self.value_externalized(self.slot_index, &state.commit.lock().unwrap().as_ref().expect("").value);

        true 
    }

    fn attempt_bump() -> bool {
        todo!()
    }
}
