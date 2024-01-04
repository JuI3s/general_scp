use std::{
    alloc::System,
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    f32::consts::E,
    rc::Rc,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use serde_derive::{Deserialize, Serialize};

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

pub type HWorkScheduler = Rc<RefCell<WorkScheduler>>;
#[derive(Clone)]
pub struct WorkScheduler {
    main_thread_queue: Rc<RefCell<MainWorkQueue>>,
    event_queue: Rc<RefCell<EventQueue>>,
}

impl WorkScheduler {
    pub fn new(clock: HVirtualClock) -> Self {
        WorkScheduler {
            main_thread_queue: Default::default(),
            event_queue: EventQueue::new(clock).into(),
        }
    }

    pub fn post_on_main_thread(&self, callback: Callback) {
        self.main_thread_queue.borrow_mut().add_task(callback);
    }

    pub fn execute_one_main_thread_task(&self) -> bool {
        self.main_thread_queue.borrow_mut().execute_one_task()
    }

    pub fn excecute_main_thread_tasks(&self) -> u64 {
        let mut num_executed = 0;
        loop {
            // Important: need to move value to cb_opt to avoid borrow_mut main_thread_queue
            // twice in loopback peer testing.
            let cb_opt = self.main_thread_queue.borrow_mut().pop();
            match cb_opt {
                Some(cb) => {
                    cb();
                    num_executed += 1;
                }
                None => {
                    return num_executed;
                }
            }
        }

        // self.main_thread_queue.borrow_mut().execute_tasks()
    }

    pub fn post_clock_event(&self, clock_event: ClockEvent) {
        self.event_queue.borrow_mut().add_task(clock_event)
    }
}

struct MainWorkQueue {
    tasks: VecDeque<Callback>,
}

impl Default for WorkScheduler {
    fn default() -> Self {
        Self {
            main_thread_queue: Default::default(),
            event_queue: EventQueue::new(VirtualClock::new_clock()).into(),
        }
    }
}

impl Default for MainWorkQueue {
    fn default() -> Self {
        Self {
            tasks: Default::default(),
        }
    }
}

impl MainWorkQueue {
    fn add_task(&mut self, callback: Callback) {
        self.tasks.push_back(Box::new(callback))
    }

    fn execute_one_task(&mut self) -> bool {
        if let Some(front) = self.tasks.pop_front() {
            front();
            true
        } else {
            false
        }
    }

    fn pop(&mut self) -> Option<Callback> {
        self.tasks.pop_front()
    }
}

pub struct EventQueue {
    clock: HVirtualClock,
    tasks: BTreeMap<SystemTime, Vec<Callback>>,
}

impl Into<Rc<RefCell<EventQueue>>> for EventQueue {
    fn into(self) -> Rc<RefCell<EventQueue>> {
        Rc::new(RefCell::new(self))
    }
}

impl EventQueue {
    pub fn new(clock: HVirtualClock) -> Self {
        EventQueue {
            clock: clock,
            tasks: Default::default(),
        }
    }

    pub fn add_task(&mut self, event: ClockEvent) -> () {
        if let Some(callbacks) = self.tasks.get_mut(&event.timestamp) {
            callbacks.push(event.callback);
        } else {
            self.tasks.insert(event.timestamp, vec![event.callback]);
        }
        // self.tasks.push_back(callback);
    }

    fn event_expired(&self, timestamp: &SystemTime) -> bool {
        self.clock.borrow().time_now() >= timestamp
    }

    pub fn execute_task(&mut self) {
        let mut elapsed_timestamps = vec![];

        // TODO: Can we avoid copying system time?
        for (ts, _) in &self.tasks {
            if self.event_expired(ts) {
                elapsed_timestamps.push(ts.to_owned());
            }
        }

        for ts in elapsed_timestamps {
            let callbacks: Vec<Box<dyn FnOnce()>> = self.tasks.remove(&ts).unwrap();
            for cb in callbacks {
                cb();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::application::work_queue::WorkScheduler;

    use super::*;

    #[test]
    fn add_one_task_to_work_schedular() {
        let mut work_scheduler = WorkScheduler::default();
        let x = 0;
        let pt = Arc::new(Mutex::new(x));
        let pt_copy = pt.clone();
        let func = move || {
            *pt_copy.lock().unwrap() += 1;
        };

        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 0);

        // Adding a task
        work_scheduler.post_on_main_thread(Box::new(func));
        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 1);

        work_scheduler.execute_one_main_thread_task();
        assert_eq!(*pt.lock().unwrap(), 1);
        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 0);
    }

    #[test]
    fn add_two_tasks_and_execute_both() {
        let mut work_scheduler = WorkScheduler::default();
        let x = 0;
        let pt = Arc::new(Mutex::new(x));
        let pt_copy_1 = pt.clone();
        let pt_copy_2 = pt.clone();

        let func1 = move || {
            *pt_copy_1.lock().unwrap() += 1;
        };
        let func2 = move || {
            *pt_copy_2.lock().unwrap() += 1;
        };

        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 0);

        // Adding a task
        work_scheduler.post_on_main_thread(Box::new(func1));
        work_scheduler.post_on_main_thread(Box::new(func2));
        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 2);

        work_scheduler.excecute_main_thread_tasks();
        assert_eq!(*pt.lock().unwrap(), 2);
        assert_eq!(work_scheduler.main_thread_queue.borrow().tasks.len(), 0);
    }
}
