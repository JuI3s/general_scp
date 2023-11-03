use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use log::debug;

use crate::{overlay::peer::PeerID, herder::herder::Herder};

use super::{nomination_protocol::{NominationProtocolState}, ballot_protocol::{BallotProtocolState}, scp::{SCPEnvelope}, scp_driver::SlotDriver};

pub type SlotIndex = u64;

pub struct Slot
{
    pub index: u64,
    pub nomination_state: NominationProtocolState,
    pub ballot_state: BallotProtocolState,
}
pub type HSlot = Arc<Mutex<Slot>>;

pub type HSCPEnvelope = Arc<Mutex<SCPEnvelope>>;

impl Slot

 {
    pub fn new(index: u64) -> Self {
        Slot {
            index: index,
            nomination_state: NominationProtocolState::default(),
            ballot_state: BallotProtocolState::default(),
        }
    }
}

impl Slot {
    pub fn compute_timeout(round_number: u64) -> Duration {
        if round_number > Slot::MAX_TIMEOUT_SECONDS {
            Duration::from_secs(Slot::MAX_TIMEOUT_SECONDS)
        } else {
            Duration::from_secs(round_number)
        }
    }

    const MAX_TIMEOUT_SECONDS: u64 = (30 * 60);
}

