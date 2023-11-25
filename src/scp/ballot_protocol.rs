use std::{
    borrow::{BorrowMut, Cow},
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet},
    hash::Hash,
    sync::{Arc, Mutex},
};

use crate::scp::{
    local_node::{self, LocalNode},
    nomination_protocol::NominationProtocol,
    scp_driver::SCPDriver,
};

use super::{
    nomination_protocol::{HNominationProtocolState, HNominationValue, NominationValue},
    scp::{NodeID, SCPEnvelope, SCP},
    scp_driver::{HSCPEnvelope, HashValue, SlotDriver},
    slot::SlotIndex,
    statement::{SCPStatement, SCPStatementConfirm, SCPStatementExternalize, SCPStatementPrepare},
};

pub trait ToBallot {
    fn to_ballot(&self) -> SCPBallot;
}
impl SCPStatement {
    fn ballot_counter(&self) -> u32 {
        match self {
            SCPStatement::Prepare(st) => st.ballot.counter,
            SCPStatement::Confirm(st) => st.ballot.counter,
            SCPStatement::Externalize(st) => st.commit.counter,
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        }
    }

    fn is_statement_sane(&self) -> bool {
        match self {
            SCPStatement::Prepare(st) => {
                // Statement from self is allowed to have b = 0 (as long as it never gets emitted)
                if !(st.from_self || st.ballot.counter > 0) {
                    return false;
                }

                // If prepared_prime and prepared are not None, then prepared_prime should be less and incompatible with prepared.
                if let Some(prepared) = st.prepared.as_ref() {
                    if let Some(prepared_prime) = st.prepared_prime.as_ref() {
                        if !prepared_prime.less_and_incompatible(prepared) {
                            return false;
                        }
                    }
                }

                // high ballot counter number should be 0 or no greater than the prepared counter (in which case the prepared ballot field is set).
                if !(st.num_high == 0
                    || st
                        .prepared
                        .as_ref()
                        .is_some_and(|prepared| st.num_high <= prepared.counter))
                {
                    return false;
                }

                // c != 0 -> c <= h <= b
                st.num_commit == 0
                    || (st.num_high != 0
                        && st.ballot.counter >= st.num_high
                        && st.num_high >= st.num_commit)
            }
            SCPStatement::Confirm(st) => {
                st.ballot.counter > 0
                    && st.num_high <= st.ballot.counter
                    && st.num_commit <= st.num_high
            }
            SCPStatement::Externalize(st) => {
                st.commit.counter > 0 && st.num_high >= st.commit.counter
            }
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        }
    }
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
}

pub trait BallotProtocol {
    fn advance_slot(self: &Arc<Self>, state: &HNominationProtocolState, hint: &SCPStatement);

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
        nomination_state_handle: &HNominationProtocolState,
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
    fn attempt_bump(self: &Arc<Self>, state_handle: &HBallotProtocolState) -> bool;
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

    pub message_level: u32,
}

type Interval = (u32, u32);

impl BallotProtocolState {
    // TODO: needs to figure out how it works....
    fn find_extended_interval(
        boundaries: &BTreeSet<u32>,
        candidate: &mut Interval,
        predicate: impl Fn(&Interval) -> bool,
    ) {
        for b in boundaries.iter().rev() {
            let mut cur: Interval = (0, 0);
            if candidate.0 == 0 {
                // First, find the high bound
                cur = (*b, *b);
            } else if b > &candidate.1 {
                // invalid
                continue;
            } else {
                cur.0 = *b;
                cur.1 = candidate.1;
            }

            if predicate(&cur) {
                *candidate = cur;
            } else if candidate.0 != 0 {
                break;
            }
        }
    }

    fn get_commit_boundaries_from_statements(&self, ballot: &SCPBallot) -> BTreeSet<u32> {
        let mut ret = BTreeSet::new();
        self.latest_envelopes
            .values()
            .into_iter()
            .for_each(|envelope| match envelope.lock().unwrap().get_statement() {
                SCPStatement::Prepare(st) => {
                    if ballot.compatible(&st.ballot) {
                        if st.num_commit > 0 {
                            ret.insert(st.num_commit);
                            ret.insert(st.num_high);
                        }
                    }
                }
                SCPStatement::Confirm(st) => {
                    if ballot.compatible(&st.ballot) {
                        ret.insert(st.num_commit);
                        ret.insert(st.num_high);
                    }
                }
                SCPStatement::Externalize(st) => {
                    if ballot.compatible(&st.commit) {
                        ret.insert(st.commit.counter);
                        ret.insert(st.num_high);
                        ret.insert(std::u32::MAX);
                    }
                }
                SCPStatement::Nominate(_) => {
                    panic!("Nomination statement encountered in ballot protocol.")
                }
            });
        ret
    }

    // This function gives a set of ballots containing candidate values that we can accept based on current state and the hint SCP statement.
    fn get_prepare_candidates(&self, hint: &SCPStatement) -> BTreeSet<SCPBallot> {
        let mut hint_ballots = BTreeSet::new();

        // Get ballots
        let _ = match hint {
            SCPStatement::Prepare(st) => {
                hint_ballots.insert(st.ballot.clone());

                if let Some(prepard) = &st.prepared {
                    hint_ballots.insert(prepard.clone());
                }

                if let Some(prepared_prime) = &st.prepared_prime {
                    hint_ballots.insert(prepared_prime.clone());
                }
            }
            SCPStatement::Confirm(st) => {
                hint_ballots.insert(SCPBallot {
                    counter: st.num_prepared,
                    value: st.ballot.value.clone(),
                });
                hint_ballots.insert(SCPBallot {
                    counter: std::u32::MAX,
                    value: st.ballot.value.clone(),
                });
            }
            SCPStatement::Externalize(st) => {
                hint_ballots.insert(SCPBallot {
                    counter: std::u32::MAX,
                    value: st.commit.value.clone(),
                });
            }
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        };

        let mut candidates = BTreeSet::new();

        // TODO: I am not entirely clear about the logic of this part, soneed to add more documentation.
        hint_ballots.iter().rev().for_each(|top_vote| {
            // find candidates that may have been prepared
            self.latest_envelopes
                .values()
                .into_iter()
                .for_each(|envelope| match envelope.lock().unwrap().get_statement() {
                    SCPStatement::Prepare(st) => {
                        if st.ballot.less_and_compatible(top_vote) {
                            candidates.insert(st.ballot.clone());
                        }
                        if let Some(prepared_ballot) = &st.prepared {
                            if prepared_ballot.less_and_compatible(top_vote) {
                                candidates.insert(prepared_ballot.clone());
                            }
                        }

                        if let Some(prepared_prime_ballot) = &st.prepared_prime {
                            if prepared_prime_ballot.less_and_compatible(top_vote) {
                                candidates.insert(prepared_prime_ballot.clone());
                            }
                        }
                    }
                    SCPStatement::Confirm(st) => {
                        if top_vote.compatible(&st.ballot) {
                            candidates.insert(top_vote.clone());
                            if st.num_prepared < top_vote.counter {
                                candidates.insert(SCPBallot {
                                    counter: st.num_prepared,
                                    value: top_vote.value.clone(),
                                });
                            }
                        }
                    }
                    SCPStatement::Externalize(st) => {
                        if st.commit.compatible(top_vote) {
                            candidates.insert(top_vote.clone());
                        }
                    }
                    SCPStatement::Nominate(_) => {
                        panic!("Nomination statement encountered in ballot protocol.")
                    }
                });
        });

        candidates
    }

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

    fn check_invariants(&self) {
        {
            match self.phase {
                SCPPhase::PhasePrepare => {}
                _ => {
                    // Confirm or Externalize phases
                    assert!(self.current_ballot.lock().unwrap().is_some());
                    assert!(self.prepared.lock().unwrap().is_some());
                    assert!(self.commit.lock().unwrap().is_some());
                    assert!(self.high_ballot.lock().unwrap().is_some());
                }
            }

            if let Some(current_ballot) = self.current_ballot.lock().unwrap().as_ref() {
                assert!(current_ballot.counter != 0);
            }

            if let Some(prepared) = self.prepared.lock().unwrap().as_ref() {
                if let Some(prepared_prime) = self.prepared_prime.lock().unwrap().as_ref() {
                    assert!(prepared_prime.less_and_compatible(prepared));
                }
            }

            if let Some(high_ballot) = self.high_ballot.lock().unwrap().as_ref() {
                assert!(high_ballot.less_and_compatible(
                    self.current_ballot
                        .lock()
                        .unwrap()
                        .as_ref()
                        .expect("Current ballot is not None")
                ));
            }

            if let Some(commit) = self.commit.lock().unwrap().as_ref() {
                assert!(self.current_ballot.lock().unwrap().is_some());
                assert!(commit.less_and_compatible(
                    self.high_ballot
                        .lock()
                        .unwrap()
                        .as_ref()
                        .expect("High ballot is not None")
                ));
                assert!(self
                    .high_ballot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("High ballot is not None")
                    .less_and_compatible(
                        self.current_ballot
                            .lock()
                            .unwrap()
                            .as_ref()
                            .expect("Current ballot is not None")
                    ));
            }
        }
    }

    // This update current ballot and other related fields according to the new high ballot.
    fn update_current_if_needed(&mut self, high_ballot: &SCPBallot) -> bool {
        if self
            .current_ballot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|current_ballot| current_ballot > high_ballot)
        {
            true
        } else {
            self.bump_to_ballot(true, high_ballot);
            false
        }
    }

    fn bump_to_ballot(&mut self, require_monotone: bool, ballot: &SCPBallot) {
        assert!(self.phase != SCPPhase::PhaseExternalize);

        if require_monotone {
            if let Some(current_ballot) = self.current_ballot.lock().unwrap().as_ref() {
                assert!(ballot >= current_ballot);
            }
        }

        let got_bumped = self.current_ballot.lock().unwrap().is_none()
            || self
                .current_ballot
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|current_ballot| current_ballot.counter != ballot.counter);

        if self.current_ballot.lock().unwrap().is_none() {
            // Start ballot protocol
        }

        *self.current_ballot.lock().unwrap() = Some(ballot.clone());

        // note: we have to clear some fields (and recompute them based on latest
        // messages)
        // invariant: h.value = b.value
        if self
            .high_ballot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|high_ballot| !high_ballot.compatible(ballot))
        {
            *self.high_ballot.lock().unwrap() = None;
            *self.commit.lock().unwrap() = None;
        }

        if got_bumped {
            self.heard_from_quorum = false;
        }
    }

    fn create_statement(&self, local_quorum_set_hash: HashValue) -> SCPStatement {
        self.check_invariants();

        match self.phase {
            SCPPhase::PhasePrepare => {
                let num_commit = if let Some(val) = self.commit.lock().unwrap().as_ref() {
                    val.counter.clone()
                } else {
                    0
                };
                let num_high = if let Some(val) = self.commit.lock().unwrap().as_ref() {
                    val.counter.clone()
                } else {
                    0
                };

                SCPStatement::Prepare(SCPStatementPrepare {
                    quorum_set_hash: local_quorum_set_hash,
                    ballot: self
                        .current_ballot
                        .lock()
                        .unwrap()
                        .as_ref()
                        .expect("Current ballot")
                        .clone(),
                    prepared: self.prepared.lock().unwrap().clone(),
                    prepared_prime: self.prepared_prime.lock().unwrap().clone(),
                    num_commit: num_commit,
                    num_high: num_high,
                    from_self: true,
                })
            }
            SCPPhase::PhaseConfirm => SCPStatement::Confirm(SCPStatementConfirm {
                quorum_set_hash: local_quorum_set_hash,
                ballot: self
                    .current_ballot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("Current ballot")
                    .clone(),
                num_prepared: self
                    .prepared
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("Prepared")
                    .counter
                    .clone(),
                num_commit: self
                    .prepared
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("Commit")
                    .counter
                    .clone(),
                num_high: self
                    .high_ballot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("High ballot")
                    .counter
                    .clone(),
            }),
            SCPPhase::PhaseExternalize => SCPStatement::Externalize(SCPStatementExternalize {
                commit_quorum_set_hash: local_quorum_set_hash,
                commit: self
                    .commit
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("Commit")
                    .clone(),
                num_high: self
                    .high_ballot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("High ballot")
                    .counter
                    .clone(),
            }),
        }
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
            message_level: Default::default(),
        }
    }
}

struct BallotProtocolUtils {}

impl BallotProtocolUtils {
    fn commit_predicate(ballot: &SCPBallot, check: &Interval, statement: &SCPStatement) -> bool {
        match statement {
            SCPStatement::Prepare(st) => false,
            SCPStatement::Confirm(st) => {
                if ballot.compatible(&st.ballot) {
                    st.num_high <= check.0 && check.1 <= st.num_high
                } else {
                    false
                }
            }
            SCPStatement::Externalize(st) => {
                if ballot.compatible(&st.commit) {
                    st.commit.counter <= check.0
                } else {
                    false
                }
            }
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        }
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
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        }
    }
}

impl SlotDriver {
    const MAXIMUM_ADVANCE_SLOT_RECURSION: u32 = 50;

    fn advance_slot(
        self: &Arc<Self>,
        ballot_state_handle: &HBallotProtocolState,
        nomination_state_handle: &HNominationProtocolState,
        hint: &SCPStatement,
    ) {
        ballot_state_handle.lock().unwrap().message_level -= 1;
        if ballot_state_handle.lock().unwrap().message_level
            >= SlotDriver::MAXIMUM_ADVANCE_SLOT_RECURSION
        {
            panic!("maximum number of transitions reached in advance_slot");
        }

        let mut did_work = false;

        did_work = self.attempt_accept_commit(ballot_state_handle, hint) || did_work;
        did_work = self.attempt_confirm_prepared(ballot_state_handle, hint) || did_work;
        did_work = self.attempt_accept_commit(ballot_state_handle, hint) || did_work;
        did_work = self.attempt_confirm_commit(nomination_state_handle, ballot_state_handle, hint)
            || did_work;

        // only bump after we're done with everything else
        if ballot_state_handle.lock().unwrap().message_level == 1 {
            let mut did_bump = false;
            loop {
                did_bump = self.attempt_bump(ballot_state_handle);
                did_work = did_bump || did_work;
                if !did_bump {
                    break;
                }
            }
        }

        ballot_state_handle.lock().unwrap().message_level -= 1;

        if did_work {}
    }

    fn emit_current_state_statement(self: &Arc<Self>, state: &mut BallotProtocolState) {
        let statement = state.create_statement(
            self.local_node
                .lock()
                .unwrap()
                .get_quorum_set()
                .hash_value(),
        );

        let mut can_emit = state.current_ballot.lock().unwrap().is_some();
        let local_node_id = self.local_node.lock().unwrap().node_id.clone();

        // if we generate the same envelope, don't process it again
        // this can occur when updating h in PREPARE phase
        // as statements only keep track of h.n (but h.x could be different)
        if let Some(last_envelope) = state
            .latest_envelopes
            .values()
            .find(|envelope| envelope.lock().unwrap().node_id == local_node_id)
        {}

        todo!()
    }

    fn has_v_blocking_subset_strictly_ahead_of(
        self: &Arc<Self>,

        envelopes: &BTreeMap<NodeID, HSCPEnvelope>,
        counter: u32,
    ) -> bool {
        let local_node = self.local_node.lock().unwrap();
        LocalNode::is_v_blocking(&local_node.quorum_set, envelopes, &|st: &SCPStatement| {
            st.ballot_counter() > counter
        })
    }

    // This method abandons the current ballot and sets the state according to state counter n.
    fn abandon_ballot(self: &Arc<Self>, state: &mut BallotProtocolState, n: u32) -> bool {
        match self.get_latest_composite_value().lock().unwrap().as_ref() {
            Some(value) => {
                if n == 0 {
                    self.bump_state(state, value, true)
                } else {
                    self.bump_state_with_counter(state, value, n)
                }
            }
            None => false,
        }
    }

    fn bump_state(
        self: &Arc<Self>,
        state: &mut BallotProtocolState,
        nomination_value: &NominationValue,
        force: bool,
    ) -> bool {
        if !force && state.current_ballot.lock().unwrap().is_none() {
            false
        } else {
            let n = if let Some(current_ballot) = state.current_ballot.lock().unwrap().as_ref() {
                current_ballot.counter
            } else {
                1
            };
            self.bump_state_with_counter(state, nomination_value, n)
        }
    }

    fn bump_state_with_counter(
        self: &Arc<Self>,
        state: &mut BallotProtocolState,
        nomination_value: &NominationValue,
        n: u32,
    ) -> bool {
        if state.phase == SCPPhase::PhaseExternalize {
            return false;
        }

        let value = match state.value_override.lock().unwrap().as_ref() {
            Some(value_override) => value_override.clone(),
            None => nomination_value.clone(),
        };

        let mut new_ballot = SCPBallot {
            counter: n,
            value: value,
        };

        let mut updated = self.update_current_value(state, &new_ballot);

        if updated {
            self.emit_current_state_statement(state);
            self.check_heard_from_quorum(state);
        }

        updated
    }

    // updates the local state based to the specified ballot
    // (that could be a prepared ballot) enforcing invariants
    fn update_current_value(
        self: &Arc<Self>,
        state: &mut BallotProtocolState,
        ballot: &SCPBallot,
    ) -> bool {
        if state.phase == SCPPhase::PhaseExternalize {
            return false;
        }

        let mut updated = false;
        match state.current_ballot.lock().unwrap().as_ref() {
            None => {
                updated = true;
            }
            Some(current_ballot) => {
                // Canonot update if the ballot is incompatible with current commit.
                if state
                    .commit
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|commit| !commit.compatible(ballot))
                {
                    return false;
                }

                if current_ballot < ballot {
                    updated = true;
                } else {
                    if current_ballot > ballot {
                        dbg! {"BallotProtocol::updateCurrentValue attempt to bump to
                        a smaller value"};
                        return false;
                    }
                }
            }
        }

        if updated {
            state.bump_to_ballot(true, ballot);
        }

        state.check_invariants();

        updated
    }

    fn process_envelope(self: &Arc<Self>, state: &mut BallotProtocolState, envelope: &SCPEnvelope) {
        assert!(envelope.slot_index == self.slot_index);
    }

    fn check_heard_from_quorum(self: &Arc<Self>, state: &mut BallotProtocolState) {
        // this method is safe to call regardless of the transitions of the
        // other nodes on the network: we guarantee that other nodes can only
        // transition to higher counters (messages are ignored upstream)
        // therefore the local node will not flip flop between "seen" and "not
        // seen" for a given counter on the local node
        if let Some(current_ballot) = state.current_ballot.lock().unwrap().as_ref() {
            let heard_predicate = |statement: &SCPStatement| match statement {
                SCPStatement::Prepare(st) => current_ballot.counter <= st.ballot.counter,
                SCPStatement::Confirm(_) => true,
                SCPStatement::Externalize(_) => true,
                SCPStatement::Nominate(_) => {
                    panic!("Nomination statement encountered in ballot protocol.")
                }
            };

            if LocalNode::is_quorum(
                &self.local_node.lock().unwrap().quorum_set,
                &state.latest_envelopes,
                heard_predicate,
            ) {
                let old_heard_from_quorum = state.heard_from_quorum;
                state.heard_from_quorum = true;
                if !old_heard_from_quorum {
                    // if we transition from not heard -> heard, we start the
                    // timer
                    todo!()
                }
            } else {
                state.heard_from_quorum = false;
            }
        }
    }
}

impl BallotProtocol for SlotDriver {
    fn attempt_accept_prepared(
        self: &Arc<Self>,
        state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let state = state_handle.lock().unwrap();
        if state.phase != SCPPhase::PhasePrepare && state.phase != SCPPhase::PhaseConfirm {
            return false;
        }

        let candidates = state.get_prepare_candidates(hint);

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
                        SCPStatement::Nominate(_) => {
                            panic!("Nomination statement encountered in ballot protocol.")
                        }
                    }
                },
                |st| BallotProtocolUtils::has_prepared_ballot(&candidate, st),
                &state.latest_envelopes,
            ) {
                return self.set_accept_prepared(state_handle, &candidate);
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
            let mut candidates = state.get_prepare_candidates(hint);

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
        did_work = did_work || state.update_current_if_needed(new_high);

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
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
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

        let predicate = |cur: &Interval| -> bool {
            self.federated_accept(
                |_st: &SCPStatement| match _st {
                    SCPStatement::Prepare(st) => {
                        if ballot.compatible(&st.ballot) && st.num_commit != 0 {
                            st.num_commit <= cur.0 && cur.1 <= st.num_high
                        } else {
                            false
                        }
                    }
                    SCPStatement::Confirm(st) => {
                        if ballot.compatible(&st.ballot) {
                            st.num_commit <= cur.0
                        } else {
                            false
                        }
                    }
                    SCPStatement::Externalize(st) => {
                        if ballot.compatible(&st.commit) {
                            st.commit.counter <= cur.0
                        } else {
                            false
                        }
                    }
                    SCPStatement::Nominate(_) => {
                        panic!("Nomination statement encountered in ballot protocol.")
                    }
                },
                |st: &SCPStatement| BallotProtocolUtils::commit_predicate(&ballot, cur, st),
                &state.latest_envelopes,
            )
        };

        let boundaries = state.get_commit_boundaries_from_statements(&ballot);
        if boundaries.is_empty() {
            return false;
        }

        let mut candidate: Interval = (0, 0);

        BallotProtocolState::find_extended_interval(&boundaries, &mut candidate, predicate);

        // TODO: I didn't quite follow this part.
        if candidate.0 != 0 {
            if state.phase != SCPPhase::PhaseConfirm
                || candidate.1
                    > state
                        .high_ballot
                        .lock()
                        .unwrap()
                        .as_ref()
                        .expect("High ballot")
                        .counter
            {
                let commit_ballot = SCPBallot {
                    counter: candidate.0,
                    value: ballot.value.clone(),
                };
                let high_ballot = SCPBallot {
                    counter: candidate.1,
                    value: ballot.value.clone(),
                };
                self.set_accept_commit(state_handle, &commit_ballot, &high_ballot)
            } else {
                false
            }
        } else {
            false
        }
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
                state.bump_to_ballot(false, high);
            }

            *state.prepared_prime.lock().unwrap() = None;
            did_work = true;
        }

        if did_work {
            state.update_current_if_needed(high);
            self.accepted_commit(&self.slot_index, high);
            self.emit_current_state_statement(&mut state);
        }

        did_work
    }

    fn attempt_confirm_commit(
        self: &Arc<Self>,
        nomination_state_handle: &HNominationProtocolState,
        ballot_state_handle: &HBallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {
        let mut state = ballot_state_handle.lock().unwrap();
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
            SCPStatement::Nominate(_) => {
                panic!("Nomination statement encountered in ballot protocol.")
            }
        };

        if !state
            .commit
            .lock()
            .unwrap()
            .as_ref()
            .expect("Commit ballot")
            .compatible(&ballot)
        {
            return false;
        }

        let boundaries = state.get_commit_boundaries_from_statements(&ballot);
        let mut candidate: Interval = (0, 0);
        let predicate = |cur: &Interval| {
            self.federated_ratify(
                |statement: &SCPStatement| {
                    BallotProtocolUtils::commit_predicate(&ballot, cur, statement)
                },
                &state.latest_envelopes,
            )
        };

        if candidate.0 != 0 {
            let commit_ballot = SCPBallot {
                counter: candidate.0,
                value: ballot.value.clone(),
            };
            let high_ballot = SCPBallot {
                counter: candidate.1,
                value: ballot.value.clone(),
            };
            self.set_confirm_commit(
                ballot_state_handle,
                nomination_state_handle,
                &commit_ballot,
                &high_ballot,
            )
        } else {
            false
        }
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
        state.update_current_if_needed(accept_commit_high);

        state.phase = SCPPhase::PhaseExternalize;

        self.emit_current_state_statement(&mut state);

        let mut nomination_state = nomination_state_handle.lock().unwrap();
        self.stop_nomination(&mut nomination_state);

        self.value_externalized(
            self.slot_index,
            &state.commit.lock().unwrap().as_ref().expect("").value,
        );

        true
    }

    // Step 9 from the paper (Feb 2016):
    //
    //   If ∃ S ⊆ M such that the set of senders {v_m | m ∈ S} is v-blocking
    //   and ∀m ∈ S, b_m.n > b_v.n, then set b <- <n, z> where n is the lowest
    //   counter for which no such S exists.
    //
    // a.k.a 4th rule for setting ballot.counter in the internet-draft (v03):
    //
    //   If nodes forming a blocking threshold all have ballot.counter values
    //   greater than the local ballot.counter, then the local node immediately
    //   cancels any pending timer, increases ballot.counter to the lowest
    //   value such that this is no longer the case, and if appropriate
    //   according to the rules above arms a new timer. Note that the blocking
    //   threshold may include ballots from SCPCommit messages as well as
    //   SCPExternalize messages, which implicitly have an infinite ballot
    //   counter.
    fn attempt_bump(self: &Arc<Self>, state_handle: &HBallotProtocolState) -> bool {
        let mut state = state_handle.lock().unwrap();
        if state.phase == SCPPhase::PhasePrepare || state.phase == SCPPhase::PhaseConfirm {
            let local_counter = match state.current_ballot.lock().unwrap().as_ref() {
                Some(local_ballot) => local_ballot.counter,
                None => 0,
            };

            // First check to see if this condition applies at all. If there
            // is no v-blocking set ahead of the local node, there's nothing
            // to do, return early.
            if !self.has_v_blocking_subset_strictly_ahead_of(&state.latest_envelopes, local_counter)
            {
                return false;
            }

            let mut all_counters = BTreeSet::new();
            for entry in &state.latest_envelopes {
                let counter = entry.1.lock().unwrap().get_statement().ballot_counter();
                if counter > local_counter {
                    all_counters.insert(counter);
                }
            }

            // If we got to here, implicitly there _was_ a v-blocking subset
            // with counters above the local counter; we just need to find a
            // minimal n at which that's no longer true. So check them in
            // order, starting from the smallest.
            for counter in all_counters {
                if !self.has_v_blocking_subset_strictly_ahead_of(&state.latest_envelopes, counter) {
                    return self.abandon_ballot(&mut state, counter);
                }
            }

            // Unreachable
            false
        } else {
            true
        }
    }

    fn advance_slot(
        self: &Arc<Self>,
        state_handle: &HNominationProtocolState,
        hint: &SCPStatement,
    ) {
    }
}
