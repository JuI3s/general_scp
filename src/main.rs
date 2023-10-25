use std::{
    borrow::BorrowMut,
    collections::VecDeque,
    fs::Permissions,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

type Callback = Arc<dyn FnMut()>;

struct State {
    value: usize,
}

type ArcState = Arc<Mutex<State>>;
struct Peer {
    state: ArcState,
}

impl Peer {
    pub fn new() -> Self {
        Peer {
            state: Arc::new(Mutex::new(State::new())),
        }
    }

    fn get_state(&mut self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    pub fn incr_one(&mut self) {
        self.get_state().incr_one();
    }

    pub fn add_to_queue(&mut self, work_queue: &mut WorkQueue) {
        let clone = self.state.clone();
        let weak = Arc::downgrade(&clone);

        let callback = Arc::new(move || {
            match weak.upgrade() {
                None => {
                    println!("State does not exist.")
                }
                Some(_state) => {
                    let mut state: std::sync::MutexGuard<'_, State> = _state.lock().unwrap();
                    state.incr_one();
                    println!("State with value {}", state.value);
                }
            };
        });
        work_queue.add_task(callback);
    }
}

impl State {
    pub fn new() -> Self {
        State { value: 0 }
    }

    pub fn incr_one(&mut self) {
        self.value += 1;
    }

    pub fn add_to_queue(this: Arc<Mutex<Self>>, work_queue: &mut WorkQueue) -> () {
        let strong = this.clone();
        // let mut strong = self.clone();
        let weak = Arc::downgrade(&strong);

        let callback = Arc::new(move || {
            match weak.upgrade() {
                None => {
                    println!("State does not exist.")
                }
                Some(_state) => {
                    let mut state: std::sync::MutexGuard<'_, State> = _state.lock().unwrap();
                    state.incr_one();
                    println!("State with value {}", state.value);
                }
            };
        });
        work_queue.add_task(callback);
    }

    pub fn to_callback<'a>(&'a mut self) -> impl FnMut() + 'a {
        let strong: Arc<Mutex<&mut State>> = Arc::new(Mutex::new(self));
        let weak = Arc::downgrade(&strong);

        let a = move || {
            match weak.upgrade() {
                None => {}
                Some(state) => {
                    state.lock().unwrap().incr_one();
                }
            };
        };
        a
    }
}

struct WorkQueue {
    tasks: VecDeque<Callback>, // TODO: add clock, etc
}

impl WorkQueue {
    pub fn new() -> Self {
        WorkQueue {
            tasks: VecDeque::<Callback>::new(),
        }
    }

    pub fn add_task(&mut self, callback: Callback) -> () {
        self.tasks.push_back(callback);
    }

    pub fn execute_task(&mut self) {
        loop {
            match self.tasks.pop_front() {
                Some(mut callback) => {
                    Arc::get_mut(&mut callback).unwrap()();
                }
                None => break,
            }
        }
    }
}

fn main() {
    let mut work_queue = WorkQueue::new();
    let mut peer = Peer::new();
    peer.incr_one();
    peer.add_to_queue(&mut work_queue);
    work_queue.execute_task();
}
