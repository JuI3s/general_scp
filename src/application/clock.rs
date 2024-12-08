use std::{
    cell::RefCell,
    rc::Rc,
    time::SystemTime,
};

pub type HVirtualClock = Rc<RefCell<VirtualClock>>;
pub struct VirtualClock {
    time_now: SystemTime,
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self {
            time_now: SystemTime::now(),
        }
    }
}

impl VirtualClock {
    pub fn new(time_now: SystemTime) -> Self {
        VirtualClock { time_now: time_now }
    }

    pub fn new_clock() -> HVirtualClock {
        Rc::new(RefCell::new(VirtualClock {
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
