use std::collections::HashMap;

use crate::scp::{nomination_protocol::NominationValue, scp::NodeID};

use super::{
    conn::PeerConn, message::SCPMessage, peer::SCPPeer
};

pub struct PeerManager<N, C>
where
    N: NominationValue,
    C: PeerConn<N>,
{
    peers: HashMap<NodeID, SCPPeer<N, C>>,
}

impl<N, C> PeerManager<N, C>
where
    N: NominationValue,
    C: PeerConn<N>,
{
    pub fn send_message(&mut self, msg: &SCPMessage<N>, node_id: &NodeID) {
        let peer = self.peers.get_mut(node_id).unwrap().conn.send_message(msg);
    }
}
