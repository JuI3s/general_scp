use crate::{
    herder::herder::HerderDriver,
    mock::state::{MockState, MockStateDriver},
    overlay::message::SCPMessage,
    scp::{envelope::MakeEnvelope, statement::MakeStatement},
};

use std::{
    cell::{Ref, RefCell},
    collections::VecDeque,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use crate::{application::work_queue::HWorkScheduler, scp::nomination_protocol::NominationValue};

use super::peer::{SCPPeer, SCPPeerState};

impl MakeStatement<MockState> for LoopbackPeer<MockState, MockStateDriver> {
    fn new_nominate_statement(&self) -> crate::scp::statement::SCPStatementNominate<MockState> {
        self.herder.borrow().new_nominate_statement()
    }
}

impl MakeEnvelope<MockState> for LoopbackPeer<MockState, MockStateDriver> {
    fn new_nomination_envelope(
        &self,
        slot_index: usize,
    ) -> crate::scp::envelope::SCPEnvelope<MockState> {
        self.herder.borrow().new_nomination_envelope(slot_index)
    }
}
pub struct LoopbackPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    work_schedular: HWorkScheduler,
    out_queue: VecDeque<SCPMessage<N>>,
    pub in_queue: VecDeque<SCPMessage<N>>,
    remote: Weak<RefCell<LoopbackPeer<N, H>>>,
    state: Rc<RefCell<SCPPeerState>>,
    herder: Rc<RefCell<H>>,
}

impl<N, H> LoopbackPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    fn new(
        work_scheduler: &HWorkScheduler,
        we_called_remote: bool,
        herder: Rc<RefCell<H>>,
    ) -> Self {
        LoopbackPeer {
            work_schedular: work_scheduler.clone(),
            out_queue: Default::default(),
            in_queue: Default::default(),
            remote: Default::default(),
            state: SCPPeerState::new(we_called_remote).into(),
            herder: herder,
        }
    }

    pub fn process_in_queue(this: &Rc<RefCell<Self>>) {
        let mut peer = this.borrow_mut();

        if let Some(message) = peer.in_queue.pop_front() {
            peer.recv_message(&message);
        }

        // If we have more messages, process them on the main thread.
        if !peer.in_queue.is_empty() {
            let self_clone = Rc::downgrade(&this.clone());
            peer.work_schedular
                .borrow()
                .post_on_main_thread(Box::new(move || {
                    if let Some(p) = self_clone.upgrade() {
                        LoopbackPeer::process_in_queue(&p);
                    }
                }))
        }
    }
}

impl<N, H> SCPPeer<N, H> for LoopbackPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    fn peer_state(&mut self) -> &std::rc::Rc<std::cell::RefCell<super::peer::SCPPeerState>> {
        &self.state
    }

    fn id(&self) -> &crate::scp::scp::NodeID {
        todo!()
    }

    fn overlay_manager(
        &self,
    ) -> &std::rc::Rc<
        std::cell::RefCell<
            dyn super::overlay_manager::OverlayManager<
                N,
                H,
                HP = std::rc::Rc<std::cell::RefCell<Self>>,
                P = Self,
            >,
        >,
    > {
        todo!()
    }

    fn send_message(&mut self, msg: &SCPMessage<N>) {
        println!("Sending a message");

        if let Some(remote) = self.remote.upgrade() {
            remote.borrow_mut().in_queue.push_back(msg.to_owned());

            let remote_clone = self.remote.clone();

            self.work_schedular
                .borrow()
                .post_on_main_thread(Box::new(move || {
                    if let Some(peer) = remote_clone.upgrade() {
                        LoopbackPeer::process_in_queue(&peer);
                    }
                }));
        }
    }

    fn herder(&self) -> Rc<RefCell<H>> {
        self.herder.clone()
    }
}

pub struct LoopbackPeerConnection<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub initiator: Rc<RefCell<LoopbackPeer<N, H>>>,
    pub acceptor: Rc<RefCell<LoopbackPeer<N, H>>>,
}

impl<N, H> LoopbackPeerConnection<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    pub fn new(
        work_scheduler: &HWorkScheduler,
        herder1: Rc<RefCell<H>>,
        herder2: Rc<RefCell<H>>,
    ) -> Self {
        let initator = LoopbackPeer::<N, H>::new(work_scheduler, true, herder1);
        let acceptor = LoopbackPeer::<N, H>::new(work_scheduler, false, herder2);
        let initiator_handle = Rc::new(RefCell::new(initator));
        let acceptor_handle = Rc::new(RefCell::new(acceptor));

        // Setting remote peers
        initiator_handle.borrow_mut().remote = Rc::downgrade(&acceptor_handle);
        acceptor_handle.borrow_mut().remote = Rc::downgrade(&initiator_handle);

        // Setting connection states
        initiator_handle
            .borrow_mut()
            .state
            .borrow_mut()
            .set_conn_state(super::peer::SCPPeerConnState::Connected);
        acceptor_handle
            .borrow_mut()
            .state
            .borrow_mut()
            .set_conn_state(super::peer::SCPPeerConnState::Connected);

        let initiator_weak = Rc::downgrade(&initiator_handle.clone());
        initiator_handle
            .borrow_mut()
            .work_schedular
            .borrow()
            .post_on_main_thread(Box::new(move || {
                if let Some(peer) = initiator_weak.upgrade() {
                    peer.borrow_mut().connect_handler();
                }
            }));

        LoopbackPeerConnection {
            initiator: initiator_handle,
            acceptor: acceptor_handle,
        }
    }
}
