use std::collections::BTreeMap;

use crate::scp::{
    nomination_protocol::{NominationProtocol, NominationValue},
    scp_driver::HashValue,
    statement::SCPStatement,
};

use super::quorum::QuorumSet;

pub struct QuorumManager {
    quorum_set_map: BTreeMap<HashValue, QuorumSet>,
}

impl QuorumManager {
    fn get_quorum_set<N: NominationValue>(
        &self,
        statement: &SCPStatement<N>,
    ) -> Option<&QuorumSet> {
        self.quorum_set_map.get(&statement.quorum_set_hash_value())
    }
}

impl Default for QuorumManager {
    fn default() -> Self {
        Self {
            quorum_set_map: Default::default(),
        }
    }
}
