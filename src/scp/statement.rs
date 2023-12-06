use std::sync::{Arc, Mutex};

use super::{
    ballot_protocol::SCPBallot, nomination_protocol::NominationValue, scp_driver::HashValue,
};

pub type HSCPStatement = Arc<Mutex<SCPStatement>>;

pub enum SCPStatement {
    Prepare(SCPStatementPrepare),
    Confirm(SCPStatementConfirm),
    Externalize(SCPStatementExternalize),
    Nominate(SCPStatementNominate),
}

pub struct SCPStatementNominate {
    pub quorum_set_hash: HashValue,
    pub votes: Vec<NominationValue>,
    pub accepted: Vec<NominationValue>,
}

pub struct SCPStatementPrepare {
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot,
    pub prepared: Option<SCPBallot>,
    pub prepared_prime: Option<SCPBallot>,
    pub num_commit: u32,
    pub num_high: u32,
    pub from_self: bool,
}

pub struct SCPStatementConfirm {
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot,
    pub num_prepared: u32,
    pub num_commit: u32,
    pub num_high: u32,
}

pub struct SCPStatementExternalize {
    pub commit_quorum_set_hash: HashValue,
    pub commit: SCPBallot,
    pub num_high: u32,
}

fn is_subset(left: &Vec<NominationValue>, right: &Vec<NominationValue>) -> (bool, bool) {
    let mut is_subset = false;
    let mut equal = false;
    if left.len() <= right.len() {
        is_subset = left.iter().all(|value| right.contains(value));
        equal = is_subset && left.len() == right.len();
    }

    (is_subset, !equal)
}

impl SCPStatementNominate {
    pub fn is_older_than(&self, other: &SCPStatementNominate) -> bool {
        let mut ret = false;

        let (is_subset_votes, equal_votes_grown) = is_subset(&self.votes, &other.votes);
        if is_subset_votes {
            let (is_subset_accepted, equal_accepted_grown) =
                is_subset(&self.accepted, &other.accepted);
            if is_subset_accepted {
                ret = equal_votes_grown || equal_accepted_grown;
            }
        }

        ret
    }
}

impl SCPStatement {
    pub fn quorum_set_hash_value(&self) -> HashValue {
        match self {
            SCPStatement::Prepare(st) => st.quorum_set_hash,
            SCPStatement::Confirm(st) => st.quorum_set_hash,
            SCPStatement::Externalize(st) => st.commit_quorum_set_hash,
            SCPStatement::Nominate(st) => st.quorum_set_hash,
        }
    }
}
