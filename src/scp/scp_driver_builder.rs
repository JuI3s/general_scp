use std::process::id;
use std::sync::{Arc, Mutex};

use typenum::U6;

use crate::application::clock::{HVirtualClock, VirtualClock};
use crate::application::work_queue::WorkQueue;
use crate::herder::herder::HerderDriver;

use super::local_node::{HLocalNode, LocalNode};
use super::nomination_protocol::NominationValue;
use super::scp_driver::{HSlotTimer, SlotDriver, SlotTimer};

pub struct SlotDriverBuilder<N, T>
where
    N: NominationValue + 'static,
    T: HerderDriver<N>,
{
    slot_index: Option<u64>,
    local_node: Option<LocalNode<N>>,
    timer: Option<HSlotTimer>,
    herder_driver: Option<T>,
}

impl<N, T> Default for SlotDriverBuilder<N, T>
where
    N: NominationValue + 'static,
    T: HerderDriver<N>,
{
    fn default() -> Self {
        Self {
            slot_index: Default::default(),
            local_node: Default::default(),
            timer: Default::default(),
            herder_driver: Default::default(),
        }
    }
}

impl<N, T> SlotDriverBuilder<N, T>
where
    N: NominationValue + 'static,
    T: HerderDriver<N> + 'static,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn slot_index(mut self, idx: u64) -> Self {
        self.slot_index = Some(idx);
        self
    }

    pub fn local_node(mut self, local_node: LocalNode<N>) -> Self {
        self.local_node = Some(local_node);
        self
    }

    pub fn timer(mut self, timer: HSlotTimer) -> Self {
        self.timer = Some(timer);
        self
    }

    pub fn herder_driver(mut self, herder_driver: T) -> Self {
        self.herder_driver = Some(herder_driver);
        self
    }

    pub fn build(self) -> Result<Arc<SlotDriver<N>>, &'static str> {
        if self.slot_index.is_none() {
            return Err("Missing slot index.");
        }

        if self.local_node.is_none() {
            return Err("Missing local node.");
        }

        if self.timer.is_none() {
            return Err("Missing timer.");
        }

        if self.herder_driver.is_none() {
            return Err("Missing Herder driver.");
        }

        Ok(Arc::new(SlotDriver::<N>::new(
            self.slot_index.unwrap(),
            Arc::new(Mutex::new(self.local_node.unwrap())),
            self.timer.unwrap(),
            Default::default(),
            Default::default(),
            Box::new(self.herder_driver.unwrap()),
        )))
    }
}

pub struct SlotTimerBuilder {
    clock: Option<HVirtualClock>,
}

impl Default for SlotTimerBuilder {
    fn default() -> Self {
        Self {
            clock: Default::default(),
        }
    }
}

impl SlotTimerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn clock(mut self, clock: HVirtualClock) -> Self {
        self.clock = Some(clock);
        self
    }

    pub fn build(self) -> Result<Arc<Mutex<SlotTimer>>, &'static str> {
        if self.clock.is_none() {
            Err("Missing clock.")
        } else {
            let work_queue_handle = Arc::new(Mutex::new(WorkQueue::new(self.clock.unwrap())));

            Ok(Arc::new(Mutex::new(SlotTimer::new(
                work_queue_handle.clone(),
            ))))
        }
    }
}
