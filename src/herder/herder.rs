use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::scp::{
    scp_driver::SlotDriver,
    slot::{HSlot, Slot, SlotIndex},
};

pub struct Herder {
    latest_slots: BTreeMap<SlotIndex, HSlot>,
}

impl Default for Herder {
    fn default() -> Self {
        Self {
            latest_slots: Default::default(),
        }
    }
}

// impl<'a> Herder<'a> {

//     pub fn get_slot(&'a mut self, index: SlotIndex, create_if_not_exists: bool) -> Option<HSlot<'a>> {
//         match self.latest_slots.get(&index) {
//             Some(slot) => {
//                 Some(slot.clone())
//             },
//             None => {
//                 if create_if_not_exists {
//                     let slot = Slot::new(index, self);
//                     let ret = self.latest_slots.insert(index, Arc::new(Mutex::new(slot)));
//                     ret
//                 } else {
//                     None
//                 }
//             },
//         }
//     }
// }
