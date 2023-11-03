use std::{collections::BTreeMap, sync::{Mutex, Arc}};

use super::{slot::{HSCPEnvelope, SlotIndex, HSlot, Slot}, nomination_protocol::{HNominationValue, NominationValue, NominationProtocol}, ballot_protocol::BallotProtocol, scp_driver::{SCPDriver, SlotDriver}};

pub type NodeID = usize;

pub struct SCPEnvelope {}

impl SCPEnvelope {
    pub fn name(&'_ self) {}
}

impl Default for SCPEnvelope {
    fn default() -> Self {
        Self {}
    }
}

pub enum EnvelopeState {
    Valid,
    Invalid,
}

pub trait SCP {
    fn recv_envelope(&mut self, envelope: HSCPEnvelope) -> EnvelopeState;
    fn set_state_from_envelope(&mut self, slot_index: SlotIndex, envelope: HSCPEnvelope);

    fn nominate(&mut self, slot_index: SlotIndex, value: HNominationValue, prev_value: &NominationValue) -> bool; 
    fn stop_nomination(&mut self) -> bool;

    fn purge_slots(&mut self, max_slot_index: u64, slot_to_keep: u64); 
    fn is_slot_fully_validated(&self, slot_index: u64) -> bool;

    fn is_validator(&self) -> bool;
    // returns if we received messages from a v-blocking set
    fn got_v_blocking(&self, slot_index: u64) -> bool;
}


pub struct SCPimpl<Driver>
where Driver: NominationProtocol + BallotProtocol + SCPDriver
 {
    driver: Driver,
    known_slots: BTreeMap<SlotIndex, HSlot>,
}

impl SCPimpl<SlotDriver> {
    pub fn get_slot(&mut self, index: SlotIndex, create_if_not_exists: bool) -> Option<HSlot> {
        match self.known_slots.get(&index) {
            Some(_) => todo!(),
            None => {
                if create_if_not_exists {
                    let new = self.known_slots.insert(index, Arc::new(Mutex::new(Slot::new(index))));
                    new
                } else {
                    None
                }
            },
        }
    }
}

impl SCP for SCPimpl<SlotDriver>
{
    
    fn recv_envelope(&mut self, envelope: HSCPEnvelope) -> EnvelopeState {
        todo!()
    }

    fn set_state_from_envelope(&mut self, slot_index: u64, envelope: HSCPEnvelope) {
        todo!()
    }

    fn nominate(&mut self, slot_index: SlotIndex, value: HNominationValue, prev_value: &NominationValue) -> bool {
        match self.get_slot(slot_index, true) {
            Some(state) => {
                self.driver.nominate(&mut state.lock().unwrap().nomination_state, value, prev_value)
            }
            None => {
                panic!()
            },
        }
    }

    fn stop_nomination(&mut self) -> bool {
        todo!()
    }

    fn purge_slots(&mut self, max_slot_index: u64, slot_to_keep: u64) {
        todo!()
    }

    fn is_slot_fully_validated(&self, slot_index: u64) -> bool {
        todo!()
    }

    fn is_validator(&self) -> bool {
        todo!()
    }

    fn got_v_blocking(&self, slot_index: u64) -> bool {
        todo!()
    }
}