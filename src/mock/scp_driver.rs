use std::{cell::RefCell, collections::HashMap, env, rc::Rc, sync::Arc};

use serde_derive::{Deserialize, Serialize};

use crate::scp::{
    envelope::{SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
    nomination_protocol::NominationValue,
    scp_driver::SlotDriver,
    slot::SlotIndex,
};

use super::state::{MockState, MockStateDriver};
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

// impl MockSCPDriver {
//     pub fn recv_scp_message(
//         &mut self,
//         envelope: &SCPEnvelopeID,
//         envelope_controller: &mut SCPEnvelopeController<MockState>,
//     ) {
//         let env = envelope_controller.get_envelope(envelope).unwrap();
//         let slot = env.slot_index;
//         if let Some(slot_driver) = self.slots.get(&slot) {
//             slot_driver.recv_scp_envelvope(envelope, envelope_controller)
//         }
//     }
// }
