use std::{collections::VecDeque, sync::{Mutex, Arc, Weak}};

use crate::application::work_queue::HWorkScheduler;

use super::{overlay_manager::SCPMessage, peer::SCPPeer};

struct LoopbackPeer {
    work_schedular: HWorkScheduler,
    out_queue: VecDeque<SCPMessage>,
    in_queue: VecDeque<SCPMessage>,
    remote: Weak<Mutex<LoopbackPeer>>,
}

impl LoopbackPeer {
    fn new(work_scheduler: &HWorkScheduler) -> Self {
        LoopbackPeer {
            work_schedular: work_scheduler.clone(),
            out_queue: Default::default(),
            in_queue: Default::default(),
            remote: Default::default(),
        }
    }

    fn process_in_queue(this: Arc<Mutex<Self>>) {
        let mut peer = this.lock().unwrap();

        if let Some(message) = peer.in_queue.pop_front() {
            peer.recv_message(&message);
        }

        // If we have more messages, process them on the main thread.
        if !peer.in_queue.is_empty() {
            let self_clone = Arc::downgrade(&this.clone());
            peer.work_schedular.lock().unwrap().post_on_main_thread(Box::new(move || {
                if let Some(p) = self_clone.upgrade() {
                    LoopbackPeer::process_in_queue(p);
                }
            }))
        }
    }
}

impl SCPPeer for LoopbackPeer {
    fn send_message(&mut self, envelope: &super::overlay_manager::HSCPMessage) {
        if let Some(remote) = self.remote.upgrade() {
            remote
            .lock().unwrap()
                .in_queue
                .push_back(envelope.lock().unwrap().clone());

            let remote_clone = self.remote.clone();

            self.work_schedular
                .lock()
                .unwrap()
                .post_on_main_thread(Box::new(move || {

                    if let Some(peer) = remote_clone.upgrade() {
                        LoopbackPeer::process_in_queue(peer); 
                    }
                }))
        }
    }

    fn recv_message(&mut self, message: &SCPMessage) {
        println!("{:?}", message);
        // todo!()
    }
}
