use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use crate::{
    application::work_queue::{ClockEvent, EventQueue},
    herder::herder::HerderDriver,
    scp::{
        nomination_protocol::NominationValue,
        scp::NodeID,
        scp_driver::{HSCPEnvelope, SCPEnvelope},
    },
};

use super::overlay_manager::{HSCPMessage, OverlayManager, SCPMessage};

type ArcState = Arc<Mutex<State>>;
pub type PeerID = &'static str;
pub type HPeer = Arc<Mutex<Peer>>;

struct State {
    value: usize,
}

pub struct Peer {
    pub id: NodeID,
    state: ArcState,
}

impl Peer {
    pub fn new() -> Self {
        Peer {
            state: Arc::new(Mutex::new(State::new())),
            id: todo!(),
        }
    }

    fn get_state(&mut self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    pub fn incr_one(&mut self) {
        self.get_state().incr_one();
    }

    pub fn add_to_queue(&mut self, work_queue: &mut EventQueue) {
        let clone = self.state.clone();
        let weak = Arc::downgrade(&clone);

        let callback = Box::new(move || {
            match weak.upgrade() {
                None => {
                    println!("State does not exist.")
                }
                Some(_state) => {
                    let mut state: std::sync::MutexGuard<'_, State> = _state.lock().unwrap();
                    state.incr_one();
                    println!("State with value {}", state.value);
                }
            };
        });
        let clock_event = ClockEvent {
            timestamp: SystemTime::now(),
            callback: callback,
        };
        work_queue.add_task(clock_event);
    }
}

impl State {
    pub fn new() -> Self {
        State { value: 0 }
    }

    pub fn incr_one(&mut self) {
        self.value += 1;
    }

    pub fn add_to_queue(this: Arc<Mutex<Self>>, work_queue: &mut EventQueue) -> () {
        let strong = this.clone();
        // let mut strong = self.clone();
        let weak = Arc::downgrade(&strong);

        let callback = Box::new(move || {
            match weak.upgrade() {
                None => {
                    println!("State does not exist.")
                }
                Some(_state) => {
                    let mut state: std::sync::MutexGuard<'_, State> = _state.lock().unwrap();
                    state.incr_one();
                    println!("State with value {}", state.value);
                }
            };
        });
        let clock_event = ClockEvent {
            timestamp: SystemTime::now(),
            callback: callback,
        };
        work_queue.add_task(clock_event);
    }

    pub fn to_callback<'a>(&'a mut self) -> impl FnMut() + 'a {
        let strong: Arc<Mutex<&mut State>> = Arc::new(Mutex::new(self));
        let weak = Arc::downgrade(&strong);

        let a = move || {
            match weak.upgrade() {
                None => {}
                Some(state) => {
                    state.lock().unwrap().incr_one();
                }
            };
        };
        a
    }
}

pub trait SCPPeer<N>
where
    N: NominationValue,
{

    fn peer_state(&mut self) -> &Rc<RefCell<SCPPeerState>>;
    fn herder(&self) -> &Rc<RefCell<dyn HerderDriver<N>>>;
    fn overlay_manager(
        &self,
    ) -> &Rc<RefCell<dyn OverlayManager<N, HP = Rc<RefCell<dyn SCPPeer<N>>>, P = dyn SCPPeer<N>>>>;

    fn send_message(&mut self, envelope: &HSCPMessage);

    fn recv_message(&mut self, message: &SCPMessage) {}
    fn connected(&mut self) {}

    fn recv_scp_message(&mut self, envelope: &SCPEnvelope<N>) {

        

        // We pass it to the herder
        self.herder()
            .as_ref()
            .borrow_mut()
            .recv_scp_envelope(envelope)
    }
}

// This struct maintains state neeed by the peer.
pub struct SCPPeerState {
    pub shutting_down: bool,
}
