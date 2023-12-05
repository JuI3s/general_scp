use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
};

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
            peer.work_schedular
                .lock()
                .unwrap()
                .post_on_main_thread(Box::new(move || {
                    if let Some(p) = self_clone.upgrade() {
                        LoopbackPeer::process_in_queue(p);
                    }
                }))
        }
    }
}

impl SCPPeer for LoopbackPeer {
    fn send_message(&mut self, envelope: &super::overlay_manager::HSCPMessage) {

        print!("Sending a message");

        if let Some(remote) = self.remote.upgrade() {
            remote
                .lock()
                .unwrap()
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
}

struct LoopbackPeerConnection {
    pub initiator: Arc<Mutex<LoopbackPeer>>,
    pub acceptor: Arc<Mutex<LoopbackPeer>>,
}

impl LoopbackPeerConnection {
    pub fn new(work_scheduler: &HWorkScheduler) -> Self {
        let initator: LoopbackPeer = LoopbackPeer::new(work_scheduler);
        let acceptor: LoopbackPeer = LoopbackPeer::new(work_scheduler);
        let initiator_handle = Arc::new(Mutex::new(initator));
        let acceptor_handle = Arc::new(Mutex::new(acceptor));
        initiator_handle.lock().unwrap().remote = Arc::downgrade(&acceptor_handle);
        acceptor_handle.lock().unwrap().remote = Arc::downgrade(&initiator_handle);
;
        LoopbackPeerConnection {
            initiator: initiator_handle,
            acceptor: acceptor_handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::application::work_queue::WorkScheduler;

    use super::*;

    #[test]
    fn send_hello_message() {
        let work_scheduler = Arc::new(Mutex::new(WorkScheduler::default()));
        let connection = LoopbackPeerConnection::new(&work_scheduler);
        let msg = Arc::new(Mutex::new(SCPMessage{}));
        
        connection.initiator.lock().unwrap().send_message(&msg);
        assert_eq!(connection.acceptor.lock().unwrap().in_queue.len(), 1);
        let num_tasks_done =  work_scheduler.lock().unwrap().excecute_main_thread_tasks();    
        assert_eq!(connection.acceptor.lock().unwrap().in_queue.len(), 0);
        assert_eq!(num_tasks_done, 1);
    }
}