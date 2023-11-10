use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

use weak_self_derive::WeakSelf;

use crate::{
    application::work_queue::{ClockEvent, HWorkQueue},
    herder::herder::Herder,
    utils::weak_self::WeakSelf,
};

use super::{
    local_node::LocalNode,
    nomination_protocol::NominationValue,
    slot::{HSlot, Slot, SlotIndex},
};

pub type HSCPDriver = Arc<Mutex<dyn SCPDriver>>;

#[derive(WeakSelf)]
pub struct SlotDriver {
    pub slot_index: u64,
    pub local_node: LocalNode,
    pub timer: SlotTimer,
}

pub trait SCPDriver {
    fn nominating_value(&mut self, value: &NominationValue);
    // fn get_slot(&mut self, )
}

pub struct SlotTimer {
    work_queue: HWorkQueue,
}

impl SlotTimer {
    pub fn add_task(&mut self, callback: ClockEvent) {
        self.work_queue.lock().unwrap().add_task(callback);
    }
}

// pub trait WeakSelf {
//     fn get_weak_self(&mut self) -> Weak<Mutex<&mut Self>>;
// }

impl SCPDriver for SlotDriver {
    fn nominating_value(&mut self, value: &NominationValue) {}
}
