use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    ca::cell::Cell,
    herder::herder::HerderDriver,
    scp::{local_node::HLocalNode, scp_driver::SlotDriver, slot::SlotIndex},
};

use super::{scp_driver::MockSCPDriver, state::MockState};

pub struct MockHerder {
    pub scp_driver: MockSCPDriver,
    pub local_node: HLocalNode<MockState>,
    pub scheduler: WorkScheduler,
}

impl MockHerder {
    fn new_slot(
        this: &Rc<RefCell<Self>>,
        slot_index: SlotIndex,
    ) -> SlotDriver<MockState, MockHerder> {
        SlotDriver::<MockState, MockHerder>::new(
            slot_index,
            this.borrow().local_node.clone(),
            this.borrow().scheduler.clone(),
            Default::default(),
            Default::default(),
            this.clone(),
        )
    }
}

impl HerderDriver<MockState> for MockHerder {
    fn combine_candidates(
        &self,
        candidates: &std::collections::BTreeSet<std::sync::Arc<MockState>>,
    ) -> Option<MockState> {
        todo!()
    }

    fn emit_envelope(&self, envelope: &crate::scp::scp_driver::SCPEnvelope<MockState>) {}

    fn extract_valid_value(&self, value: &MockState) -> Option<MockState> {
        Some(value.to_owned())
    }

    fn get_quorum_set(
        &self,
        statement: &crate::scp::statement::SCPStatement<MockState>,
    ) -> Option<crate::application::quorum::HQuorumSet> {
        todo!()
    }

    fn recv_scp_envelope(&mut self, envelope: &crate::scp::scp_driver::SCPEnvelope<MockState>) {
        // self.scp_driver
    }
}

#[cfg(test)]
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

        let a = A::new();
        A::get_b_to_say_hello(&a, 1);
        A::get_b_to_say_hello(&a, 2);
        A::get_b_to_say_hello(&a, 1);

        assert_eq!(a.borrow().val, 3);
    }
}
