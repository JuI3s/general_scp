use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    sync::{Arc, Mutex},
};

use syn::token::Percent;

use crate::{
    application::work_queue::{HWorkScheduler, WorkScheduler},
    scp::{nomination_protocol::NominationValue, scp::NodeID},
};

use super::peer::{HPeer, Peer, PeerID, SCPPeer};

#[derive(Clone, Debug)]
pub struct SCPMessage {}

pub type HSCPMessage = Arc<Mutex<SCPMessage>>;

impl SCPMessage {
    fn to_hash(&self) -> u64 {
        todo!()
    }
}

// The consensus protocol works on top of an underlying overlay network, and
// envelopes emitted by the consensus algorithm are broadcast to remote peers.
// The diagram below gives an overview of how the different components interact
// in broadcasting messages.
//
// ┌──────────────────────┐
// │                      │
// │  Consensus protocol  │
// │                      │
// └──────────────────────┘
//             │
//             │
// triggered by consensus events, such as
// externalization or nomination of a value
//             │
//             │
//             ▼
// ┌──────────────────────┐
// │        Herder        │
// └──────────────────────┘
//             │                             ┌───────────────────────────────┐
//             │                             │                               │
// pass to overlay manager to send           │The overlay manager keeps track│
//  message to remote peers                  │   of information about the    │
//             ▼                             │underlying overlay network that│
//  ┌────────────────────┐                   │  the local node needs, e.g.   │
//  │                    │                   │connected/authenticated peers, │
//  │  Overlay Manager   │─ ─ ─ ─ ─ ─ ─ ─ ─ ▶│ which peers have received the │
//  │                    │                   │ msg, etc. The overlay manager │
//  └────────────────────┘                   │  owns the floodgate which is  │
//             │                             │ responsible for broadcasting  │
//             │                             │    messages in the overlay    │
//             │                             │           network.            │
//           pass                            │                               │
//        message to                         └───────────────────────────────┘
//        broadcast
//             │                              ┌──────────────────────────────┐
//             ▼                              │ The floodgate keeps track of │
// ┌──────────────────────┐                   │ state and information about  │
// │                      │                   │ broadcasting messages in the │
// │      Flood gate      │                   │overlay network. For example, │
// │                      │ ─ ─ ─ ─ ─ ─ ─ ─ ─▶│  it stores for each message  │
// │                      │                   │ whether each peer for which  │
// └──────────────────────┘                   │the message has been sent has │
//                                            │          responded.          │
//                                            │                              │
//                                            └──────────────────────────────┘
pub trait OverlayManager<N>
where
    N: NominationValue,
{
    // Peer handle.
    type HP;
    type P: SCPPeer<N>;

    // TODO:
    // Send a given message to all peers, via the FloodGate.
    // returns true if message was sent to at least one peer
    fn broadcast_message(&mut self, msg: &SCPMessage, force: bool, hash: Option<u64>) -> bool;

    // Make a note in the FloodGate that a given peer has provided us with an
    // given broadcast message, so that it is inhibited from being resent nto
    // that peer. This does _not_ cause the message to be broadcast anew; to do
    // that, call broadcastMessage, above.
    // Returns true if this is a new message
    // fills msgID with msg's hash
    fn recv_flooded_message(&mut self, msg: &SCPMessage, peer: &Self::P, msg_id: u64);

    // removes msgID from the floodgate's internal state
    // as it's not tracked anymore, calling "broadcast" with a (now forgotten)
    // message with the ID msgID will cause it to be broadcast to all peers
    fn forget_flooded_message(&mut self, msg_id: &u64);

    fn remove_peer(&mut self, peer: &Self::P);

    fn get_authenticated_peers(&self) -> BTreeMap<NodeID, Self::HP>;
}

type HFloodRecord = Arc<Mutex<FloodRecord>>;
struct FloodRecord {
    pub peers_told: BTreeSet<NodeID>,
}

impl FloodRecord {
    fn insert(&mut self, peer_id: &NodeID) -> bool {
        self.peers_told.insert(peer_id.clone())
    }
}

impl Default for FloodRecord {
    fn default() -> Self {
        Self {
            peers_told: Default::default(),
        }
    }
}

struct FloodGate {
    flood_records: BTreeMap<u64, HFloodRecord>,
}

impl FloodGate {
    fn add_record(&mut self, msg_hash: &u64, peer_id: &NodeID) {
        match self.flood_records.get(msg_hash) {
            Some(record) => {
                record.lock().unwrap().insert(peer_id);
            }
            None => {
                let mut new_record: FloodRecord = Default::default();
                new_record.insert(peer_id);
                self.flood_records
                    .insert(msg_hash.clone(), Arc::new(Mutex::new(new_record)));
            }
        }
    }

    fn get_or_create_record(&mut self, msg_hash: &u64) -> HFloodRecord {
        match self.flood_records.get(msg_hash) {
            Some(val) => val.to_owned(),
            None => {
                // Create a new record
                let new = Arc::new(Mutex::new(Default::default()));
                self.flood_records.insert(msg_hash.clone(), new.clone());
                new
            }
        }
    }
}

struct OverlayManagerImpl {
    flood_gate: FloodGate,
    work_schedular: HWorkScheduler,
}

// impl<N> OverlayManager<N> for OverlayManagerImpl
// where
// N: NominationValue
// {
//     type HP = HPeer;
//     type P = Peer;

//     fn broadcast_message(&mut self, msg: &SCPMessage, force: bool, hash:
// Option<u64>) -> bool {         let mut broadcasted = false;
//         let peers = self.get_authenticated_peers();

//         let msg_hash = msg.to_hash();
//         let msg_to_send = Arc::new(Mutex::new(msg.clone()));

//         // Creating a new flood record.
//         let record = self.flood_gate.get_or_create_record(&msg_hash);

//         // Sending messages to peers to whom we haven't sent the msg before.
//         peers.values().into_iter().for_each(|peer| {
//             if !record.lock().unwrap().insert(&peer.lock().unwrap().id) {
//                 let weak = Arc::downgrade(peer);
//                 let mes_to_send_copy = msg_to_send.clone();

//                 let send_msg_predicate = move || {
//                     if let Some(strong) = weak.upgrade() {
//
// strong.lock().unwrap().send_message(&mes_to_send_copy)                     }
//                 };

//                 self.work_schedular
//                     .borrow_mut()
//                     .post_on_main_thread(Box::new(send_msg_predicate));
//                 broadcasted = true;
//             }
//         });

//         broadcasted
//     }

//     fn recv_flooded_message(&mut self, msg: &SCPMessage, peer: &Peer, msg_id:
// u64) {         self.flood_gate.add_record(&msg.to_hash(), &peer.id);
//     }

//     fn forget_flooded_message(&mut self, msg_id: &u64) {
//         self.flood_gate.flood_records.remove(msg_id);
//     }

//     fn remove_peer(&mut self, peer: &Peer) {
//         todo!()
//     }

//     fn get_authenticated_peers(&self) -> BTreeMap<NodeID, Self::HP> {
//         todo!()
//     }
// }
