use std::{sync::Mutex, sync::Weak};

pub trait WeakSelf {
    fn get_weak_self(&mut self) -> Weak<Mutex<&mut Self>>;
}
