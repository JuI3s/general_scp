
use std::{rc::{Rc, Weak}, sync::{Arc, Mutex}, collections::VecDeque, borrow::BorrowMut, ops::DerefMut};

type Callback<'a> = Arc<dyn FnMut() + 'a>;

struct State {
    value: usize,
}



impl State {
    pub fn new() -> Self {
        State { value: 0 }
    }

    pub fn incr_one(&mut self) {
        self.value += 1;
    }

    // pub fn add_to_queue(&'_ mut self, work_queue: &mut WorkQueue) {
    //     let strong = Arc::new(Mutex::new(self));
    //     let weak = Arc::downgrade(&strong); 

    //     let callback = move || {
    //         match weak.upgrade() {
    //             None => {},
    //             Some(state) =>{
    //                 state.lock().unwrap().incr_one();
    //             },
    //         };
    //     };
    //     work_queue.add_task(Arc::new(callback));
    // }

    pub fn add_to_queue<'a>(&'a mut self, work_queue: &mut WorkQueue<'a>) {
        let  strong = Arc::new(Mutex::new(self)).clone();
        // let mut strong = self.clone();
        let weak = Arc::downgrade(&strong);

        let callback = Arc::new(move || {
            match weak.upgrade() {
                None => {
                    println!("State does not exist.")
                },
                Some( _state) =>{
                    let mut state = _state.lock().unwrap();
                    state.incr_one();
                    println!("State with value {}", state.value);
                },
            };
        });
        work_queue.add_task(callback);
    }

    pub fn to_callback<'a>(&'a mut self) -> impl FnMut() + 'a  {
        let strong: Arc<Mutex<&mut State>> = Arc::new(Mutex::new(self));
        let weak = Arc::downgrade(&strong); 

        let a = move || {
            match weak.upgrade() {
                None => {},
                Some(state) =>{
                    state.lock().unwrap().incr_one();
                },
            };
        };
        a
    } 
}

struct WorkQueue<'a>
{
    tasks: VecDeque<Callback<'a>> 
    // TODO: add clock, etc
}

impl<'a> WorkQueue<'a> {
    pub fn new() -> Self {
        WorkQueue { tasks: VecDeque::<Callback>::new() }
    }

    pub fn add_task(&mut self, callback: Callback<'a>) -> () {
        self.tasks.push_back(callback);
    }

    pub fn execute_task(&mut self) {
        loop {
            match self.tasks.pop_front() {
                Some(mut callback) => {
                    Arc::get_mut(&mut callback).unwrap()();
                }, 
                None => break,
            }
        }
    }
}

fn main() {
    let mut work_queue = WorkQueue::new();
    let state1 = Arc::new(Mutex::new(State::new()));

    state1.lock().unwrap().add_to_queue(&mut work_queue);
    work_queue.execute_task();
    


    // let state = Arc::new(State{value: 0});
    // let weak = Arc::downgrade(&state);
    // 
    // let strong = weak.upgrade();
    // assert!(strong.is_some());
    // 
    // drop(strong);
    // drop(state);
// 
    // assert!(weak.upgrade().is_none());
}
