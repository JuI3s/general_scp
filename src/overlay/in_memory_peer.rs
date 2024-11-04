// use std::{
//     cell::RefCell,
//     collections::VecDeque,
//     rc::{Rc, Weak},
// };

// use crate::{
//     application::work_queue::HWorkScheduler, herder::herder::HerderDriver,
//     scp::nomination_protocol::NominationValue,
// };

// use super::{
//     message::SCPMessage,
//     peer::{SCPPeer, SCPPeerState},
// };

// pub struct InMemoryPeer<N, H>
// where
//     N: NominationValue,
//     H: HerderDriver<N>,
// {
//     work_schedular: HWorkScheduler,
//     in_queue: VecDeque<SCPMessage<N>>,
//     state: Rc<RefCell<SCPPeerState>>,
//     herder: Rc<RefCell<H>>,
// }

// impl<N, H> InMemoryPeer<N, H>
// where
//     N: NominationValue,
//     H: HerderDriver<N> + 'static,
// {
//     fn new(
//         work_scheduler: &HWorkScheduler,
//         we_called_remote: bool,
//         herder: Rc<RefCell<H>>,
//     ) -> Self {
//         Self {
//             work_schedular: work_scheduler.clone(),
//             in_queue: Default::default(),
//             state: SCPPeerState::new(we_called_remote).into(),
//             herder: herder,
//         }
//     }

//     pub fn process_in_queue(this: &Rc<RefCell<Self>>) {
//         let mut peer = this.borrow_mut();

//         if let Some(message) = peer.in_queue.pop_front() {
//             peer.recv_message(&message);
//         }

//         // If we have more messages, process them on the main thread.
//         if !peer.in_queue.is_empty() {
//             let self_clone = Rc::downgrade(&this.clone());
//             peer.work_schedular
//                 .borrow()
//                 .post_on_main_thread(Box::new(move || {
//                     if let Some(p) = self_clone.upgrade() {
//                         Self::process_in_queue(&p);
//                     }
//                 }))
//         }
//     }
// }

// impl<N, H> SCPPeer<N, H> for InMemoryPeer<N, H>
// where
//     N: NominationValue,
//     H: HerderDriver<N>,
// {
//     fn id(&self) -> &crate::scp::scp::NodeID {
//         todo!()
//     }

//     fn peer_state(&mut self) -> &Rc<RefCell<SCPPeerState>> {
//         todo!()
//     }

//     fn herder(&self) -> Rc<RefCell<H>> {
//         todo!()
//     }

//     fn overlay_manager(
//         &self,
//     ) -> &Rc<
//         RefCell<dyn super::overlay_manager::OverlayManager<N, H, HP = Rc<RefCell<Self>>, P = Self>>,
//     > {
//         todo!()
//     }

//     fn send_message(&mut self, msg: &SCPMessage<N>) {
//         todo!()
//     }
// }
