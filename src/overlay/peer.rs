use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    env,
    marker::PhantomData,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use log::info;

use crate::{
    application::work_queue::{ClockEvent, EventQueue},
    herder::herder::HerderDriver,
    scp::{
        envelope::{SCPEnvelope, SCPEnvelopeController, SCPEnvelopeID},
        nomination_protocol::NominationValue,
        scp::NodeID,
    },
};

use super::{
    message::{HelloEnvelope, SCPMessage},
    overlay_manager::OverlayManager,
};

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

        let timestamp = SystemTime::now();
        let clock_event = ClockEvent::new(timestamp, callback);
        work_queue.add_task(&timestamp, clock_event.into());
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

        let timestamp = SystemTime::now();
        let clock_event = ClockEvent::new(timestamp.to_owned(), callback);
        work_queue.add_task(&timestamp, clock_event.into());
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

pub enum SCPPeerConnState {
    Connecting,
    Connected,
    GotAuth,
    GotHello,
    Closing,
}

pub struct SCPPeer<N, C>
where
    N: NominationValue,
    C: PeerConn<N>,
{
    pub node_id: NodeID,
    pub state: SCPPeerState,
    pub conn: C,
    phantom: PhantomData<N>,
}

impl<N, C> SCPPeer<N, C>
where
    N: NominationValue,
    C: PeerConn<N>,
{
    pub fn on_connect(&mut self) {
        self.state.set_conn_state(SCPPeerConnState::Connected);

        let hello_env = HelloEnvelope {};
        self.conn.send_hello(hello_env);
    }
}

pub trait PeerConn<N>
where
    N: NominationValue,
    Self: Sized,
{
    // Implemented by struct implementing the trait.
    fn send_message(&mut self, msg: &SCPMessage<N>);

    fn send_hello(&mut self, envelope: HelloEnvelope) {
        self.send_message(&SCPMessage::Hello(envelope))
    }

    fn send_scp_msg(&mut self, envelope: SCPEnvelope<N>) {
        self.send_message(&SCPMessage::SCP(envelope))
    }
}

// This struct maintains state neeed by the peer.
pub struct SCPPeerState {
    pub conn_state: SCPPeerConnState,
    pub shutting_down: bool,
}

impl SCPPeerState {
    pub fn new(we_called_remote: bool) -> Self {
        SCPPeerState {
            conn_state: {
                if we_called_remote {
                    SCPPeerConnState::Connecting
                } else {
                    SCPPeerConnState::Connected
                }
            },
            shutting_down: false,
        }
    }

    pub fn set_conn_state(&mut self, state: SCPPeerConnState) {
        self.conn_state = state
    }
}

impl Into<Rc<RefCell<SCPPeerState>>> for SCPPeerState {
    fn into(self) -> Rc<RefCell<SCPPeerState>> {
        Rc::new(RefCell::new(self))
    }
}
