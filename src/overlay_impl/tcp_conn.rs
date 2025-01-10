use std::{io::Write, marker::PhantomData, net::TcpStream};

use crate::{
    application::quorum::QuorumNode,
    overlay::{
        conn::{PeerConn, PeerConnBuilder},
        peer::SCPPeerConnState,
    },
    scp::nomination_protocol::NominationValue,
};

#[derive(Debug)]
pub struct TCPConn<N>
where
    N: NominationValue,
{
    pub node: QuorumNode,
    stream: Option<TcpStream>,
    state: SCPPeerConnState,
    phantom: PhantomData<N>,
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

pub struct TCPConnBuilder<N>
where
    N: NominationValue,
{
    phantom: PhantomData<N>,
}

impl<N> TCPConnBuilder<N>
where
    N: NominationValue,
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<N> PeerConnBuilder<N, TCPConn<N>> for TCPConnBuilder<N>
where
    N: NominationValue,
{
    fn build(&self, peer: &QuorumNode) -> TCPConn<N> {
        TCPConn::new(peer.clone())
    }
}

#[cfg(test)]
mod test {
    use crate::{application::quorum::QuorumNode, mock::state::MockState};

    use super::TCPConn;

    #[test]
    fn init_tcp_conn() {
        let node1 = QuorumNode::from_toml(&"node1".to_string()).unwrap();
        let tcp_conn1: TCPConn<MockState> = TCPConn::<MockState>::new(node1);

        let node2 = QuorumNode::from_toml(&"node2".to_string()).unwrap();
        let tcp_conn1 = TCPConn::<MockState>::new(node2);
    }
}
