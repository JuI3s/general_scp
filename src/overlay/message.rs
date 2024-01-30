use bincode::de;
use serde::Serialize;

use crate::{
    crypto::types::Blake2Hashable,
    scp::{envelope::SCPEnvelope, nomination_protocol::NominationValue},
};

#[derive(Clone, Serialize)]
pub enum SCPMessage<N>
where
    N: NominationValue,
{
    SCP(SCPEnvelope<N>),
    Hello(HelloEnvelope),
}

impl<N> Blake2Hashable for SCPMessage<N> where N: NominationValue {}

impl<N> SCPMessage<N>
where
    N: NominationValue,
{
    pub fn is_boardcast_msg(&self) -> bool {
        true
    }
}

#[derive(Serialize, Clone)]
pub struct HelloEnvelope {}
