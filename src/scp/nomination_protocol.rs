use std::{
    collections::hash_map::DefaultHasher,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use log::debug;
use serde::Serialize;

use crate::{
    herder::herder::HerderDriver,
    overlay::peer::PeerID,
};

use super::{
    ballot_protocol::BallotProtocolState,
    envelope::{SCPEnvelopeController, SCPEnvelopeID},
    queue::{RetryNominateArg, SlotJob, SlotTask},
    scp::{EnvelopeState, NodeID},
    scp_driver::{SCPDriver, SlotDriver, SlotStateTimer, ValidationLevel},
    slot::SlotIndex,
    statement::{SCPStatement, SCPStatementNominate},
};

pub trait NominationProtocol<N>
where
    N: NominationValue,
{
    fn nominate(
        self: &Arc<Self>,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> Option<SCPEnvelopeID>;
    fn stop_nomination(self: &Arc<Self>, state: &mut NominationProtocolState<N>);

    fn update_round_learders(&mut self);

    fn process_nomination_envelope(
        self: &Arc<Self>,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
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
    pub latest_nominations: BTreeMap<String, SCPEnvelopeID>,

    pub latest_envelope: Option<SCPEnvelopeID>,
    pub round_leaders: BTreeSet<String>,

    pub nomination_started: bool,
    pub latest_composite_candidate: HLatestCompositeCandidateValue<N>,
    pub previous_value: N,

    pub num_timeouts: u64,
    pub timed_out: bool,
}

impl<N: NominationValue> NominationProtocolState<N> {
    pub fn new(leader_id: PeerID) -> Self {
        let mut state: NominationProtocolState<N> = Default::default();
        state.round_leaders.insert(leader_id);
        state
    }
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
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        let mut updated = false;

        for leader in &self.round_leaders {
            if let Some(nomination) = self
                .latest_nominations
                .get(leader)
                .and_then(|env_id| envelope_controller.get_envelope(env_id))
            {
                if let Some(new_value) = self.get_new_value_form_nomination(
                    nomination.get_statement().as_nomination_statement(),
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
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        if let Some(env_id) = self.latest_nominations.get(node_id) {
            envelope_controller
                .get_envelope(env_id)
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
        envelope_controller: &SCPEnvelopeController<N>,
    ) -> bool {
        if let Some(env_id) = self.latest_nominations.get(node_id) {
            statement.is_older_than(
                envelope_controller
                    .get_envelope(env_id)
                    .unwrap()
                    .get_statement()
                    .as_nomination_statement(),
            )
        } else {
            false
        }
    }

    fn is_sane(&self, statement: &SCPStatementNominate<N>) -> bool {
        (statement.votes.len() + statement.accepted.len() != 0)
            && statement
                .votes
                .windows(2)
                .all(|window| window[0] < window[1])
            && statement
                .accepted
                .windows(2)
                .all(|window| window[0] < window[1])
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
    fn record_envelope(
        &mut self,
        env_id: &SCPEnvelopeID,
        envelope_controller: &SCPEnvelopeController<N>,
    ) {
        let nomination_env = envelope_controller.get_envelope(env_id).unwrap();
        let node_id = &nomination_env.node_id;

        self.latest_nominations
            .insert(node_id.to_string(), env_id.clone());

        // TODO: record statement
        // I think it's not needed for SCP - just some routine bookkeeping.
    }

    fn set_state_from_envelope(
        &mut self,
        env_id: &SCPEnvelopeID,
        envelope_controller: &SCPEnvelopeController<N>,
    ) {
        if self.nomination_started {
            panic!("Cannot set state after nomination is started.")
        }

        self.record_envelope(env_id, envelope_controller);
        let nomination_env = envelope_controller.get_envelope(env_id).unwrap();
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

        self.latest_envelope = Some(env_id.clone());
    }

    fn get_current_votes(&self) -> Vec<N> {
        let mut votes = vec![];

        self.votes.iter().for_each(|vote: &Arc<N>| {
            votes.push(vote.as_ref().clone());
        });
        self.accepted.iter().for_each(|accepted| {
            votes.push(accepted.as_ref().clone());
        });
        votes
    }
}

impl<N, H> SlotDriver<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    fn emit_nomination(
        self: &Arc<Self>,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> Option<SCPEnvelopeID> {
        // This function creats a nomination statement that contains the current
        // nomination value. The statement is then wrapped in an SCP envelope which is
        // checked for validity before being passed to Herder for broadcasting.

        let local_node = self.local_node.borrow();

        let votes = nomination_state.get_current_votes();

        // Creating the nomination statement
        let nom_st: SCPStatementNominate<N> =
            SCPStatementNominate::<N>::new(&local_node.quorum_set, votes);

        // Creating the envelop
        let st = SCPStatement::Nominate(nom_st);

        let env_id = self.create_envelope(st, envelope_controller);

        // Process the envelope. This may triggers more envelops being emitted.
        match self.process_nomination_envelope(
            nomination_state,
            ballot_state,
            &env_id,
            envelope_controller,
        ) {
            EnvelopeState::Valid => {
                if nomination_state
                    .latest_envelope
                    .as_ref()
                    .and_then(|env_id| envelope_controller.get_envelope(env_id))
                    .is_some_and(|env| match &env.statement {
                        SCPStatement::Nominate(st) => {
                            st.is_older_than(env.statement.as_nomination_statement())
                        }
                        _ => {
                            panic!("Nomination state should only contain nomination statements.")
                        }
                    })
                {
                    // Fix this

                    // Do not do anything if we have already emitted a newer evenlope.
                    return None;
                }

                nomination_state.latest_envelope = Some(env_id.clone());

                if self.slot_state.borrow().fully_validated {
                    Some(env_id)
                } else {
                    None
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
        state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> Option<SCPEnvelopeID> {
        if !state.candidates.is_empty() {
            debug!(
                "Skip nomination round {}, already have a candidate",
                state.round_number
            );
            return None;
        }

        let mut updated = false;

        if state.timed_out {
            state.num_timeouts += 1;
        }

        if state.timed_out && !state.nomination_started {
            debug!("NominationProtocol::nominate (TIMED OUT)");
            return None;
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
                    envelope_controller,
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

        {
            // Create renominating task.
            let renominate_task_arg = RetryNominateArg {
                slot_idx: self.slot_index.clone(),
                value: value.clone(),
                previous_value: previous_value.clone(),
            };

            let renominate_task = SlotTask::RetryNominate(renominate_task_arg);
            let renominate_job = SlotJob {
                id: self.slot_index.clone(),
                timestamp: SystemTime::now() + timeout,
                task: renominate_task,
            };

            self.task_queue.borrow_mut().submit(renominate_job);
        }

        if updated {
            println!("Updated");

            self.emit_nomination(state, ballot_state, envelope_controller)
        } else {
            debug!("NominationProtocol::nominate (SKIPPED)");
            None
        }
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
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) -> EnvelopeState {
        // todo!();

        let env = envelope_controller.get_envelope(envelope).unwrap();
        let node_id = &env.node_id;
        let statement = env.get_statement().as_nomination_statement();

        // TODO: this comment seems to be wrong
        // If we've processed the same envelope, we'll process it again
        // since the validity of values might have changed
        // (e.g., tx set fetch)

        if nomination_state.processed_newer_statement(&node_id, statement, envelope_controller) {
            return EnvelopeState::Invalid;
        }

        if !nomination_state.is_sane(statement) {
            return EnvelopeState::Invalid;
        }

        nomination_state.record_envelope(envelope, envelope_controller);

        if nomination_state.nomination_started {
            // Whether we have modified nomination state.
            let modified = statement.votes.iter().any(|vote| {
                if nomination_state.accepted.contains(vote) {
                    return false;
                }
                if self.federated_accept(
                    |st| st.as_nomination_statement().votes.contains(vote),
                    |st| Self::accept_predicate(vote, st),
                    &nomination_state.latest_nominations,
                    envelope_controller,
                ) {
                    match self.herder_driver.borrow().validate_value(vote, true) {
                        ValidationLevel::FullyValidated => {
                            let value = Arc::new(vote.clone());
                            nomination_state.accepted.insert(value.clone());
                            nomination_state.votes.insert(value.clone());
                            return true;
                        }
                        _ => {
                            if let Some(value) =
                                self.herder_driver.borrow().extract_valid_value(vote)
                            {
                                nomination_state.accepted.insert(Arc::new(value.clone()));
                                nomination_state.votes.insert(Arc::new(value.clone()));
                                return true;
                            }
                        }
                    }
                }
                false
            });

            let new_candidates = statement.accepted.iter().any(|value| {
                if nomination_state.candidates.contains(value) {
                    return false;
                }

                if self.federated_ratify(
                    |st| Self::accept_predicate(value, st),
                    &nomination_state.latest_nominations,
                    envelope_controller,
                ) {
                    nomination_state.candidates.insert(Arc::new(value.clone()));

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
                self.emit_nomination(nomination_state, ballot_state, envelope_controller);
            }

            if new_candidates {
                // TODO: Is this correct?

                if let Some(value) = self
                    .herder_driver
                    .borrow()
                    .combine_candidates(&nomination_state.candidates)
                {}

                *nomination_state.latest_composite_candidate.lock().unwrap() = self
                    .herder_driver
                    .borrow()
                    .combine_candidates(&nomination_state.candidates);

                let _ = match nomination_state
                    .latest_composite_candidate
                    .clone()
                    .lock()
                    .unwrap()
                    .as_ref()
                {
                    Some(val) => {
                        self.bump_state_(
                            val,
                            ballot_state,
                            nomination_state,
                            false,
                            envelope_controller,
                        );
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
    
}
