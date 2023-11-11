use std::{sync::Mutex, sync::{Weak, Arc}};

pub trait WeakSelf {
    fn get_weak_self(self: Arc<Self>) -> Weak<Mutex<Self>>;
}
