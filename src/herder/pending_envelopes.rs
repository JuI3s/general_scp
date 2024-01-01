use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
    future::Pending,
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    application::quorum::QuorumSet,
    crypto::types::{Blake2Hash, Blake2Hashable, Blake2Hasher},
    scp::{nomination_protocol::NominationValue, scp::EnvelopeState, scp_driver::SCPEnvelope},
};

use super::herder::{HerderDriver, HerderEnvelopeStatus};

pub struct SlotEnvelopes<N>
where
    N: NominationValue,
{
    ready_envelopes: HashSet<SCPEnvelope<N>>,
    discarded_envelopes: HashSet<SCPEnvelope<N>>,
    processed_envelopes: HashSet<SCPEnvelope<N>>,
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

    pub fn is_discarded(&self, envelope: &SCPEnvelope<N>) -> bool {
        self.discarded_envelopes.contains(envelope)
    }

    pub fn is_processed(&self, envelope: &SCPEnvelope<N>) -> bool {
        self.processed_envelopes.contains(envelope)
    }
}

pub struct PendingEnvelopes<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    nomination_value_fetcher: ItemFetcher<N>,
    scp_quorum_set_fetcher: ItemFetcher<N>,
    slot_envelopes: BTreeMap<usize, SlotEnvelopes<N>>,
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

impl<N, H> PendingEnvelopes<N, H>
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
        }
    }

    fn is_discarded(&self, envelope: &SCPEnvelope<N>) -> bool {
        todo!()
    }

    pub fn envelope_status(&self, envelope: &SCPEnvelope<N>) -> HerderEnvelopeStatus {
        // if self.slot_envelopes.f

        todo!()
    }

    pub fn recv_scp_quorum_set(&mut self, quorum_set: &QuorumSet) {
        self.scp_quorum_set_fetcher
            .recv(&quorum_set.to_blake2(), &mut |env| {
                self.herder.borrow_mut().recv_scp_envelope(env);
            });
    }

    pub fn recv_nomination_value(&mut self, value: &N) {
        self.nomination_value_fetcher
            .recv(&Blake2Hasher::<N>::hash(value), &mut |env| {
                self.herder.borrow_mut().recv_scp_envelope(env);
            })
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
