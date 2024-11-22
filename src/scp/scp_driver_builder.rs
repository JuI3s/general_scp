use std::cell::RefCell;
use std::process::id;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use typenum::U6;

use crate::application::clock::{HVirtualClock, VirtualClock};
use crate::application::work_queue::{EventQueue, HWorkScheduler, WorkScheduler};
use crate::herder::herder::HerderDriver;

use super::ballot_protocol::BallotProtocolState;
use super::local_node::{HLocalNode, LocalNodeInfo};
use super::nomination_protocol::{NominationProtocolState, NominationValue};
use super::queue::SlotJobQueue;
use super::scp_driver::SlotDriver;
use super::slot::SlotIndex;

pub struct SlotDriverBuilder<N, T>
where
    N: NominationValue + 'static,
    T: HerderDriver<N>,
{
    slot_index: Option<SlotIndex>,
    local_node: Option<Rc<RefCell<LocalNodeInfo<N>>>>,
    timer: Option<Rc<RefCell<WorkScheduler>>>,
    herder_driver: Option<Rc<RefCell<T>>>,
    nomination_protocol_state: Option<NominationProtocolState<N>>,
    ballot_protocol_state: Option<BallotProtocolState<N>>,
    task_queue: Option<Rc<RefCell<SlotJobQueue<N, T>>>>,
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
            task_queue: Default::default(),
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

    pub fn slot_index(mut self, idx: SlotIndex) -> Self {
        self.slot_index = Some(idx);
        self
    }

    pub fn local_node(mut self, local_node: HLocalNode<N>) -> Self {
        self.local_node = Some(local_node.into());
        self
    }

    pub fn timer(mut self, timer: Rc<RefCell<WorkScheduler>>) -> Self {
        self.timer = Some(timer);
        self
    }

    pub fn herder_driver(mut self, herder_driver: Rc<RefCell<T>>) -> Self {
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

    pub fn task_queue(mut self, task_queue: Rc<RefCell<SlotJobQueue<N, T>>>) -> Self {
        self.task_queue = Some(task_queue);
        self
    }

    pub fn build(self) -> Result<SlotDriver<N, T>, &'static str> {
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

        Ok(SlotDriver::<N, T>::new(
            self.slot_index.unwrap(),
            self.local_node.unwrap().into(),
            Arc::new(Mutex::new(
                self.nomination_protocol_state.unwrap_or_default(),
            )),
            Arc::new(Mutex::new(self.ballot_protocol_state.unwrap_or_default())),
            self.herder_driver.unwrap(),
            self.task_queue
                .unwrap_or(Rc::new(RefCell::new(SlotJobQueue::new()))),
            self.timer.unwrap(),
        ))
    }

    pub fn build_handle(self) -> Result<Arc<SlotDriver<N, T>>, &'static str> {
        match self.build() {
            Ok(ret) => Ok(Arc::new(ret)),
            Err(err) => Err(err),
        }
    }
}
