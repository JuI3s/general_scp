use core::time;
use std::{
    alloc::System,
    fmt::DebugStruct,
    sync::{Arc, Mutex},
    time::SystemTime,
};

pub type HVirtualClock = Arc<Mutex<VirtualClock>>;
pub struct VirtualClock {
    time_now: SystemTime,
}

impl VirtualClock {
    pub fn new(time_now: SystemTime) -> Self {
        VirtualClock { time_now: time_now }
    }

    pub fn new_clock() -> HVirtualClock {
        Arc::new(Mutex::new(VirtualClock {
            time_now: SystemTime::now(),
        }))
    }

    pub fn set_current_virtual_time(&mut self, time_now: SystemTime) {
        self.time_now = time_now;
    }

    pub fn time_now(&self) -> &SystemTime {
        &self.time_now
    }
}
