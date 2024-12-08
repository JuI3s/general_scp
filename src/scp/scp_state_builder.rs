use super::envelope::SCPEnvelopeID;
use super::nomination_protocol::{
    HLatestCompositeCandidateValue, NominationProtocolState, NominationValue, SCPNominationValueSet,
};


use std::collections::{BTreeMap, BTreeSet};

pub struct NominationProtocolStateBuilder<N>
where
    N: NominationValue,
{
    round_number: Option<u64>,
    votes: Option<SCPNominationValueSet<N>>,
    accepted: Option<SCPNominationValueSet<N>>,
    candidates: Option<SCPNominationValueSet<N>>,
    latest_nominations: Option<BTreeMap<String, SCPEnvelopeID>>,

    latest_envelope: Option<SCPEnvelopeID>,
    round_leaders: Option<BTreeSet<String>>,

    nomination_started: Option<bool>,
    latest_composite_candidate: Option<HLatestCompositeCandidateValue<N>>,
    previous_value: Option<N>,

    num_timeouts: Option<u64>,
    timed_out: Option<bool>,
}

impl<N> Default for NominationProtocolStateBuilder<N>
where
    N: NominationValue,
{
    fn default() -> Self {
        Self {
            round_number: Default::default(),
            votes: Default::default(),
            accepted: Default::default(),
            candidates: Default::default(),
            latest_nominations: Default::default(),
            latest_envelope: Default::default(),
            round_leaders: Default::default(),
            nomination_started: Default::default(),
            latest_composite_candidate: Default::default(),
            previous_value: Default::default(),
            num_timeouts: Default::default(),
            timed_out: Default::default(),
        }
    }
}

impl<N> NominationProtocolStateBuilder<N>
where
    N: NominationValue,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn round_number(mut self, round_number: Option<u64>) -> Self {
        self.round_number = round_number;
        self
    }

    pub fn votes(mut self, votes: SCPNominationValueSet<N>) -> Self {
        self.votes = Some(votes);
        self
    }

    pub fn accepted(mut self, accepted: SCPNominationValueSet<N>) -> Self {
        self.accepted = Some(accepted);
        self
    }

    pub fn candidates(mut self, candidates: SCPNominationValueSet<N>) -> Self {
        self.candidates = Some(candidates);
        self
    }

    pub fn latest_nominations(
        mut self,
        latest_nomination: BTreeMap<String, SCPEnvelopeID>,
    ) -> Self {
        self.latest_nominations = Some(latest_nomination);
        self
    }

    pub fn latest_envelope(mut self, latest_envelope: SCPEnvelopeID) -> Self {
        self.latest_envelope = Some(latest_envelope);
        self
    }

    pub fn round_leaders(mut self, round_leaders: BTreeSet<String>) -> Self {
        self.round_leaders = Some(round_leaders);
        self
    }

    pub fn nomination_started(mut self, nomination_started: bool) -> Self {
        self.nomination_started = Some(nomination_started);
        self
    }

    pub fn latest_composite_candidate(
        mut self,
        latest_composite_candidate: HLatestCompositeCandidateValue<N>,
    ) -> Self {
        self.latest_composite_candidate = Some(latest_composite_candidate);
        self
    }

    pub fn previous_value(mut self, previous_value: N) -> Self {
        self.previous_value = Some(previous_value);
        self
    }

    pub fn num_timeouts(mut self, num_timeouts: u64) -> Self {
        self.num_timeouts = Some(num_timeouts);
        self
    }

    pub fn timed_out(mut self, timed_out: bool) -> Self {
        self.timed_out = Some(timed_out);
        self
    }

    pub fn build(self) -> NominationProtocolState<N> {
        let round_number = self.round_number.unwrap_or_default();

        let votes = self.votes.unwrap_or_default();
        let accepted = self.accepted.unwrap_or(Default::default());
        let candidates = self.candidates.unwrap_or(Default::default());
        let latest_nominations = self.latest_nominations.unwrap_or_default();

        let latest_envelope = self.latest_envelope;
        let round_leaders = self.round_leaders.unwrap_or_default();

        let nomination_started = self.nomination_started.unwrap_or_default();
        let latest_composite_candidate = self.latest_composite_candidate.unwrap_or_default();
        let previous_value = self.previous_value.unwrap_or_default();

        let num_timeouts = self.num_timeouts.unwrap_or_default();
        let timed_out = self.timed_out.unwrap_or_default();

        let state = NominationProtocolState::<N> {
            round_number: round_number,
            votes: votes,
            accepted: accepted,
            candidates: candidates,
            latest_nominations: latest_nominations,
            latest_envelope: latest_envelope,
            round_leaders: round_leaders,
            nomination_started: nomination_started,
            latest_composite_candidate: latest_composite_candidate,
            previous_value: previous_value,
            num_timeouts: num_timeouts,
            timed_out: timed_out,
        };

        state
    }
}
