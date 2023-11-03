use std::{sync::{Arc, Mutex}, collections::BTreeMap};

use crate::herder::herder::Herder;

use super::{nomination_protocol::NominationValue, slot::{Slot, SlotIndex, HSlot}};

pub type HSCPDriver = Arc<Mutex<dyn SCPDriver>>;

pub trait SCPDriver {
    fn nominating_value(&mut self, value: &NominationValue);
    // fn get_slot(&mut self, )
}


pub struct SlotDriver {
    pub slot_index: u64,
}

impl SCPDriver for SlotDriver {
    fn nominating_value(&mut self, value: &NominationValue) {
        todo!()
    }
}
