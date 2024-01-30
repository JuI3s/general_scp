use serde_derive::{Deserialize, Serialize};

use super::{
    nomination_protocol::NominationValue, scp::NodeID, scp_driver::HashValue, slot::SlotIndex,
    statement::SCPStatement,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
