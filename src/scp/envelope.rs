use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::SystemTime,
};

use serde_derive::{Deserialize, Serialize};

use crate::application::quorum::QuorumSet;

use super::{
    nomination_protocol::NominationValue, scp::NodeID, scp_driver::HashValue, slot::SlotIndex,
    statement::SCPStatement,
};

pub trait MakeEnvelope<N>
where
    N: NominationValue,
{
    fn new_nomination_envelope(&self, slot_index: usize, vote: N) -> SCPEnvelope<N>;
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SCPEnvelope<N>
where
    N: NominationValue,
{
    pub statement: SCPStatement<N>,
    pub node_id: NodeID,
    pub slot_index: SlotIndex,

    #[serde(with = "serde_bytes")]
    pub signature: HashValue,
}

impl<N: NominationValue> SCPEnvelope<N> {
    pub fn get_quorum_set(&self) -> Option<&QuorumSet> {
        // TODO: should really refactor quorum set out into the scpenvelope struct.
        match &self.statement {
            SCPStatement::Prepare(scpstatement_prepare) => scpstatement_prepare.quorum_set.as_ref(),
            SCPStatement::Confirm(scpstatement_confirm) => scpstatement_confirm.quorum_set.as_ref(),
            SCPStatement::Externalize(scpstatement_externalize) => {
                scpstatement_externalize.commit_quorum_set.as_ref()
            }
            SCPStatement::Nominate(scpstatement_nominate) => {
                scpstatement_nominate.quorum_set.as_ref()
            }
        }
    }
}

pub struct EnvMap<N: NominationValue>(pub BTreeMap<SCPEnvelopeID, SCPEnvelope<N>>);
impl<N: NominationValue> Default for EnvMap<N> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<N: NominationValue> EnvMap<N> {
    pub fn display_for_env_ids<'a, I>(&self, env_ids: I)
    where
        I: IntoIterator<Item = &'a SCPEnvelopeID>,
    {
        for env_id in env_ids {
            let env = self.0.get(env_id).unwrap();
            println!("env_id: {:?}, env: {:?}", env_id, env);
        }
    }
}

impl<N: NominationValue> EnvMap<N> {
    pub fn add_envelope(&mut self, envelope: SCPEnvelope<N>) -> SCPEnvelopeID {
        let timestamp = SystemTime::now();
        self.0.insert(timestamp, envelope.clone());
        timestamp
    }
}

pub type SCPEnvelopeID = SystemTime;

pub struct SCPEnvelopeController<N>
where
    N: NominationValue,
{
    pub envs_to_emit: VecDeque<SCPEnvelopeID>,
    pub envelopes: EnvMap<N>,
    // envelopes:
}

impl<N> SCPEnvelopeController<N>
where
    N: NominationValue,
{
    pub fn new() -> Self {
        Self {
            envs_to_emit: Default::default(),
            envelopes: Default::default(),
        }
    }

    pub fn pop_next_env_to_emit(&mut self) -> Option<SCPEnvelopeID> {
        self.envs_to_emit.pop_front()
    }

    pub fn add_env_to_emit(&mut self, env_id: &SCPEnvelopeID) {
        self.envs_to_emit.push_back(env_id.clone());
    }

    pub fn add_envelope(&mut self, envelope: SCPEnvelope<N>) -> SCPEnvelopeID {
        let timestamp = SystemTime::now();
        self.envelopes.0.insert(timestamp, envelope.clone());
        timestamp
    }

    pub fn get_envelope(&self, env_id: &SCPEnvelopeID) -> Option<&SCPEnvelope<N>> {
        self.envelopes.0.get(env_id)
    }
}
