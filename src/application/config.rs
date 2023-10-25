use super::clock::{HVirtualClock, VirtualClock};

pub struct Config {
    pub clock: HVirtualClock,
}

impl Config {
    pub fn new(clock: HVirtualClock) -> Self {  
        Config{clock: clock.clone()}
    }

    pub fn new_config() -> Self {
        let clock = VirtualClock::new_clock();
        Config { clock: clock }
    }
}