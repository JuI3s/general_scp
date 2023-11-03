use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex}, borrow::BorrowMut,
};

use log::debug;
use tokio::time::timeout;

use crate::overlay::peer::PeerID;

use super::{
    scp::SCPEnvelope,
    slot::{HSCPEnvelope, Slot, self}, scp_driver::{SCPDriver, SlotDriver},
};

pub trait NominationProtocol {
    fn nominate(&mut self, state: &mut NominationProtocolState, value: HNominationValue, previous_value: &NominationValue) -> bool;
    fn recv_nomination_msg(&mut self);
    fn stop_nomination(&mut self);

    fn update_round_learders(&mut self);

    fn set_state_from_envelope(&mut self, envelope: HSCPEnvelope);

    fn get_latest_composite_value(&self) -> HNominationValue;
    fn get_json_info(&self);
}

pub type NominationValueSet = BTreeSet<NominationValue>;
pub type HNominationValue = Arc<Mutex<NominationValue>>;

type HNominationEnvelope = Arc<Mutex<NominationEnvelope>>;
struct NominationEnvelope {}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct NominationValue {}

impl Default for NominationValue {
    fn default() -> Self {
        Self {}
    }
}
// TODO: double check these fields are correct
pub struct NominationProtocolState {
    pub round_number: u64,
    pub votes: NominationValueSet,
    pub accepted: NominationValueSet,
    pub candidates: NominationValueSet,
    pub latest_nominations: BTreeMap<PeerID, SCPEnvelope>,

    pub latest_envelope: HSCPEnvelope,
    pub round_leaders: BTreeSet<PeerID>,

    pub nomination_started: bool,
    pub latest_composite_candidate: HNominationValue,
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
            latest_envelope: Arc::new(Mutex::new(Default::default())),
            round_leaders: Default::default(),
            nomination_started: Default::default(),
            latest_composite_candidate: Default::default(),
            previous_value: Default::default(),
            num_timeouts: Default::default(),
            timed_out: Default::default(),
        }
    }
}

impl NominationProtocolState {
    fn get_new_value_form_nomination(&self, nomination: &SCPEnvelope) -> Option<NominationValue> {
        todo!()
    }
}

impl NominationProtocol for SlotDriver {
    fn nominate(&mut self, state: &mut NominationProtocolState, value: HNominationValue, previous_value: &NominationValue) -> bool {
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

        let timeout = Slot::compute_timeout(state.round_number);

        // for (auto const& leader : mRoundLeaders)
        
        for leader in &state.round_leaders {
            match state.latest_nominations.get(leader).to_owned() {
                Some(nomination) => {
                    match state.get_new_value_form_nomination(nomination) {
                        Some(new_value) => {
                            state.votes.insert(new_value.clone());
                            updated = true;
                            self.nominating_value(&new_value);
                        }
                        None => {},
                    }
                }
                _ => (),
            }
        }
        true
    }

    fn recv_nomination_msg(&mut self) {
        todo!()
    }

    fn stop_nomination(&mut self) {
        todo!()
    }

    fn update_round_learders(&mut self) {
        todo!()
    }

    fn set_state_from_envelope(&mut self, envelope: HSCPEnvelope) {
        todo!()
    }

    fn get_latest_composite_value(&self) -> HNominationValue {
        todo!()
    }

    fn get_json_info(&self) {
        todo!()
    }
}
