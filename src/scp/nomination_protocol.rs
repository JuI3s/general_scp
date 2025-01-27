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

use bincode::de;
use log::{debug, info};
use serde::Serialize;
use tracing::field::debug;

use crate::{
    application::{
        quorum::accept_predicate,
        quorum_manager::{self, QuorumManager},
    },
    herder::{self, herder::HerderDriver},
    overlay::{node, peer::PeerID},
    utils::test::pretty_print_scp_env_id,
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

pub trait NominationProtocol<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    fn nominate(
        &self,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) -> Option<SCPEnvelopeID>;
    fn stop_nomination(&self, state: &mut NominationProtocolState<N>);

    fn update_round_learders(&mut self);

    fn process_nomination_envelope(
        &self,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
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

#[derive(Clone, Debug)]
pub struct NominationProtocolState<N>
where
    N: NominationValue,
{
    pub round_number: SlotIndex,
    pub votes: SCPNominationValueSet<N>,
    pub accepted: SCPNominationValueSet<N>,

    // https://johnpconley.com/wp-content/uploads/2021/01/stellar-consensus-protocol.pdf (p.19)
    // Definition (candidate). A node 𝑣 considers a value 𝑥 to be a candidate when 𝑣 has confirmed the statement nominate 𝑥—i.e., 𝑣 has ratified accept (nominate 𝑥).
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
        // todo!("Do not allow node to set itself as leader");
        println!("NominationProtocolState::new: leader_id: {:?}", leader_id);
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
    pub fn as_nomination_statement(&self) -> &SCPStatementNominate<N> {
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
        // TODO: need to fix the implementaion
        todo!();
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

        debug!(
            "node record_envelope {:?} from node {:?}, updating latest nomination",
            pretty_print_scp_env_id(env_id),
            node_id.to_string()
        );

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
        self: &Self,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) -> Option<SCPEnvelopeID> {
        // This function creats a nomination statement that contains the current
        // nomination value. The statement is then wrapped in an SCP envelope which is
        // checked for validity before being passed to Herder for broadcasting.

        info!(
            "emit_nomination: node {:?}, num accepted: {:?}, num votes: {:?}",
            self.node_idx(),
            nomination_state.accepted.len(),
            nomination_state.votes.len(),
        );

        let votes = nomination_state.get_current_votes();
        let accepted = Vec::from_iter(nomination_state.accepted.iter().map(|v| v.as_ref().clone()));

        // Creating the nomination statement
        let nom_st: SCPStatementNominate<N> =
            SCPStatementNominate::<N>::new(&self.local_node.quorum_set, votes, accepted);

        // Creating the envelop
        let st = SCPStatement::Nominate(nom_st);
        let cur_env_id = self.create_envelope(st, envelope_controller);

        // Process the envelope. This may triggers more envelops being emitted.
        let env_state = self.process_nomination_envelope(
            nomination_state,
            ballot_state,
            &cur_env_id,
            envelope_controller,
            quorum_manager,
            herder_driver,
        );

        match env_state {
            EnvelopeState::Valid => {
                if let Some(latest_envelope_id) = nomination_state.latest_envelope.as_ref() {
                    let env = envelope_controller
                        .get_envelope(latest_envelope_id)
                        .unwrap();
                    match &env.statement {
                        SCPStatement::Nominate(last_st) => {
                            if envelope_controller
                                .get_envelope(&cur_env_id)
                                .unwrap()
                                .statement
                                .as_nomination_statement()
                                .is_older_than(last_st)
                            {
                                debug!(
                                    "emit_nomination: node {:?} skipped statement with votes {:?} and accepts {:?} becauses it has already emitted newer envelope",
                                    self.node_idx(), last_st.votes, last_st.accepted,
                                );

                                return None;
                            }
                        }
                        _ => {
                            panic!("Nomination state should only contain nomination statements.")
                        }
                    };
                }

                debug!(
                    "emit_nomination: node {:?} emitted sets latest envelope {:?}",
                    self.node_idx(),
                    pretty_print_scp_env_id(&cur_env_id)
                );
                nomination_state.latest_envelope = Some(cur_env_id.clone());

                if self.slot_state.borrow().fully_validated {
                    Some(cur_env_id)
                } else {
                    None
                }
            }
            EnvelopeState::Invalid => {
                panic!("Self issuing an invalid statement.")
            }
        }
    }
}

impl<N, H> NominationProtocol<N, H> for SlotDriver<N, H>
where
    N: NominationValue + 'static,
    H: HerderDriver<N> + 'static,
{
    fn nominate(
        self: &Self,
        state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        value: HSCPNominationValue<N>,
        previous_value: &N,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) -> Option<SCPEnvelopeID> {
        debug!("NominationProtocol::nominate, node: {:?}", self.node_idx());
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

        let timeout: std::time::Duration = herder_driver.compute_timeout(state.round_number);

        {
            updated = updated
                || state.gather_votes_from_round_leaders(
                    &self.slot_index,
                    &self.local_node.node_id,
                    &|value| herder_driver.extract_valid_value(value),
                    &|value| herder_driver.validate_value(value, true),
                    &|value| herder_driver.nominating_value(value, &self.slot_index),
                    envelope_controller,
                );

            debug!(
                "NominationProtocol::nominate, updated after gathering votes from round leaders: {:?}, node {:?}",
                updated,
                self.node_idx()
            );

            // if we're leader, add our value if we haven't added any votes yet
            if state.round_leaders.contains(&self.local_node.node_id) && state.votes.is_empty() {
                state.votes.insert(value.clone().into());
                updated = true;
                herder_driver.nominating_value(&value, &self.slot_index);
                debug!(
                    "NominationProtocol::nominate, node {:?} adds value {:?} as leader, updated: {:?}",
                    self.node_idx(),
                    value,
                    updated
                );
            }

            // TODO: remove
            // state.add_value_from_leaders(self);
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

        debug!(
            "NominationProtocol::nominate, updated: {:?}, node {:?}",
            updated,
            self.node_idx()
        );
        if updated {
            self.emit_nomination(
                state,
                ballot_state,
                envelope_controller,
                quorum_manager,
                herder_driver,
            )
        } else {
            debug!(
                "NominationProtocol::nominate (SKIPPED), node {:?}",
                self.node_idx()
            );
            None
        }
    }

    fn stop_nomination(&self, state: &mut NominationProtocolState<N>) {
        state.nomination_started = false;
    }

    fn update_round_learders(&mut self) {
        let local_id = &self.local_node.node_id;

        let max_leader_count = &self.local_node.quorum_set;

        todo!()
    }

    fn process_nomination_envelope(
        &self,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope: &SCPEnvelopeID,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) -> EnvelopeState {
        debug!(
            "process_nomination_envelope: Node {:?} process nomination envelope {:?}",
            self.node_idx(),
            pretty_print_scp_env_id(&envelope),
        );
        let env = envelope_controller.get_envelope(envelope).unwrap();
        let node_id = &env.node_id;
        let statement = env.get_statement().as_nomination_statement();

        // TODO: this comment seems to be wrong
        // If we've processed the same envelope, we'll process it again
        // since the validity of values might have changed
        // (e.g., tx set fetch)

        if nomination_state.processed_newer_statement(&node_id, statement, envelope_controller) {
            debug!(
                "Node {:?} processed nomination envelope {:?} skipped",
                self.node_idx(),
                pretty_print_scp_env_id(&envelope),
            );
            return EnvelopeState::Invalid;
        }

        // if !nomination_state.is_sane(statement) {
        // todo!();
        // return EnvelopeState::Invalid;
        // }

        nomination_state.record_envelope(envelope, envelope_controller);

        // Whether we have modified nomination state.
        let modified = self.state_may_have_changed(
            statement,
            nomination_state,
            &envelope_controller,
            &quorum_manager,
            herder_driver,
        );

        debug!(
            "Node {:?} processing nomination envelope {:?} triggers stage change: {:?}, current candidates: {:?}",
            self.node_idx(),
            pretty_print_scp_env_id(&envelope),
            modified,
            nomination_state.candidates,
        );

        let new_candidates = statement.accepted.iter().any(|value| {
            if nomination_state.candidates.contains(value) {
                return false;
            }

            if self.federated_ratify(
                |st| accept_predicate(value, st),
                &nomination_state.latest_nominations,
                &envelope_controller.envelopes,
                quorum_manager,
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

        debug!(
            "node {:?} process_nomination_envelope new_candidates: {:?}",
            self.node_idx(),
            new_candidates
        );

        if modified {
            // Somehow is not modified..
            debug!(
                "Node {} emit nomination because of state change",
                self.node_idx()
            );
            self.emit_nomination(
                nomination_state,
                ballot_state,
                envelope_controller,
                quorum_manager,
                herder_driver,
            );
        }

        if new_candidates {
            // TODO: Is this correct?

            if let Some(value) = herder_driver.combine_candidates(&nomination_state.candidates) {
                debug!("new latest composite candidate: {:?}", value);
                *nomination_state.latest_composite_candidate.lock().unwrap() = Some(value);
            }

            let _ = match nomination_state
                .latest_composite_candidate
                .clone()
                .lock()
                .unwrap()
                .as_ref()
            {
                Some(val) => {
                    info!(
                        "Node {:?} bumps state with nomination value {:?}",
                        self.node_idx(),
                        val
                    );

                    debug!(
                        "Node {:?} envs_to_emit before bumping state {:?}",
                        self.node_idx(),
                        envelope_controller.envs_to_emit
                    );

                    self.bump_state(
                        ballot_state,
                        nomination_state,
                        val,
                        true,
                        &mut envelope_controller.envelopes,
                        &mut envelope_controller.envs_to_emit,
                        quorum_manager,
                        herder_driver
                    );

                    debug!("Node {:?} bumped state", self.node_idx());
                    debug!(
                        "Node {:?} envs_to_emit after bumping state {:?}",
                        self.node_idx(),
                        envelope_controller.envs_to_emit
                    );
                }
                None => {
                    todo!();
                }
            };
        } else {
            //
            println!("Nomination candidates: {:?}", nomination_state.candidates);
        }

        EnvelopeState::Valid
    }
}

#[cfg(test)]
mod tests {}
