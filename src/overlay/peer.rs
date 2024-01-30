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
    scp::{envelope::SCPEnvelope, nomination_protocol::NominationValue, scp::NodeID},
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

pub trait SCPPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
    Self: Sized,
{
    fn id(&self) -> &NodeID;
    fn peer_state(&mut self) -> &Rc<RefCell<SCPPeerState>>;
    fn herder(&self) -> &Rc<RefCell<H>>;
    fn overlay_manager(
        &self,
    ) -> &Rc<RefCell<dyn OverlayManager<N, H, HP = Rc<RefCell<Self>>, P = Self>>>;

    // Setting state on connected
    fn set_state_on_connected(&mut self) {
        self.borrow_mut()
            .peer_state()
            .as_ref()
            .borrow_mut()
            .set_conn_state(SCPPeerConnState::Connected);
    }

    fn connect_handler(&mut self) {
        self.set_state_on_connected();

        let hello_env = HelloEnvelope {};
        self.send_hello(hello_env);
    }

    // Implemented by struct implementing the trait.
    fn send_message(&mut self, msg: &SCPMessage<N>);

    fn send_hello(&mut self, envelope: HelloEnvelope) {
        self.send_message(&SCPMessage::Hello(envelope))
    }

    fn send_scp_msg(&mut self, envelope: SCPEnvelope<N>) {
        self.send_message(&SCPMessage::SCP(envelope))
    }

    fn recv_message(&mut self, msg: &SCPMessage<N>) {
        // if msg.is_boardcast_msg() {
        //     self.overlay_manager()
        //         .as_ref()
        //         .borrow_mut()
        //         .recv_flooded_message(msg, self)
        // }

        match msg {
            SCPMessage::SCP(scp_envelope) => self.recv_scp_envelope(scp_envelope),
            SCPMessage::Hello(hello) => self.recv_hello_envelope(hello),
        }
    }

    fn recv_hello_envelope(&mut self, enevlope: &HelloEnvelope) {
        self.peer_state()
            .as_ref()
            .borrow_mut()
            .set_conn_state(SCPPeerConnState::GotHello);
    }

    fn recv_scp_envelope(&mut self, envelope: &SCPEnvelope<N>) {
        // We pass it to the herder
        H::recv_scp_envelope(self.herder(), envelope);
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
