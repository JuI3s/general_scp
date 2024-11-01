use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    fmt::Debug,
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

use bincode::de;
use log::{debug, info};
use serde::{Deserialize, Deserializer, Serialize};
use tokio::time::timeout;

use crate::{
    application::{quorum::QuorumSet, work_queue::ClockEvent},
    crypto::types::Blake2Hashable,
    herder::herder::HerderDriver,
    overlay::peer::PeerID,
    scp::{slot, statement::SCPStatementPrepare},
    utils::weak_self::WeakSelf,
};

use super::{
    scp::{EnvelopeState, NodeID},
    scp_driver::{HSCPEnvelope, SCPDriver, SlotDriver, SlotStateTimer, ValidationLevel},
    slot::{Slot, SlotIndex},
    statement::{SCPStatement, SCPStatementNominate},
};

pub trait NominationProtocol<N>
where
    N: NominationValue,
{
    fn nominate(
        self: &Arc<Self>,
        state: HNominationProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
    ) -> bool;
    fn stop_nomination(self: &Arc<Self>, state: &mut NominationProtocolState<N>);

    fn update_round_learders(&mut self);

    fn process_nomination_envelope(
        self: &Arc<Self>,
        state_handle: &HNominationProtocolState<N>,
        envelope: &HSCPEnvelope<N>,
    ) -> EnvelopeState;
}

type HNominationEnvelope = Arc<Mutex<NominationEnvelope>>;
struct NominationEnvelope {}

pub trait NominationValue:
    Clone + PartialEq + PartialOrd + Eq + Ord + Hash + Default + Serialize + 'static + Default + Debug
{
}

#[derive(Default, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize)]
pub struct SCPNominationValue {}

impl NominationValue for SCPNominationValue {}

pub type HSCPNominationValue<N> = Arc<N>;
pub type HLatestCompositeCandidateValue<N> = Arc<Mutex<Option<N>>>;
pub type SCPNominationValueSet<N> = BTreeSet<HSCPNominationValue<N>>;

pub type HNominationProtocolState<N> = Arc<Mutex<NominationProtocolState<N>>>;
pub struct NominationProtocolState<N>
where
    N: NominationValue,
{
    pub round_number: SlotIndex,
    pub votes: SCPNominationValueSet<N>,
    pub accepted: SCPNominationValueSet<N>,
    pub candidates: SCPNominationValueSet<N>,
    pub latest_nominations: BTreeMap<String, HSCPEnvelope<N>>,

    pub latest_envelope: Option<HSCPEnvelope<N>>,
    pub round_leaders: BTreeSet<String>,

    pub nomination_started: bool,
    pub latest_composite_candidate: HLatestCompositeCandidateValue<N>,
    pub previous_value: N,

    pub num_timeouts: u64,
    pub timed_out: bool,
}

impl<N> Default for NominationProtocolState<N>
where
    N: NominationValue,
{
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

impl<N> SCPStatement<N>
where
    N: NominationValue,
{
    fn as_nomination_statement(&self) -> &SCPStatementNominate<N> {
        match self {
            SCPStatement::Nominate(st) => st,
            _ => panic!("Not a nomination statement."),
        }
    }

    fn get_accepted(&self) -> Vec<N> {
        match self {
            SCPStatement::Nominate(st) => st.accepted.clone(),
            _ => panic!("Not a nomination statement."),
        }
    }

    fn get_votes(&self) -> Vec<N> {
        match self {
            SCPStatement::Nominate(st) => st.votes.clone(),
            _ => panic!("Not a nomination statement."),
        }
    }
}

impl<N> NominationProtocolState<N>
where
    N: NominationValue,
{
    // TODO: I really need to make local_id a part of nomination state.
    fn gather_votes_from_round_leaders(
        &mut self,
        slot_index: &SlotIndex,
        local_id: &NodeID,
        extract_valid_value_predicate: &impl Fn(&N) -> Option<N>,
        validate_value_predicate: &impl Fn(&N) -> ValidationLevel,
        nominating_value_predicate: &impl Fn(&N),
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

    fn get_statement_values(&self, statement: &SCPStatementNominate<N>) -> Vec<N> {
        let mut ret = Vec::new();
        Self::apply_all(statement, |value: &N| ret.push(value.clone()));
        ret
    }

    fn apply_all(statement: &SCPStatementNominate<N>, mut function: impl FnMut(&N)) {
        statement.votes.iter().for_each(|vote| function(vote));
        // Accepted should be a subset of votes.
        statement
            .accepted
            .iter()
            .for_each(|accepted| function(accepted));
    }

    fn is_newer_statement_for_node(
        &self,
        node_id: &NodeID,
        statement: &SCPStatementNominate<N>,
    ) -> bool {
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
        statement: &SCPStatementNominate<N>,
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

    fn is_sane(&self, statement: &SCPStatementNominate<N>) -> bool {
        info!("Checking if statement is sane");
        info!("Total votes: {}", statement.votes.len() + statement.accepted.len());
        (statement.votes.len() + statement.accepted.len() != 0)
            && statement.votes.windows(2).all(|win| win[0] < win[1])
            && statement.accepted.windows(2).all(|win| win[0] < win[1])
    }

    fn hash_value(value: &N) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    fn get_new_value_form_nomination(
        &self,
        statement: &SCPStatementNominate<N>,
        extract_valid_value_predicate: impl Fn(&N) -> Option<N>,
        validate_value_predicate: impl Fn(&N) -> ValidationLevel,
    ) -> Option<N> {
        let mut cur_hash = 0;
        let mut cur_value: Option<N> = None;

        // TODO: Can we avoid copying?
        let mut pick_value = |value: &N| {
            if let Some(value_to_nominate) = match validate_value_predicate(value) {
                ValidationLevel::VoteToNominate => Some(value.to_owned()),
                ValidationLevel::FullyValidated => Some(value.to_owned()),
                _ => extract_valid_value_predicate(value),
            } {
                if !self.votes.contains(&value_to_nominate) {
                    let new_hash = Self::hash_value(&value_to_nominate);
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

    // pub fn add_value_from_leaders(&mut self, driver: &Arc<impl SCPDriver>) ->
    // bool {     let mut updated = false;
    //     for leader in &self.round_leaders {
    //         match self.latest_nominations.get(leader) {
    //             Some(nomination) => match
    // self.get_new_value_form_nomination(nomination) {
    // Some(new_value) => {
    // driver.nominating_value(&new_value);                     let
    // new_value_handle = Arc::new(new_value);
    // self.votes.insert(new_value_handle);                     updated = true;
    //                 }
    //                 None => {}
    //             },
    //             _ => (),
    //         }
    //     }
    //     updated
    // }

    // only called after a call to isNewerStatement so safe to replace the
    // mLatestNomination
    fn record_envelope(&mut self, envelope: &HSCPEnvelope<N>) {
        info!("Recording envelope");
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
        // I think it's not needed for SCP - just some routine bookkeeping.
    }

    fn set_state_from_envelope(&mut self, envelope: &HSCPEnvelope<N>) {
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

impl<N, H> SlotDriver<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    fn emit_nomination(self: &Arc<Self>, state: &mut NominationProtocolState<N>) {
        // This function creats a nomination statement that contains the current
        // nomination value. The statement is then wrapped in an SCP envelope which is
        // checked for validity before being passed to Herder for broadcasting.

        let local_node = self.local_node.borrow();


        let mut votes = vec![];


        self.nomination_state()
            .lock()
            .unwrap()
            .votes
            .iter()
            .for_each(|vote: &Arc<N>| {
                votes.push(vote.as_ref().clone());
            });
        self.nomination_state()
            .lock()
            .unwrap()
            .accepted
            .iter()
            .for_each(|accepted| {
               votes.push(accepted.as_ref().clone());
            });

                // Creating the nomination statement
        let nom_st= SCPStatementNominate::<N>::new(&local_node.quorum_set, votes);
    

        // Creating the envelop
        let st = SCPStatement::Nominate(nom_st);
        let env = self.create_envelope(st).to_handle();


        // Process the envelope. This may triggers more envelops being emitted.
        let processed_result: EnvelopeState = self.process_nomination_envelope(&self.nomination_state(), &env);
        
        info!("Processed result: {:?}", processed_result);

        match processed_result {
            EnvelopeState::Valid => {
                let mut nomination_state = self.nomination_state().lock().unwrap();
                if !nomination_state
                    .latest_envelope
                    .as_ref()
                    .is_some_and(|env| match &env.lock().unwrap().statement {
                        SCPStatement::Nominate(st) => {
                            if env
                                .lock()
                                .unwrap()
                                .statement
                                .as_nomination_statement()
                                .is_older_than(&st)
                            {
                                false
                            } else {
                                true
                            }
                        }
                        _ => {
                            panic!("Nomination state should only contain nomination statements.")
                        }
                    })
                {
                    // Do not do anything if we have already emitted a newer evenlope.
                    return;
                }

                nomination_state.latest_envelope = Some(env.clone());
                if self.slot_state.borrow().fully_validated {
                    self.herder_driver
                        .borrow()
                        .emit_envelope(&env.lock().unwrap());
                }
            }
            EnvelopeState::Invalid => {
                panic!("Self issuing an invalid statement.")
            }
        }
    }

    fn accept_predicate(value: &N, statement: &SCPStatement<N>) -> bool {
        statement.as_nomination_statement().accepted.contains(value)
    }
}

impl<N, H> NominationProtocol<N> for SlotDriver<N, H>
where
    N: NominationValue + 'static,
    H: HerderDriver<N> + 'static,
{
    fn nominate(
        self: &Arc<Self>,
        state_handle: HNominationProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
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

        let timeout: std::time::Duration = self
            .herder_driver
            .borrow()
            .compute_timeout(state.round_number);

        {
            let local_node = &self.local_node.borrow();

            updated = updated
                || state.gather_votes_from_round_leaders(
                    &self.slot_index,
                    &local_node.node_id,
                    &|value| self.herder_driver.borrow().extract_valid_value(value),
                    &|value| self.herder_driver.borrow().validate_value(value, true),
                    &|value| {
                        self.herder_driver
                            .borrow()
                            .nominating_value(value, &self.slot_index)
                    },
                );

            // if we're leader, add our value if we haven't added any votes yet
            if state.round_leaders.contains(&local_node.node_id) && state.votes.is_empty() {
                state.votes.insert(value.clone().into());
                updated = true;
                self.herder_driver
                    .borrow()
                    .nominating_value(&value, &self.slot_index);
            }

            // state.add_value_from_leaders(self);

            // if we're leader, add our value if we haven't added any votes yet
            if state.round_leaders.contains(&local_node.node_id) && state.votes.is_empty() {
                if state.votes.insert(value.clone()) {
                    updated = true;
                    self.nominating_value(value.as_ref());
                }
            }
        }

        let weak_self = Arc::downgrade(self);
        let weak_state = Arc::downgrade(&state_handle.clone());
        let value_copy = value.clone();
        let prev_value_copy = previous_value.clone();

        let re_nominate_callback = move || match weak_self.upgrade() {
            Some(slot_driver) => match weak_state.upgrade() {
                Some(state) => {
                    slot_driver.nominate(state, value_copy, &prev_value_copy);
                }
                None => {}
            },
            None => {}
        };

        let timestamp = SystemTime::now() + timeout;
        let clock_event = ClockEvent::new(timestamp.to_owned(), Box::new(re_nominate_callback));
        self.slot_state
            .borrow_mut()
            .restart_timer(SlotStateTimer::NominationProtocol, clock_event.into());

        if updated {
            println!("Updated");
            self.emit_nomination(&mut state);
        } else {
            debug!("NominationProtocol::nominate (SKIPPED");
        }

        updated
    }

    fn stop_nomination(self: &Arc<Self>, state: &mut NominationProtocolState<N>) {
        state.nomination_started = false;
    }

    fn update_round_learders(&mut self) {
        let local_id = &self.local_node.borrow().node_id;

        let max_leader_count = &self.local_node.borrow().quorum_set;

        todo!()
    }

    fn process_nomination_envelope(
        self: &Arc<Self>,
        state_handle: &HNominationProtocolState<N>,
        envelope: &HSCPEnvelope<N>,
    ) -> EnvelopeState {
        info!("Processing nomination envelope");
        let env = envelope.lock().unwrap();
        let node_id = &env.node_id;
        let statement = env.get_statement().as_nomination_statement();
        let mut state = state_handle.lock().unwrap();

        // TODO: this comment seems to be wrong
        // If we've processed the same envelope, we'll process it again
        // since the validity of values might have changed
        // (e.g., tx set fetch)

        info!("br1");

        if state.processed_newer_statement(&node_id, statement) {
            return EnvelopeState::Invalid;
        }


        if !state.is_sane(statement) {
            info!("br1.2");
            return EnvelopeState::Invalid;
        }

        info!("br2");
        state.record_envelope(envelope);

        info!("br3");

        if state.nomination_started {
            // Whether we have modified nomination state.
            let modified = statement.votes.iter().any(|vote| {
                if state.accepted.contains(vote) {
                    return false;
                }
                if self.federated_accept(
                    |st| st.as_nomination_statement().votes.contains(vote),
                    |st| Self::accept_predicate(vote, st),
                    &state.latest_nominations,
                ) {
                    match self.herder_driver.borrow().validate_value(vote, true) {
                        ValidationLevel::FullyValidated => {
                            let value = Arc::new(vote.clone());
                            state.accepted.insert(value.clone());
                            state.votes.insert(value.clone());
                            return true;
                        }
                        _ => {
                            if let Some(value) =
                                self.herder_driver.borrow().extract_valid_value(vote)
                            {
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
                    |st| Self::accept_predicate(value, st),
                    &state.latest_nominations,
                ) {
                    state.candidates.insert(Arc::new(value.clone()));

                    // Stop the timer, as there's no need to continue nominating,
                    // per the whitepaper:
                    // "As soon as `v` has a candidate value, however, it must cease
                    // voting to nominate `x` for any new values `x`"
                    self.slot_state
                        .borrow_mut()
                        .stop_timer(&SlotStateTimer::NominationProtocol);

                    return true;
                }

                false
            });

            if modified {
                self.emit_nomination(&mut state);
            }

            if new_candidates {
                // TODO: Is this correct?

                if let Some(value) = self
                    .herder_driver
                    .borrow()
                    .combine_candidates(&state.candidates)
                {}

                *state.latest_composite_candidate.lock().unwrap() = self
                    .herder_driver
                    .borrow()
                    .combine_candidates(&state.candidates);
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
