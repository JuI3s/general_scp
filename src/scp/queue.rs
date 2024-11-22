use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    env,
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
    time::SystemTime,
};

use syn::token::Ref;

use crate::{ca::cell::Cell, herder::herder::HerderDriver};

use super::{
    ballot_protocol::HBallotProtocolState,
    envelope::{self, SCPEnvelopeController},
    nomination_protocol::{
        HNominationProtocolState, HSCPNominationValue, NominationProtocol, NominationProtocolState,
        NominationValue,
    },
    scp_driver::SlotDriver,
    slot::{self, SlotIndex},
};

pub struct SlotJobQueue<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    jobs: VecDeque<SlotJob<N>>,
    phantom: PhantomData<H>,
}

impl<N, H> SlotJobQueue<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    pub fn new() -> Self {
        Self {
            jobs: VecDeque::new(),
            phantom: PhantomData,
        }
    }

    pub fn submit(&mut self, job: SlotJob<N>) {
        self.jobs.push_back(job);
    }

    pub fn process_one(
        &mut self,
        slots: &HashMap<SlotIndex, Arc<SlotDriver<N, H>>>,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) {
        if let Some(job) = self.jobs.pop_front() {
            if let Some(slot_driver) = slots.get(&job.id) {
                match job.task {
                    SlotTask::RetryNominate(arg) => arg.execute(slot_driver, envelope_controller),
                    SlotTask::AbandonBallot(arg) => arg.execute(slot_driver, envelope_controller),
                }
            }
        }
    }
}

pub struct SlotJob<N>
where
    N: NominationValue,
{
    pub id: SlotIndex,
    pub timestamp: SystemTime,
    pub task: SlotTask<N>,
}

pub enum SlotTask<N>
where
    N: NominationValue,
{
    RetryNominate(RetryNominateArg<N>),
    AbandonBallot(AbandonBallotArg<N>),
}

pub struct RetryNominateArg<N>
where
    N: NominationValue,
{
    pub state_handle: HNominationProtocolState<N>,
    pub value: HSCPNominationValue<N>,
    pub previous_value: N,
    // envelope_controller: &SCPEnvelopeController<N>,
}

impl<N> RetryNominateArg<N>
where
    N: NominationValue,
{
    pub fn execute<H: HerderDriver<N> + 'static>(
        self,
        slot_driver: &Arc<SlotDriver<N, H>>,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) {
        let state = self.state_handle;
        let value = self.value;
        let prev_value = self.previous_value;

        SlotDriver::nominate(slot_driver, state, value, &prev_value, envelope_controller);
    }
}

pub struct AbandonBallotArg<N>
where
    N: NominationValue,
{
    pub state: HBallotProtocolState<N>,
    pub nomination_state: HNominationProtocolState<N>,
    pub n: u32,
}

impl<N> AbandonBallotArg<N>
where
    N: NominationValue,
{
    pub fn execute<H: HerderDriver<N> + 'static>(
        self,
        slot_driver: &Arc<SlotDriver<N, H>>,
        envelope_controller: &SCPEnvelopeController<N>,
    ) {
        SlotDriver::abandon_ballot(
            slot_driver,
            self.state.lock().unwrap().borrow_mut(),
            self.nomination_state.lock().unwrap().borrow_mut(),
            self.n,
            envelope_controller,
        );
    }
}
