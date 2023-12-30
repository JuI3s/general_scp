use crate::overlay::message::SCPMessage;

use std::{
    cell::{Ref, RefCell},
    collections::VecDeque,
    rc::{Rc, Weak},
};

use crate::{application::work_queue::HWorkScheduler, scp::nomination_protocol::NominationValue};

use super::peer::{SCPPeer, SCPPeerState};

struct LoopbackPeer<N>
where
    N: NominationValue,
{
    work_schedular: HWorkScheduler,
    out_queue: VecDeque<SCPMessage<N>>,
    in_queue: VecDeque<SCPMessage<N>>,
    remote: Weak<RefCell<LoopbackPeer<N>>>,
    state: Rc<RefCell<SCPPeerState>>,
}

impl<N> LoopbackPeer<N>
where
    N: NominationValue,
{
    fn new(work_scheduler: &HWorkScheduler, we_called_remote: bool) -> Self {
        LoopbackPeer {
            work_schedular: work_scheduler.clone(),
            out_queue: Default::default(),
            in_queue: Default::default(),
            remote: Default::default(),
            state: SCPPeerState::new(we_called_remote).into(),
        }
    }

    fn process_in_queue(this: Rc<RefCell<Self>>) {
        let mut peer = this.borrow_mut();

        if let Some(message) = peer.in_queue.pop_front() {
            peer.recv_message(&message);
        }

        // If we have more messages, process them on the main thread.
        if !peer.in_queue.is_empty() {
            let self_clone = Rc::downgrade(&this.clone());
            peer.work_schedular
                .borrow_mut()
                .post_on_main_thread(Box::new(move || {
                    if let Some(p) = self_clone.upgrade() {
                        LoopbackPeer::process_in_queue(p);
                    }
                }))
        }
    }
}

impl<N> SCPPeer<N> for LoopbackPeer<N>
where
    N: NominationValue,
{
    fn peer_state(&mut self) -> &std::rc::Rc<std::cell::RefCell<super::peer::SCPPeerState>> {
        &self.state
    }

    fn id(&self) -> &crate::scp::scp::NodeID {
        todo!()
    }

    fn herder(
        &self,
    ) -> &std::rc::Rc<std::cell::RefCell<dyn crate::herder::herder::HerderDriver<N>>> {
        todo!()
    }

    fn overlay_manager(
        &self,
    ) -> &std::rc::Rc<
        std::cell::RefCell<
            dyn super::overlay_manager::OverlayManager<
                N,
                HP = std::rc::Rc<std::cell::RefCell<Self>>,
                P = Self,
            >,
        >,
    > {
        todo!()
    }

    fn send_message(&mut self, msg: &SCPMessage<N>) {
        print!("Sending a message");

        if let Some(remote) = self.remote.upgrade() {
            remote.borrow_mut().in_queue.push_back(msg.to_owned());

            let remote_clone = self.remote.clone();

            self.work_schedular
                .borrow_mut()
                .post_on_main_thread(Box::new(move || {
                    if let Some(peer) = remote_clone.upgrade() {
                        LoopbackPeer::process_in_queue(peer);
                    }
                }))
        }
    }
}

struct LoopbackPeerConnection<N>
where
    N: NominationValue,
{
    pub initiator: Rc<RefCell<LoopbackPeer<N>>>,
    pub acceptor: Rc<RefCell<LoopbackPeer<N>>>,
}

impl<N> LoopbackPeerConnection<N>
where
    N: NominationValue,
{
    pub fn new(work_scheduler: &HWorkScheduler) -> Self {
        let initator = LoopbackPeer::<N>::new(work_scheduler, true);
        let acceptor = LoopbackPeer::<N>::new(work_scheduler, false);
        let initiator_handle = Rc::new(RefCell::new(initator));
        let acceptor_handle = Rc::new(RefCell::new(acceptor));
        initiator_handle.borrow_mut().remote = Rc::downgrade(&acceptor_handle);
        acceptor_handle.borrow_mut().remote = Rc::downgrade(&initiator_handle);
        LoopbackPeerConnection {
            initiator: initiator_handle,
            acceptor: acceptor_handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::work_queue::WorkScheduler, mock::state::MockState,
        overlay::message::HelloEnvelope,
    };

    use super::*;

    #[test]
    fn send_hello_message() {
        let work_scheduler = Rc::new(RefCell::new(WorkScheduler::default()));
        let connection = LoopbackPeerConnection::<MockState>::new(&work_scheduler);
        let msg = HelloEnvelope {};

        connection.initiator.borrow_mut().send_hello(msg);
        assert_eq!(connection.acceptor.borrow_mut().in_queue.len(), 1);
        let num_tasks_done = work_scheduler.borrow_mut().excecute_main_thread_tasks();
        assert_eq!(connection.acceptor.borrow_mut().in_queue.len(), 0);
        assert_eq!(num_tasks_done, 1);
    }
}
