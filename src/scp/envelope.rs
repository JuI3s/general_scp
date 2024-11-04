use core::time;
use std::{collections::BTreeMap, time::SystemTime};

use env_logger::fmt::Timestamp;
use serde_derive::{Deserialize, Serialize};

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

pub type SCPEnvelopeID = SystemTime;
pub struct SCPEnvelopeController<N>
where
    N: NominationValue,
{
    envelopes: BTreeMap<SCPEnvelopeID, SCPEnvelope<N>>,
    // envelopes:
}

impl<N> SCPEnvelopeController<N>
where
    N: NominationValue,
{
    pub fn new() -> Self {
        Self {
            envelopes: Default::default(),
        }
    }

    pub fn add_envelope(&mut self, envelope: SCPEnvelope<N>) -> SCPEnvelopeID {
        let timestamp = SystemTime::now();
        self.envelopes.insert(timestamp, envelope.clone());
        timestamp
    }

    pub fn get_envelope(&self, env_id: &SCPEnvelopeID) -> Option<&SCPEnvelope<N>> {
        self.envelopes.get(env_id)
    }
}
