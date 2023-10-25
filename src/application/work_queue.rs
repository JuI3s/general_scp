use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use super::clock::{HVirtualClock, VirtualClock};

pub struct ClockEvent {
    pub timestamp: SystemTime,
    pub callback: Arc<dyn FnMut()>,
}

pub struct WorkQueue {
    clock: HVirtualClock,
    tasks: VecDeque<ClockEvent>, // TODO: add clock, etc
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

    pub fn execute_task(&mut self) {
        loop {
            match self.tasks.pop_front() {
                Some(mut clock_event) => {
                    if clock_event.timestamp <= SystemTime::now() {
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
