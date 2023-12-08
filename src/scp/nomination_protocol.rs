use std::{
    collections::hash_map::DefaultHasher,
    hash::{self, Hash, Hasher},
    ops::Deref,
    sync::Weak,
};

use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, BTreeSet, HashSet},
    rc::Rc,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use log::debug;
use tokio::time::timeout;

use crate::{
    application::work_queue::ClockEvent, herder::herder::HerderDriver, overlay::peer::PeerID,
    scp::slot, utils::weak_self::WeakSelf,
};

use super::{
    scp::{EnvelopeState, NodeID},
    scp_driver::{HSCPEnvelope, SCPDriver, SlotDriver, ValidationLevel},
    slot::Slot,
    statement::{SCPStatement, SCPStatementNominate},
};

pub trait NominationProtocol {
    fn nominate(
        self: &Arc<Self>,
        state: HNominationProtocolState,
        value: HNominationValue,
        previous_value: &NominationValue,
    ) -> bool;
    fn stop_nomination(self: &Arc<Self>, state: &mut NominationProtocolState);

    fn update_round_learders(&mut self);

    fn get_json_info(&self);

    fn process_envelope(
        self: &Arc<Self>,
        state_handle: &HNominationProtocolState,
        envelope: &HSCPEnvelope,
    ) -> EnvelopeState;
}

type HNominationEnvelope = Arc<Mutex<NominationEnvelope>>;
struct NominationEnvelope {}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct NominationValue {}

impl Default for NominationValue {
    fn default() -> Self {
        Self {}
    }
}

pub type HNominationValue = Arc<NominationValue>;
pub type HLatestCompositeCandidateValue = Arc<Mutex<Option<NominationValue>>>;
pub type NominationValueSet = BTreeSet<HNominationValue>;

pub type HNominationProtocolState = Arc<Mutex<NominationProtocolState>>;
// TODO: double check these fields are correct
// #[derive(WeakSelf)]
pub struct NominationProtocolState {
    pub round_number: u64,
    pub votes: NominationValueSet,
    pub accepted: NominationValueSet,
    pub candidates: NominationValueSet,
    pub latest_nominations: BTreeMap<String, HSCPEnvelope>,

    pub latest_envelope: Option<HSCPEnvelope>,
    pub round_leaders: BTreeSet<String>,

    pub nomination_started: bool,
    pub latest_composite_candidate: HLatestCompositeCandidateValue,
    pub previous_value: NominationValue,

    pub num_timeouts: usize,
    pub timed_out: bool,
}

impl Default for NominationProtocolState {
    fn default() -> Self {
        Self {
            round_number: Default::default(),
            votes: Default::default(),
            accepted: Default::default(),
            candidates: Default::default(),
            latest_nominations: Default::default(),
            latest_envelope: Default::default(),
            round_leaders: Default::default(),
            nomination_started: Default::default(),
            latest_composite_candidate: Default::default(),
            previous_value: Default::default(),
            num_timeouts: Default::default(),
            timed_out: Default::default(),
        }
    }
}

impl SCPStatement {
    fn as_nomination_statement(&self) -> &SCPStatementNominate {
        match self {
            SCPStatement::Nominate(st) => st,
            _ => panic!("Not a nomination statement."),
        }
    }

    fn get_accepted(&self) -> Vec<NominationValue> {
        match self {
            SCPStatement::Nominate(st) => st.accepted.clone(),
            _ => panic!("Not a nomination statement."),
        }
    }

    fn get_votes(&self) -> Vec<NominationValue> {
        match self {
            SCPStatement::Nominate(st) => st.votes.clone(),
            _ => panic!("Not a nomination statement."),
        }
    }
}

impl NominationProtocolState {
    // TODO: I really need to make local_id a part of nomination state.
    fn gather_votes_from_round_leaders(
        &mut self,
        slot_index: &u64,
        local_id: &NodeID,
        extract_valid_value_predicate: &impl Fn(&NominationValue) -> Option<NominationValue>,
        validate_value_predicate: &impl Fn(&NominationValue) -> ValidationLevel,
        nominating_value_predicate: &impl Fn(&NominationValue),
    ) -> bool {
        let mut updated = false;

        for leader in &self.round_leaders {
            if let Some(nomination) = self.latest_nominations.get(leader) {
                if let Some(new_value) = self.get_new_value_form_nomination(
                    nomination
                        .lock()
                        .unwrap()
                        .get_statement()
                        .as_nomination_statement(),
                    |value| extract_valid_value_predicate(value),
                    |value| validate_value_predicate(value),
                ) {
                    self.votes.insert(new_value.to_owned().into());
                    updated = true;
                    nominating_value_predicate(&new_value);
                }
            }
        }

        updated
    }

    fn get_statement_values(&self, statement: &SCPStatementNominate) -> Vec<NominationValue> {
        let mut ret = Vec::new();
        apply_all(statement, |value: &NominationValue| ret.push(value.clone()));
        ret
    }

    fn is_newer_statement(&self, node_id: &NodeID, statement: &SCPStatementNominate) -> bool {
        if let Some(envelope) = self.latest_nominations.get(node_id) {
            envelope
                .lock()
                .unwrap()
                .get_statement()
                .as_nomination_statement()
                .is_older_than(statement)
        } else {
            true
        }
    }

    // Returns true if we have processed a statement newer than s
    fn processed_newer_statement(
        &self,
        node_id: &NodeID,
        statement: &SCPStatementNominate,
    ) -> bool {
        if let Some(envelope) = self.latest_nominations.get(node_id) {
            statement.is_older_than(
                envelope
                    .lock()
                    .unwrap()
                    .get_statement()
                    .as_nomination_statement(),
            )
        } else {
            false
        }
    }

    fn is_sane(&self, statement: &SCPStatementNominate) -> bool {
        (statement.votes.len() + statement.accepted.len() != 0)
            && statement.votes.windows(2).all(|win| win[0] < win[1])
            && statement.accepted.windows(2).all(|win| win[0] < win[1])
    }

    fn get_new_value_form_nomination(
        &self,
        statement: &SCPStatementNominate,
        extract_valid_value_predicate: impl Fn(&NominationValue) -> Option<NominationValue>,
        validate_value_predicate: impl Fn(&NominationValue) -> ValidationLevel,
    ) -> Option<NominationValue> {
        let mut cur_hash: u64 = 0;
        let mut cur_value: Option<NominationValue> = None;

        // TODO: Can we avoid copying?
        let mut pick_value = |value: &NominationValue| {
            if let Some(value_to_nominate) = match validate_value_predicate(value) {
                ValidationLevel::VoteToNominate => Some(value.to_owned()),
                ValidationLevel::FullyValidated => Some(value.to_owned()),
                _ => extract_valid_value_predicate(value),
            } {
                if !self.votes.contains(&value_to_nominate) {
                    let new_hash = hash_value(&value_to_nominate);
                    if new_hash > cur_hash {
                        cur_hash = new_hash;
                        cur_value = Some(value_to_nominate);
                    }
                }
                true
            } else {
                false
            }
        };

        if statement.accepted.iter().all(|val| !pick_value(val)) {
            statement.votes.iter().all(|val| pick_value(val));
        }

        cur_value
    }

    // pub fn add_value_from_leaders(&mut self, driver: &Arc<impl SCPDriver>) -> bool {
    //     let mut updated = false;
    //     for leader in &self.round_leaders {
    //         match self.latest_nominations.get(leader) {
    //             Some(nomination) => match self.get_new_value_form_nomination(nomination) {
    //                 Some(new_value) => {
    //                     driver.nominating_value(&new_value);
    //                     let new_value_handle = Arc::new(new_value);
    //                     self.votes.insert(new_value_handle);
    //                     updated = true;
    //                 }
    //                 None => {}
    //             },
    //             _ => (),
    //         }
    //     }
    //     updated
    // }

    // only called after a call to isNewerStatement so safe to replace the mLatestNomination
    fn record_envelope(&mut self, envelope: &HSCPEnvelope) {
        let nomination_env = envelope.lock().unwrap();
        let node_id = &nomination_env.node_id;
        if let Some(old_nomination) = self.latest_nominations.get(node_id).borrow_mut() {
            *old_nomination = &envelope.clone()
            // TODO: is this right?
        } else {
            self.latest_nominations
                .insert(node_id.to_string(), envelope.clone());
        }
        // TODO: record statement
    }

    fn set_state_from_envelope(&mut self, envelope: &HSCPEnvelope) {
        if self.nomination_started {
            panic!("Cannot set state after nomination is started.")
        }

        self.record_envelope(envelope);
        let nomination_env = envelope.lock().unwrap();
        let nomination_statement = nomination_env.get_statement();
        nomination_statement
            .get_accepted()
            .into_iter()
            .for_each(|statement| {
                self.accepted.insert(Arc::new(statement));
            });
        nomination_statement
            .get_votes()
            .into_iter()
            .for_each(|statement| {
                self.votes.insert(Arc::new(statement));
            });

        self.latest_envelope = Some(envelope.clone());
    }
}

fn accept_predicat(value: &NominationValue, statement: &SCPStatement) -> bool {
    statement.as_nomination_statement().accepted.contains(value)
}

fn apply_all(statement: &SCPStatementNominate, mut function: impl FnMut(&NominationValue)) {
    statement.votes.iter().for_each(|vote| function(vote));
    // Accepted should be a subset of votes.
    statement
        .accepted
        .iter()
        .for_each(|accepted| function(accepted));
}

fn hash_value(value: &NominationValue) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl<T: HerderDriver> SlotDriver<T> {
    fn emit_nomination(self: &Arc<Self>, state: &mut NominationProtocolState) {
        todo!()
    }
}

impl<T> NominationProtocol for SlotDriver<T>
where
    T: HerderDriver + 'static,
{
    fn nominate(
        self: &Arc<Self>,
        state_handle: HNominationProtocolState,
        value: HNominationValue,
        previous_value: &NominationValue,
    ) -> bool {
        let mut state = state_handle.lock().unwrap();
        if !state.candidates.is_empty() {
            debug!(
                "Skip nomination round {}, already have a candidate",
                state.round_number
            );
            return false;
        }

        let mut updated = false;

        if state.timed_out {
            state.num_timeouts += 1;
        }

        if state.timed_out && !state.nomination_started {
            debug!("NominationProtocol::nominate (TIMED OUT)");
            return false;
        }

        state.nomination_started = true;
        state.previous_value = previous_value.clone();
        state.round_number += 1;

        let timeout: std::time::Duration = self.herder_driver.compute_timeout(state.round_number);

        let local_node = &self.local_node.lock().unwrap();

        updated = updated
            || state.gather_votes_from_round_leaders(
                &self.slot_index,
                &local_node.node_id,
                &|value| self.herder_driver.extract_valid_value(value),
                &|value| self.herder_driver.validate_value(value, true),
                &|value| self.herder_driver.nominating_value(value, &self.slot_index),
            );

        // if we're leader, add our value if we haven't added any votes yet
        if state.round_leaders.contains(&local_node.node_id) && state.votes.is_empty() {
            state.votes.insert(value.clone().into());
            updated = true;
            self.herder_driver
                .nominating_value(&value, &self.slot_index);
        }

        // state.add_value_from_leaders(self);

        // if we're leader, add our value if we haven't added any votes yet
        if state
            .round_leaders
            .contains(&self.local_node.lock().unwrap().node_id)
            && state.votes.is_empty()
        {
            if state.votes.insert(value.clone()) {
                updated = true;
                self.nominating_value(value.as_ref());
            }
        }

        let weak_self = Arc::downgrade(self);
        let weak_state = Arc::downgrade(&state_handle.clone());
        let value_copy = value.clone();
        let prev_value_copy = previous_value.clone();

        let callback = move || match weak_self.upgrade() {
            Some(slot_driver) => match weak_state.upgrade() {
                Some(state) => {
                    slot_driver.nominate(state, value_copy, &prev_value_copy);
                }
                None => todo!(),
            },
            None => todo!(),
        };

        let clock_event = ClockEvent::new(SystemTime::now() + timeout, Box::new(callback));
        self.timer.lock().unwrap().add_task(clock_event);

        if updated {
            self.emit_nomination(&mut state);
        } else {
            debug!("NominationProtocol::nominate (SKIPPED");
        }

        updated
    }

    fn stop_nomination(self: &Arc<Self>, state: &mut NominationProtocolState) {
        state.nomination_started = false;
    }

    fn update_round_learders(&mut self) {
        let local_id = &self.local_node.lock().unwrap().node_id;

        let max_leader_count = &self.local_node.lock().unwrap().quorum_set;

        todo!()
    }

    fn get_json_info(&self) {
        todo!()
    }

    fn process_envelope(
        self: &Arc<Self>,
        state_handle: &HNominationProtocolState,
        envelope: &HSCPEnvelope,
    ) -> EnvelopeState {
        let env = envelope.lock().unwrap();
        let node_id = &env.node_id;
        let statement = env.get_statement().as_nomination_statement();
        let mut state = state_handle.lock().unwrap();

        // TODO: this comment seems to be wrong
        // If we've processed the same envelope, we'll process it again
        // since the validity of values might have changed
        // (e.g., tx set fetch)

        if state.processed_newer_statement(&node_id, statement) {
            return EnvelopeState::Invalid;
        }

        if !state.is_sane(statement) {
            return EnvelopeState::Invalid;
        }

        state.record_envelope(envelope);

        if state.nomination_started {
            // Whether we have modified nomination state.
            let modified = statement.votes.iter().any(|vote| {
                if state.accepted.contains(vote) {
                    return false;
                }
                if self.federated_accept(
                    |st: &SCPStatement| st.as_nomination_statement().votes.contains(vote),
                    |st: &SCPStatement| accept_predicat(vote, st),
                    &state.latest_nominations,
                ) {
                    match self.herder_driver.validate_value(vote, true) {
                        ValidationLevel::FullyValidated => {
                            let value = Arc::new(vote.clone());
                            state.accepted.insert(value.clone());
                            state.votes.insert(value.clone());
                            return true;
                        }
                        _ => {
                            if let Some(value) = self.herder_driver.extract_valid_value(vote) {
                                state.accepted.insert(Arc::new(value.clone()));
                                state.votes.insert(Arc::new(value.clone()));
                                return true;
                            }
                        }
                    }
                }
                false
            });

            let new_candidates = statement.accepted.iter().any(|value| {
                if state.candidates.contains(value) {
                    return false;
                }
                if self.federated_ratify(
                    |st: &SCPStatement| accept_predicat(value, st),
                    &state.latest_nominations,
                ) {
                    state.candidates.insert(Arc::new(value.clone()));
                    todo!();
                    // Stop timer.
                    return true;
                }
                false
            });

            if modified {
                self.emit_nomination(&mut state);
            }

            if new_candidates {
                // TODO: Is this correct?

                if let Some(value) = self.herder_driver.combine_candidates(&state.candidates) {}

                *state.latest_composite_candidate.lock().unwrap() =
                    self.herder_driver.combine_candidates(&state.candidates);
                let _ = match state.latest_composite_candidate.lock().unwrap().as_ref() {
                    Some(val) => {
                        self.bump_state_(val, false);
                    }
                    None => {}
                };
            }
        }
        EnvelopeState::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
