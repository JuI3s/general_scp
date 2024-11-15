use syn::token::Ref;

use crate::{
    herder::herder::HerderDriver,
    mock::state::{MockState, MockStateDriver},
    overlay::message::SCPMessage,
    scp::{
        envelope::{MakeEnvelope, SCPEnvelopeController},
        statement::MakeStatement,
    },
};

use std::{
    cell::RefCell,
    collections::VecDeque,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use crate::{application::work_queue::HWorkScheduler, scp::nomination_protocol::NominationValue};

use super::peer::{SCPPeerState};

impl MakeStatement<MockState> for LoopbackPeer<MockState, MockStateDriver> {
    fn new_nominate_statement(
        &self,
        vote: MockState,
    ) -> crate::scp::statement::SCPStatementNominate<MockState> {
        self.herder.borrow().new_nominate_statement(vote)
    }
}

impl MakeEnvelope<MockState> for LoopbackPeer<MockState, MockStateDriver> {
    fn new_nomination_envelope(
        &self,
        slot_index: usize,
        vote: MockState,
    ) -> crate::scp::envelope::SCPEnvelope<MockState> {
        self.herder
            .borrow()
            .new_nomination_envelope(slot_index, vote)
    }
}
pub struct LoopbackPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    work_schedular: HWorkScheduler,
    out_queue: VecDeque<SCPMessage<N>>,
    pub in_queue: VecDeque<SCPMessage<N>>,
    remote: Weak<RefCell<LoopbackPeer<N, H>>>,
    state: Rc<RefCell<SCPPeerState>>,
    herder: Rc<RefCell<H>>,
    other_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
    self_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
}

impl<N, H> LoopbackPeer<N, H>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
{
    fn new(
        work_scheduler: &HWorkScheduler,
        we_called_remote: bool,
        herder: Rc<RefCell<H>>,
        other_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
        self_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
    ) -> Self {
        LoopbackPeer {
            work_schedular: work_scheduler.clone(),
            out_queue: Default::default(),
            in_queue: Default::default(),
            remote: Default::default(),
            state: SCPPeerState::new(we_called_remote).into(),
            herder: herder,
            other_envs,
            self_envs,
        }
    }
}

pub struct LoopbackPeerConnection<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    pub initiator: Rc<RefCell<LoopbackPeer<N, H>>>,
    pub acceptor: Rc<RefCell<LoopbackPeer<N, H>>>,
    pub initiator_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
    pub acceptor_envs: Rc<RefCell<SCPEnvelopeController<N>>>,
}
