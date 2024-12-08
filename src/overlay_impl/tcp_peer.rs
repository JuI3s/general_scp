use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    application::work_queue::WorkScheduler,
    herder::herder::{HerderBuilder, HerderDriver},
    overlay::peer_node::PeerNode,
    scp::{local_node::LocalNodeInfo, nomination_protocol::NominationValue},
};

use super::tcp_conn::TCPConnBuilder;


pub struct TCPPeerBuilder<N, H, HB>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
    HB: HerderBuilder<N, H>,
{
    herder_builder: HB,
    phantom_h: PhantomData<H>,
    phantom_n: PhantomData<N>,
}

impl<N, H, HB> TCPPeerBuilder<N, H, HB>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
    HB: HerderBuilder<N, H>,
{
    pub fn new(herder_builder: HB) -> Self {
        Self {
            herder_builder,
            phantom_h: PhantomData,
            phantom_n: PhantomData,
        }
    }

    pub fn build_node(
        &mut self,
        local_node_info: LocalNodeInfo<N>,
    ) -> Rc<RefCell<PeerNode<N, H, super::tcp_conn::TCPConn<N>, TCPConnBuilder<N>>>> {
        let conn_builder = TCPConnBuilder::new();
        let node = PeerNode::new(
            local_node_info.node_id.clone(),
            self.herder_builder.build(),
            conn_builder,
            local_node_info,
            Rc::new(RefCell::new(WorkScheduler::new(None))),
        );
        Rc::new(RefCell::new(node))
    }
}

