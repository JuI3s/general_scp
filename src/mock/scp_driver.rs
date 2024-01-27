use std::{cell::RefCell, collections::HashMap, env, rc::Rc, sync::Arc};

use serde_derive::{Deserialize, Serialize};

use crate::scp::{
    nomination_protocol::NominationValue,
    scp_driver::{SCPEnvelope, SlotDriver},
    slot::SlotIndex,
};

use super::{
    herder::MockHerder,
    state::{MockState, MockStateDriver},
};
pub struct MockSCPDriver {
    pub slots: HashMap<SlotIndex, Arc<SlotDriver<MockState, MockStateDriver>>>,
    // pub slots: HashMap<SlotIndex, Rc<RefCell<SlotDriver<MockState, MockStateDriver>>>>,
}

impl Default for MockSCPDriver {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

impl MockSCPDriver {
    pub fn recv_scp_message(mut self, envelope: &SCPEnvelope<MockState>) {
        let slot = envelope.slot_index;
        if let Some(slot_driver) = self.slots.get(&slot) {
            slot_driver.recv_scp_envelvope(envelope)
        }
    }
}
