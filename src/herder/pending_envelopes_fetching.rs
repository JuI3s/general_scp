use std::{
    cell::{Ref, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    future::Pending,
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    application::quorum::{QuorumSet, QuorumSetHash},
    crypto::types::{Blake2Hash, Blake2Hashable, Blake2Hasher},
    scp::{
        envelope::{SCPEnvelope, SCPEnvelopeController},
        nomination_protocol::NominationValue,
        scp::EnvelopeState,
        slot::{self, SlotIndex},
    },
};

use super::{
    herder::{HerderDriver, HerderEnvelopeStatus},
    pending_envelope_manager::PendingEnvelopeManager,
};

pub struct SlotEnvelopes<N>
where
    N: NominationValue,
{
    ready_envelopes: Vec<SCPEnvelope<N>>,
    discarded_envelopes: HashSet<SCPEnvelope<N>>,
    processed_envelopes: HashSet<SCPEnvelope<N>>,
    fetching_envelopes: HashSet<SCPEnvelope<N>>,
}

impl<N> Default for SlotEnvelopes<N>
where
    N: NominationValue,
{
    fn default() -> Self {
        Self {
            ready_envelopes: Default::default(),
            discarded_envelopes: Default::default(),
            processed_envelopes: Default::default(),
            fetching_envelopes: Default::default(),
        }
    }
}

impl<N> SlotEnvelopes<N>
where
    N: NominationValue,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn pop(&mut self) -> Option<SCPEnvelope<N>> {
        self.ready_envelopes.pop()
    }

    pub fn is_discarded(&self, envelope: &SCPEnvelope<N>) -> bool {
        self.discarded_envelopes.contains(envelope)
    }

    pub fn is_processed(&self, envelope: &SCPEnvelope<N>) -> bool {
        self.processed_envelopes.contains(envelope)
    }

    pub fn is_fetching(&self, envelopes: &SCPEnvelope<N>) -> bool {
        self.fetching_envelopes.contains(envelopes)
    }

    pub fn envelope_ready(&mut self, envelope: &SCPEnvelope<N>) -> bool {
        let ret = self.fetching_envelopes.remove(envelope);
        self.processed_envelopes.insert(envelope.to_owned());
        ret
    }
}

pub struct PendingEnvelopesFetchingManager<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    nomination_value_fetcher: ItemFetcher<N>,
    scp_quorum_set_fetcher: ItemFetcher<N>,
    slot_envelopes: BTreeMap<SlotIndex, SlotEnvelopes<N>>,

    known_quorum_set_hashes: HashMap<QuorumSetHash, Rc<RefCell<QuorumSet>>>,
    known_value_hashes: HashMap<Blake2Hash, Rc<RefCell<N>>>,

    herder: Rc<RefCell<H>>,
}

// The ItemFetcher manages trackers for a type of item.
pub struct ItemFetcher<N>
where
    N: NominationValue,
{
    trackers: BTreeMap<Blake2Hash, Tracker<N>>,
}

// The tracker manages fetchings state for a item of a specific hash value.
pub struct Tracker<N>
where
    N: NominationValue,
{
    last_seen_index: Option<usize>,
    waiting_envelopes: Vec<SCPEnvelope<N>>,
}

impl<N, H> PendingEnvelopesFetchingManager<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub fn new(herder: Rc<RefCell<H>>) -> Self {
        Self {
            nomination_value_fetcher: Default::default(),
            scp_quorum_set_fetcher: Default::default(),
            slot_envelopes: Default::default(),
            herder: herder,
            known_quorum_set_hashes: Default::default(),
            known_value_hashes: Default::default(),
        }
    }

    fn is_discarded(&self, envelope: &SCPEnvelope<N>) -> bool {
        if let Some(slot_envelopes) = self.slot_envelopes.get(&envelope.slot_index) {
            slot_envelopes.is_discarded(envelope)
        } else {
            false
        }
    }

    fn is_processed(&self, envelope: &SCPEnvelope<N>) -> bool {
        if let Some(slot_envelopes) = self.slot_envelopes.get(&envelope.slot_index) {
            slot_envelopes.is_processed(envelope)
        } else {
            false
        }
    }

    fn is_fetching(&self, envelope: &SCPEnvelope<N>) -> bool {
        if let Some(slot_envelopes) = self.slot_envelopes.get(&envelope.slot_index) {
            slot_envelopes.is_fetching(envelope)
        } else {
            false
        }
    }

    fn get_nomination_value(&self, hash: &Blake2Hash) -> Option<Rc<RefCell<N>>> {
        let val = self.known_value_hashes.get(hash)?;
        Some(val.to_owned())
    }

    fn get_quorum_set(&self, hash: &QuorumSetHash) -> Option<Rc<RefCell<QuorumSet>>> {
        let q_set = self.known_quorum_set_hashes.get(hash)?;
        Some(q_set.to_owned())
    }

    fn start_fetching(&mut self, envelope: &SCPEnvelope<N>) {
        // Maybe fetcing quorum set.
        let q_hash = envelope.statement.quorum_set_hash_value();
        if self.get_quorum_set(&q_hash).is_none() {
            self.scp_quorum_set_fetcher.fetch(&q_hash, envelope);
        }

        // Fetching nomination values that we do not currently have.

        todo!()
    }

    fn fully_fetched(&self, envelope: &SCPEnvelope<N>) -> bool {
        todo!()
    }

    fn envelope_ready(&mut self, envelope: &SCPEnvelope<N>) -> Result<(), ()> {
        let _ = match self.slot_envelopes.get_mut(&envelope.slot_index) {
            Some(slot_envelope) => {
                slot_envelope.envelope_ready(envelope);
            }
            None => return Err(()),
        };

        todo!();

        Ok(())
    }
}

impl<N, H> PendingEnvelopeManager<N> for PendingEnvelopesFetchingManager<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    fn envelope_status(&mut self, envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus {
        if self.is_processed(envelope) {
            return HerderEnvelopeStatus::EnvelopeStatusProcessed;
        }

        if self.is_discarded(envelope) {
            return HerderEnvelopeStatus::EnvelopeStatusDiscarded;
        }

        if self.fully_fetched(envelope) {
            self.envelope_ready(envelope).unwrap();
        } else {
            self.start_fetching(envelope);
        }

        HerderEnvelopeStatus::EnvelopeStatusFetching
    }

    fn recv_scp_quorum_set(
        &mut self,
        quorum_set: &QuorumSet,
        envelope_controller: &mut SCPEnvelopeController<N>,
    ) {
        self.scp_quorum_set_fetcher
            .recv(&quorum_set.to_blake2(), &mut |env| {
                H::recv_scp_envelope(&self.herder, env, &envelope_controller);
            });
    }

    fn recv_nomination_value(&mut self, value: &N) {
        self.nomination_value_fetcher
            .recv(&Blake2Hasher::<N>::hash(value), &mut |env| {
                H::recv_scp_envelope(&self.herder, env);
            })
    }

    fn pop(&mut self, slot_index: &SlotIndex) -> Option<SCPEnvelope<N>> {
        let slot_envelopes = self.slot_envelopes.get_mut(slot_index)?;
        slot_envelopes.pop()
    }
}

impl<N> ItemFetcher<N>
where
    N: NominationValue,
{
    pub fn fetch(&mut self, hash: &Blake2Hash, envelope: &SCPEnvelope<N>) {
        let tracker = self.trackers.entry(*hash).or_default();
        tracker.listen(envelope);
    }

    pub fn recv(&mut self, hash: &Blake2Hash, callback: &mut impl FnMut(&SCPEnvelope<N>)) {
        if let Some(tracker) = self.trackers.get_mut(hash) {
            tracker.visit(callback);
        }
    }

    pub fn last_seen_slot_index(&mut self, hash: &Blake2Hash) -> Option<usize> {
        let tracker = self.trackers.get(hash)?;
        tracker.last_seen_index
    }
}

impl<N> Default for ItemFetcher<N>
where
    N: NominationValue,
{
    fn default() -> Self {
        Self {
            trackers: Default::default(),
        }
    }
}

impl<N> Default for Tracker<N>
where
    N: NominationValue,
{
    fn default() -> Self {
        Self {
            waiting_envelopes: Default::default(),
            last_seen_index: Default::default(),
        }
    }
}

impl<N> Tracker<N>
where
    N: NominationValue,
{
    pub fn listen(&mut self, envelope: &SCPEnvelope<N>) {}

    pub fn try_next_peer(&mut self) {}

    pub fn cancel(&mut self) {
        self.last_seen_index = None;
    }

    pub fn visit(&mut self, callback: &mut impl FnMut(&SCPEnvelope<N>)) {
        for envelope in &self.waiting_envelopes {
            callback(&envelope);
        }
    }
}
