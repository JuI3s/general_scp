use weak_self_derive::WeakSelf;
use std::sync::Weak;

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
    application::work_queue::ClockEvent, overlay::peer::PeerID, utils::weak_self::WeakSelf,
};

use super::{
    scp::SCPEnvelope,
    scp_driver::{SCPDriver, SlotDriver},
    slot::{self, HSCPEnvelope, Slot},
};

pub trait NominationProtocol {
    fn nominate(
        self: &Arc<Self>,
        state: HNominationProtocolState,
        value: HNominationValue,
        previous_value: &NominationValue,
    ) -> bool;
    fn stop_nomination(&mut self, state: &mut NominationProtocolState);

    fn update_round_learders(&mut self);

    fn set_state_from_envelope(&mut self, envelope: HSCPEnvelope);

    fn get_latest_composite_value(&self) -> HNominationValue;
    fn get_json_info(&self);
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
pub type NominationValueSet = BTreeSet<HNominationValue>;

pub type HNominationProtocolState = Arc<Mutex<NominationProtocolState>>;
// TODO: double check these fields are correct
// #[derive(WeakSelf)]
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
    fn get_new_value_form_nomination(
        &self,
        nomination: &SCPEnvelope,
    ) -> Option<NominationValue> {
        todo!()
    }

    pub fn add_value_from_leaders(&mut self, driver: &Arc<impl SCPDriver>) -> bool {
        let mut updated = false;
        for leader in  &self.round_leaders {
            match self.latest_nominations.get(leader).to_owned() {
                Some(nomination) => {
                    match self.get_new_value_form_nomination(
                        nomination,
                    ) {
                        Some(new_value) => {
                            driver.nominating_value(&new_value);
                            let new_value_handle = Arc::new(new_value);
                            self.votes.insert(new_value_handle);
                            updated = true;
                        }
                        None => {}
                    }
                }
                _ => (),
            }
        }
        updated
    }

}

impl NominationProtocol for SlotDriver {
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

        let timeout = Slot::compute_timeout(state.round_number);

        state.add_value_from_leaders(self);

        // if we're leader, add our value if we haven't added any votes yet
        if state.round_leaders.contains(&self.local_node.lock().unwrap().node_id) && state.votes.is_empty() {
            if state.votes.insert(value.clone()) {
                updated = true;
                self.nominating_value(value.as_ref());
            }
        }

        let weak_self = Arc::downgrade(self);
        let weak_state = Arc::downgrade(&state_handle.clone());
        let value_copy =value.clone(); 
        let prev_value_copy = previous_value.clone();

        let  callback  =  move || {
            // strong_self.lock().unwrap().nominate(&mut strong_state.lock().unwrap(), value, previous_value);

            match weak_self.upgrade() {
                Some(slot_driver) => {
                    match weak_state.upgrade() {
                        Some(state) => {
                            slot_driver.nominate(state, value_copy, &prev_value_copy);
                        },
                        None => todo!(),
                    }
                },
                None => todo!(),
            }
        };

        let clock_event = ClockEvent::new(SystemTime::now() + timeout, Box::new(callback) );
        self.timer.lock().unwrap().add_task(clock_event);
            
        if updated {
            todo!();
            // Emit nomination
        } else {
            debug!("NominationProtocol::nominate (SKIPPED");
        }

        updated
    }


    fn stop_nomination(&mut self, state: &mut NominationProtocolState) {
        state.nomination_started = false;
    }

    fn update_round_learders(&mut self) {
        let local_id = self.local_node.lock().unwrap().node_id;
        
        let max_leader_count = &self.local_node.lock().unwrap().quorum_set;

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
