use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::overlay::peer::PeerID;

use super::{
    ballot_protocol::BallotProtocol,
    nomination_protocol::{
        HSCPNominationValue, NominationProtocol, NominationValue, SCPNominationValue,
    },
    scp_driver::{HSCPEnvelope, SCPDriver, SlotDriver},
    slot::{HSlot, Slot, SlotIndex},
    statement::SCPStatement,
};

pub type NodeID = String;

// pub struct SCPEnvelope {
//     pub node_id: NodeID,
//     pub slot_index: SlotIndex,
//     pub statement: SCPStatement,
// }

// impl SCPEnvelope {
//     pub fn name(&'_ self) {}
// }

#[derive(PartialEq, Eq, Debug)]
pub enum EnvelopeState {
    Valid,
    Invalid,
}

pub trait SCP {
    type N: NominationValue;

    fn recv_envelope(&mut self, envelope: HSCPEnvelope<Self::N>) -> EnvelopeState;
    fn set_state_from_envelope(&mut self, slot_index: SlotIndex, envelope: HSCPEnvelope<Self::N>);

    fn nominate(
        &mut self,
        slot_index: SlotIndex,
        value: HSCPNominationValue<Self::N>,
        prev_value: &SCPNominationValue,
    ) -> bool;
    fn stop_nomination(&mut self) -> bool;

    fn purge_slots(&mut self, max_slot_index: u64, slot_to_keep: u64);
    fn is_slot_fully_validated(&self, slot_index: u64) -> bool;

    fn is_validator(&self) -> bool;
    // returns if we received messages from a v-blocking set
    fn got_v_blocking(&self, slot_index: u64) -> bool;
}