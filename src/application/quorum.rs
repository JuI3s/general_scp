use std::{collections::HashSet, net::SocketAddr};

pub type QuorumSet = HashSet<SocketAddr>;
pub type Quorum = HashSet<QuorumSet>;
