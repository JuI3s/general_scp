use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    marker::PhantomData,
    sync::Arc,
    time::SystemTime,
};

use crate::{
    application::quorum_manager::{self, QuorumManager},
    herder::herder::HerderDriver,
};

use super::{
    ballot_protocol::BallotProtocolState,
    envelope::SCPEnvelopeController,
    nomination_protocol::{
        HSCPNominationValue, NominationProtocol, NominationProtocolState, NominationValue,
    },
    scp_driver::SlotDriver,
    slot::SlotIndex,
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
        nomination_states: &mut BTreeMap<SlotIndex, NominationProtocolState<N>>,
        ballot_states: &mut BTreeMap<SlotIndex, BallotProtocolState<N>>,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) {
        if let Some(job) = self.jobs.pop_front() {
            if let Some(slot_driver) = slots.get(&job.id) {
                let nomination_state = nomination_states
                    .get_mut(&job.id)
                    .expect("Nomination state not found for slot");
                let ballot_state = ballot_states
                    .get_mut(&job.id)
                    .expect("Ballot state not found for slot");
                match job.task {
                    SlotTask::RetryNominate(arg) => arg.execute(
                        slot_driver,
                        nomination_state,
                        ballot_state,
                        envelope_controller,
                        quorum_manager,
                        herder_driver,
                    ),
                    SlotTask::AbandonBallot(arg) => arg.execute(
                        slot_driver,
                        nomination_state,
                        ballot_state,
                        envelope_controller,
                        quorum_manager,
                        herder_driver,
                    ),
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
    pub slot_idx: SlotIndex,
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
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &mut QuorumManager,
        herder_driver: &mut H,
    ) {
        let value = self.value;
        let prev_value = self.previous_value;

        SlotDriver::nominate(
            slot_driver,
            nomination_state,
            ballot_state,
            value,
            &prev_value,
            envelope_controller,
            quorum_manager,
            herder_driver,
        );
    }
}

pub struct AbandonBallotArg<N>
where
    N: NominationValue,
{
    pub slot: SlotIndex,
    pub n: u32,
    phantom: PhantomData<N>,
}

impl<N> AbandonBallotArg<N>
where
    N: NominationValue,
{
    pub fn new(slot: SlotIndex, n: u32) -> Self {
        Self {
            slot,
            n,
            phantom: PhantomData,
        }
    }

    pub fn execute<H: HerderDriver<N> + 'static>(
        self,
        slot_driver: &Arc<SlotDriver<N, H>>,
        nomination_state: &mut NominationProtocolState<N>,
        ballot_state: &mut BallotProtocolState<N>,
        envelope_controller: &mut SCPEnvelopeController<N>,
        quorum_manager: &QuorumManager,
        herder: &mut H,
    ) {
        slot_driver.abandon_ballot(
            ballot_state,
            nomination_state,
            self.n,
            &mut envelope_controller.envelopes,
            &mut envelope_controller.envs_to_emit,
            quorum_manager,
            herder,
        );
    }
}
