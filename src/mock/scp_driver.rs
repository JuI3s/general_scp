use std::collections::HashMap;

use crate::scp::{
    nomination_protocol::NominationValue,
    scp_driver::{SCPEnvelope, SlotDriver},
    slot::SlotIndex,
};

use super::{herder::MockHerder, state::MockState};

pub struct MockSCPDriver {
    pub slots: HashMap<SlotIndex, SlotDriver<MockState, MockHerder>>,
}

impl Default for MockSCPDriver {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

impl MockSCPDriver {
    pub fn new() -> Self {
        Default::default()
    }

    // pub fn recv_scp_envelope(&mut self, envelope: &SCPEnvelope<MockState>) {
    //     let slot =
    // self.slots.entry(envelope.slot_index).or_insert(Default::default());
    //     slot.recv_scp_envelvope(envelope);
    // }
}
