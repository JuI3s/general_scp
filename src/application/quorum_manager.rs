use std::collections::BTreeMap;

use log::info;

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
    pub fn get_quorum_set<N: NominationValue>(
        &self,
        statement: &SCPStatement<N>,
    ) -> Option<&QuorumSet> {
        self.quorum_set_map.get(&statement.quorum_set_hash_value())
    }

    pub fn add_quorum_set(&mut self, quorum_set: &QuorumSet) {
        if self
            .quorum_set_map
            .insert(quorum_set.hash_value(), quorum_set.clone())
            .is_none()
        {
            info!("get_quorum_set: Quorum set added: {:?}", quorum_set);
        }
    }
}

impl Default for QuorumManager {
    fn default() -> Self {
        Self {
            quorum_set_map: Default::default(),
        }
    }
}
