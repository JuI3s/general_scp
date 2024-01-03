use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::scp::{
    nomination_protocol::NominationValue,
    scp_driver::{SCPEnvelope, SlotDriver},
    slot::SlotIndex,
};

use super::{herder::MockHerder, state::MockState};

pub struct MockSCPDriver {
    pub slots: HashMap<SlotIndex, Rc<RefCell<SlotDriver<MockState, MockHerder>>>>,
}

impl Default for MockSCPDriver {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}
