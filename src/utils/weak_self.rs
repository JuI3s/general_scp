use std::{
    sync::Mutex,
    sync::{Arc, Weak},
};

pub trait WeakSelf {
    fn get_weak_self(self: Arc<Self>) -> Weak<Mutex<Self>>;
}
