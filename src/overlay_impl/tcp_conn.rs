use std::{io::Write, marker::PhantomData, net::TcpStream};

use serde::Serialize;
use tokio::stream;

use crate::{
    application::quorum::QuorumNode,
    overlay::{
        conn::PeerConn,
        peer::{PeerID, SCPPeerConnState},
    },
    scp::{local_node::LocalNodeInfo, nomination_protocol::NominationValue},
};

pub struct TCPConn<N>
where
    N: NominationValue,
{
    pub node: QuorumNode,
    stream: Option<TcpStream>,
    phantom: PhantomData<N>,
    state: SCPPeerConnState,
}

impl<N: NominationValue> PeerConn<N> for TCPConn<N> {
    fn send_message(&mut self, msg: &crate::overlay::message::SCPMessage<N>) {
        if let Some(stream) = &mut self.stream {
            let bytes = serde_json::to_vec(msg).unwrap();
            let _ = stream.write(bytes.as_slice());
        }
    }

    fn set_state(&mut self, state: crate::overlay::peer::SCPPeerConnState) {
        self.state = state;
    }
}

impl<N: NominationValue> TCPConn<N> {
    pub fn new(node: QuorumNode) -> Self {
        Self {
            node,
            phantom: PhantomData,
            stream: None,
            state: SCPPeerConnState::Connecting,
        }
    }

    pub fn connect(&mut self) {
        let address = self.node.ip_addr.unwrap().to_string();
        if let Ok(stream) = TcpStream::connect(address) {
            self.stream = Some(stream);
            self.set_state(SCPPeerConnState::Connected);
        }
    }
}
