use std::{collections::{BTreeMap, BTreeSet}, sync::{Mutex, Arc}};

use crate::overlay::peer::PeerID;

use super::{scp::SCPEnvelope, slot::HSCPEnvelope};

pub trait NominationProtocol {
    fn nominate(&mut self);
    fn recv_nomination_msg(&mut self);
}

pub type NominationValueSet = BTreeSet<NominationValue>;
pub type HNominationValue = Arc<Mutex<NominationValue>>;

type HNominationEnvelope = Arc<Mutex<NominationEnvelope>>;
struct NominationEnvelope {}
pub struct NominationValue {}

impl Default for NominationValue {
    fn default() -> Self {
        Self {}
    }
}
// TODO: double check these fields are correct
pub struct NominationProtocolState {
    round_number: usize,
    votes: NominationValueSet,
    accepted: NominationValueSet,
    candidates: NominationValueSet,
    latest_nominations: BTreeMap<PeerID, SCPEnvelope>,

    latest_envelope: HSCPEnvelope,
    round_leaders: BTreeSet<PeerID>,

    nomination_started: bool,
    latest_composite_candidate: HNominationValue,
    previous_value: NominationValue,
}

impl NominationProtocolState {}

impl Default for NominationProtocolState {
    fn default() -> Self {
        Self {
            round_number: Default::default(),
            votes: Default::default(),
            accepted: Default::default(),
            candidates: Default::default(),
            latest_nominations: Default::default(),
            latest_envelope: Arc::new(Mutex::new(Default::default())),
            round_leaders: Default::default(),
            nomination_started: Default::default(),
            latest_composite_candidate: Default::default(),
            previous_value: Default::default(),
        }
    }
}
