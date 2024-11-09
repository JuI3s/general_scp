use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use crate::{
    herder::herder::HerderDriver,
    mock::state::{MockState, MockStateDriver},
    scp::{local_node::LocalNodeInfo, nomination_protocol::NominationValue, scp::NodeID},
};

use super::overlay_manager::OverlayManager;

pub struct LocalNode<N, H>
where
    N: NominationValue,
    H: HerderDriver<N>,
{
    herder: Rc<RefCell<H>>,
    overlay_manager: Box<dyn OverlayManager<N, H>>,    

    phantom: PhantomData<N>,
}

pub struct Simulation {
    pub nodes: HashMap<NodeID, LocalNode<MockState, MockStateDriver>>,
}


