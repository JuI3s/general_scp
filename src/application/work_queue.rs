use std::{
    alloc::System,
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use super::clock::{HVirtualClock, VirtualClock};

pub type Callback = Box<dyn FnOnce()>;

pub struct ClockEvent {
    pub timestamp: SystemTime,
    pub callback: Callback,
}

impl ClockEvent {
    pub fn new(timestamp: SystemTime, callback: Callback) -> Self {
        ClockEvent {
            timestamp: timestamp,
            callback: callback,
        }
    }
}

pub type HWorkQueue = Arc<Mutex<WorkQueue>>;
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
                Some(clock_event) => {
                    if self.event_expired(&clock_event.timestamp) {
                        (clock_event.callback)();
                        // let val = clock_event.callback.to_owned();
                        // let result = Arc::as_ref(&clock_event.callback);
                        // Arc::(&mut clock_event.callback).unwrap()();
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
    }
}
