use std::sync::{Arc, Mutex};

use super::{
    ballot_protocol::SCPBallot,
    nomination_protocol::{NominationProtocol, NominationValue, SCPNominationValue},
    scp_driver::HashValue,
};

pub type HSCPStatement<N> = Arc<Mutex<SCPStatement<N>>>;

pub enum SCPStatement<N>
where
    N: NominationValue,
{
    Prepare(SCPStatementPrepare<N>),
    Confirm(SCPStatementConfirm<N>),
    Externalize(SCPStatementExternalize<N>),
    Nominate(SCPStatementNominate<N>),
}

pub struct SCPStatementNominate<N>
where
    N: NominationValue,
{
    pub quorum_set_hash: HashValue,
    pub votes: Vec<N>,
    pub accepted: Vec<N>,
}

pub struct SCPStatementPrepare<N>
where
    N: NominationValue,
{
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot<N>,
    pub prepared: Option<SCPBallot<N>>,
    pub prepared_prime: Option<SCPBallot<N>>,
    pub num_commit: u32,
    pub num_high: u32,
    pub from_self: bool,
}

pub struct SCPStatementConfirm<N>
where
    N: NominationValue,
{
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot<N>,
    pub num_prepared: u32,
    pub num_commit: u32,
    pub num_high: u32,
}

pub struct SCPStatementExternalize<N>
where
    N: NominationValue,
{
    pub commit_quorum_set_hash: HashValue,
    pub commit: SCPBallot<N>,
    pub num_high: u32,
}

impl<N> SCPStatementNominate<N>
where
    N: NominationValue,
{
    fn is_subset(left: &Vec<N>, right: &Vec<N>) -> (bool, bool) {
        let mut is_subset = false;
        let mut equal = false;
        if left.len() <= right.len() {
            is_subset = left.iter().all(|value| right.contains(value));
            equal = is_subset && left.len() == right.len();
        }

        (is_subset, !equal)
    }

    pub fn is_older_than(&self, other: &Self) -> bool {
        let mut ret = false;

        let (is_subset_votes, equal_votes_grown) = Self::is_subset(&self.votes, &other.votes);
        if is_subset_votes {
            let (is_subset_accepted, equal_accepted_grown) =
                Self::is_subset(&self.accepted, &other.accepted);
            if is_subset_accepted {
                ret = equal_votes_grown || equal_accepted_grown;
            }
        }

        ret
    }
}

impl<N> SCPStatement<N>
where
    N: NominationValue,
{
    pub fn quorum_set_hash_value(&self) -> HashValue {
        match self {
            SCPStatement::Prepare(st) => st.quorum_set_hash,
            SCPStatement::Confirm(st) => st.quorum_set_hash,
            SCPStatement::Externalize(st) => st.commit_quorum_set_hash,
            SCPStatement::Nominate(st) => st.quorum_set_hash,
        }
    }
}
