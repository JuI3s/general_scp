
use crate::{
    herder::herder::HerderDriver,
    overlay::message::SCPMessage,
    scp::envelope::SCPEnvelopeController,
};

use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::{Rc, Weak},
};

use crate::{application::work_queue::HWorkScheduler, scp::nomination_protocol::NominationValue};

use super::peer::SCPPeerState;

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
