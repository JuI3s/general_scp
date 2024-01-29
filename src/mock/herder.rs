use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    ca::cell::Cell,
    herder::herder::HerderDriver,
    scp::{local_node::HLocalNode, scp_driver::SlotDriver, slot::SlotIndex},
};

use super::{scp_driver::MockSCPDriver, state::MockState};


mod tests {
    use std::collections::HashMap;

    use syn::token::Ref;

    use super::*;

    #[test]
    fn borrow_rule() {
        struct A {
            entries: HashMap<u16, Rc<RefCell<B>>>,
            pub val: usize,
        };

        impl A {
            pub fn new() -> Rc<RefCell<Self>> {
                Rc::new(RefCell::new(A {
                    entries: Default::default(),
                    val: 0,
                }))
            }

            pub fn say_hello(&mut self) {
                self.val += 1;
            }

            pub fn get_b_to_say_hello(this: &Rc<RefCell<Self>>, key: u16) {
                let b = this
                    .borrow_mut()
                    .entries
                    .entry(key)
                    .or_insert(Self::new_b(&this).into())
                    .to_owned();
                b.borrow_mut().say_hello();
            }

            pub fn new_b(this: &Rc<RefCell<A>>) -> B {
                B { a: this.to_owned() }
            }
        }

        struct B {
            a: Rc<RefCell<A>>,
        };

        impl Into<Rc<RefCell<B>>> for B {
            fn into(self) -> Rc<RefCell<B>> {
                Rc::new(RefCell::new(self))
            }
        }

        impl B {
            pub fn say_hello(&mut self) {
                self.a.borrow_mut().say_hello();
            }
        }

        #[derive(Debug, PartialEq, Eq)]
        struct C {
            a: u64,
            b: u64,
        }

        impl C {
            pub fn new(a: u64, b: u64) -> Self {
                Self { a: a, b: b }
            }

            pub fn modify(a: &mut u64, mut b: &mut u64) {
                *a += 1;
                *b += 1;
            }

            pub fn self_modify(&mut self) {
                Self::modify(&mut self.a, &mut self.b);
            }
        }

        let a = A::new();
        A::get_b_to_say_hello(&a, 1);
        A::get_b_to_say_hello(&a, 2);
        A::get_b_to_say_hello(&a, 1);

        let mut c = C::new(0, 1);
        // C::modify(&mut c.a, &mut c.b);
        c.self_modify();
        assert_eq!(c, C::new(1, 2));

        assert_eq!(a.borrow().val, 3);
    }
}
