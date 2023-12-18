use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use log::debug;

use crate::overlay::peer::PeerID;

use super::{
    ballot_protocol::BallotProtocolState,
    nomination_protocol::{NominationProtocolState, NominationValue},
    scp_driver::SlotDriver,
};

pub type SlotIndex = u64;

pub struct Slot<N>
where
    N: NominationValue,
{
    pub index: u64,
    pub nomination_state: NominationProtocolState<N>,
    pub ballot_state: BallotProtocolState<N>,
}
pub type HSlot<N> = Arc<Mutex<Slot<N>>>;

impl<N> Slot<N>
where
    N: NominationValue,
{
    const MAX_TIMEOUT_SECONDS: u64 = (30 * 60);

    pub fn new(index: u64) -> Self {
        Slot {
            index: index,
            nomination_state: NominationProtocolState::default(),
            ballot_state: BallotProtocolState::default(),
        }
    }

    pub fn compute_timeout(round_number: u64) -> Duration {
        if round_number > Slot::<N>::MAX_TIMEOUT_SECONDS {
            Duration::from_secs(Slot::<N>::MAX_TIMEOUT_SECONDS)
        } else {
            Duration::from_secs(round_number)
        }
    }
}
