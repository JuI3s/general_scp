use std::process::id;
use std::sync::{Arc, Mutex};

use typenum::U6;

use crate::herder::herder::HerderDriver;

use super::local_node::{HLocalNode, LocalNode};
use super::nomination_protocol::NominationValue;
use super::scp_driver::{HSlotTimer, SlotDriver};

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

    pub fn build(self) -> Result<SlotDriver<N>, &'static str> {
        if self.slot_index.is_none() {
            return Err("Missing slot index.");
        }

        Ok(SlotDriver::<N>::new(
            self.slot_index.unwrap(),
            Arc::new(Mutex::new(self.local_node.unwrap())),
            self.timer.unwrap(),
            Default::default(),
            Default::default(),
            Box::new(self.herder_driver.unwrap()),
        ))
    }
}
