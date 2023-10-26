use std::{alloc::System, collections::VecDeque, sync::Arc, time::SystemTime};

use super::clock::{HVirtualClock, VirtualClock};

pub type Callback = Arc<dyn FnMut()>;

pub struct ClockEvent {
    pub timestamp: SystemTime,
    pub callback: Callback,
}

pub struct WorkQueue {
    clock: HVirtualClock,
    tasks: VecDeque<ClockEvent>,
}

impl WorkQueue {
    pub fn new(clock: HVirtualClock) -> Self {
        WorkQueue {
            clock: clock,
            tasks: VecDeque::<ClockEvent>::new(),
        }
    }

    pub fn add_task(&mut self, callback: ClockEvent) -> () {
        self.tasks.push_back(callback);
    }

    fn event_expired(&mut self, timestamp: &SystemTime) -> bool {
        self.clock.lock().unwrap().time_now() >= timestamp
    }

    pub fn execute_task(&mut self) {
        loop {
            match self.tasks.pop_front() {
                Some(mut clock_event) => {
                    if self.event_expired(&clock_event.timestamp) {
                        Arc::get_mut(&mut clock_event.callback).unwrap()();
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
    }
}
