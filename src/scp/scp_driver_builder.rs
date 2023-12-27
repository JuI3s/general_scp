use std::process::id;
use std::sync::{Arc, Mutex};

use typenum::U6;

use crate::application::clock::{HVirtualClock, VirtualClock};
use crate::application::work_queue::{EventQueue, HWorkScheduler, WorkScheduler};
use crate::herder::herder::HerderDriver;

use super::ballot_protocol::BallotProtocolState;
use super::local_node::{HLocalNode, LocalNode};
use super::nomination_protocol::{NominationProtocolState, NominationValue};
use super::scp_driver::{SlotDriver};

pub struct SlotDriverBuilder<N, T>
where
    N: NominationValue + 'static,
    T: HerderDriver<N>,
{
    slot_index: Option<u64>,
    local_node: Option<LocalNode<N>>,
    timer: Option<WorkScheduler>,
    herder_driver: Option<T>,
    nomination_protocol_state: Option<NominationProtocolState<N>>,
    ballot_protocol_state: Option<BallotProtocolState<N>>,
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
            nomination_protocol_state: Default::default(),
            ballot_protocol_state: Default::default(),
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

    pub fn timer(mut self, timer: WorkScheduler) -> Self {
        self.timer = Some(timer);
        self
    }

    pub fn herder_driver(mut self, herder_driver: T) -> Self {
        self.herder_driver = Some(herder_driver);
        self
    }

    pub fn nomination_protocol_state(
        mut self,
        nomination_protocol_state: NominationProtocolState<N>,
    ) -> Self {
        self.nomination_protocol_state = Some(nomination_protocol_state);
        self
    }

    pub fn ballot_protocol_state(mut self, ballot_protocol_state: BallotProtocolState<N>) -> Self {
        self.ballot_protocol_state = Some(ballot_protocol_state);
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
            Arc::new(Mutex::new(
                self.nomination_protocol_state.unwrap_or_default(),
            )),
            Arc::new(Mutex::new(self.ballot_protocol_state.unwrap_or_default())),
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

    pub fn build(self) -> Result<WorkScheduler, &'static str> {
        if self.clock.is_none() {
            Err("Missing clock.")
        } else {
            Ok(WorkScheduler::new(self.clock.unwrap()))

        }
    }
}
