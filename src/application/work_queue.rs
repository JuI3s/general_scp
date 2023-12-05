use std::{
    alloc::System,
    collections::VecDeque,
    f32::consts::E,
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

pub type HWorkScheduler = Arc<Mutex<WorkScheduler>>;
pub struct WorkScheduler {
    main_thread_queue: MainWorkQueue,
}

impl WorkScheduler {
    pub fn post_on_main_thread(&mut self, callback: Callback) {
        self.main_thread_queue.add_task(callback);
    }

    pub fn execute_one_main_thread_task(&mut self) -> bool {
        self.main_thread_queue.execute_one_task()
    }

    pub fn excecute_main_thread_tasks(&mut self) -> u64 {
        self.main_thread_queue.execute_tasks()
    }
}

struct MainWorkQueue {
    tasks: VecDeque<Callback>,
}

impl Default for WorkScheduler {
    fn default() -> Self {
        Self {
            main_thread_queue: Default::default(),
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

    // Execute tasks and return the total number of tasks executed.
    fn execute_tasks(&mut self) -> u64 {
        let mut num_tasks_executed = 0;

        while let Some(top) = self.tasks.pop_front() {
            println!("Executing task... {}", num_tasks_executed);
            top();
            num_tasks_executed += 1;
        }

        num_tasks_executed
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

        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 0);

        // Adding a task
        work_scheduler.post_on_main_thread(Box::new(func));
        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 1);

        work_scheduler.execute_one_main_thread_task();
        assert_eq!(*pt.lock().unwrap(), 1);
        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 0);
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

        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 0);

        // Adding a task
        work_scheduler.post_on_main_thread(Box::new(func1));
        work_scheduler.post_on_main_thread(Box::new(func2));
        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 2);

        work_scheduler.excecute_main_thread_tasks();
        assert_eq!(*pt.lock().unwrap(), 2);
        assert_eq!(work_scheduler.main_thread_queue.tasks.len(), 0);
    }
}
