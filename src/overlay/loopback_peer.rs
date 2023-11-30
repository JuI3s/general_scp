use std::{collections::VecDeque, rc::Weak, sync::Mutex};

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

    fn process_in_queue(&mut self) {
        todo!();
    }
}

impl SCPPeer for LoopbackPeer {
    fn send_message(&self, envelope: &super::overlay_manager::HSCPMessage) {
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
                        peer.lock().unwrap().process_in_queue();
                    }
                }))
        }
    }
}
