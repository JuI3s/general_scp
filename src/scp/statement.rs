use std::{
    collections::HashSet,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::application::quorum::QuorumSet;

use super::{
    ballot_protocol::SCPBallot, nomination_protocol::NominationValue, scp::NodeID,
    scp_driver::HashValue,
};

pub type HSCPStatement<N> = Arc<Mutex<SCPStatement<N>>>;

pub trait MakeStatement<N>
where
    N: NominationValue,
{
    // The struct implementing this trait is responsible for passing the relevant
    // bookkeeping information to make a new scp statement, e.g. local node id,
    // quorum set or quorum set hash, etc.

    // Make a new scp nominate statement with empty votes and accepts fields.
    fn new_nominate_statement(&self, vote: N) -> SCPStatementNominate<N>;
}

#[derive(PartialEq, Eq, Ord)]
pub enum SCPStatementType {
    Nominate,
    Prepare,
    Confirm,
    Externalize,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SCPStatement<N>
where
    N: NominationValue,
{
    Prepare(SCPStatementPrepare<N>),
    Confirm(SCPStatementConfirm<N>),
    Externalize(SCPStatementExternalize<N>),
    Nominate(SCPStatementNominate<N>),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SCPStatementNominate<N>
where
    N: NominationValue,
{
    pub node_id: NodeID,
    #[serde(with = "serde_bytes")]
    pub quorum_set_hash: HashValue,
    pub votes: Vec<N>,
    pub accepted: Vec<N>,

    pub quorum_set: Option<QuorumSet>,
}

impl<N: NominationValue> Debug for SCPStatementNominate<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SCPStatementNominate")
            .field("node_id", &self.node_id)
            .field("accepted", &self.accepted)
            .field("votes", &self.votes)
            .finish()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SCPStatementPrepare<N>
where
    N: NominationValue,
{
    #[serde(with = "serde_bytes")]
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot<N>,
    pub prepared: Option<SCPBallot<N>>,
    pub prepared_prime: Option<SCPBallot<N>>,
    pub num_commit: u32,
    pub num_high: u32,

    pub quorum_set: Option<QuorumSet>,
}

impl<N: NominationValue> Debug for SCPStatementPrepare<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SCPStatementPrepare")
            // .field("quorum_set_hash", &self.quorum_set_hash)
            .field("ballot", &self.ballot)
            .field("prepared", &self.prepared)
            .field("prepared_prime", &self.prepared_prime)
            .field("num_commit", &self.num_commit)
            .field("num_high", &self.num_high)
            .field("quorum_set", &self.quorum_set)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SCPStatementConfirm<N>
where
    N: NominationValue,
{
    #[serde(with = "serde_bytes")]
    pub quorum_set_hash: HashValue,
    pub ballot: SCPBallot<N>,
    pub num_prepared: u32,
    pub num_commit: u32,
    pub num_high: u32,

    pub quorum_set: Option<QuorumSet>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SCPStatementExternalize<N>
where
    N: NominationValue,
{
    #[serde(with = "serde_bytes")]
    pub commit_quorum_set_hash: HashValue,
    pub commit: SCPBallot<N>,
    pub num_high: u32,

    pub commit_quorum_set: Option<QuorumSet>,
}

impl SCPStatementType {
    fn value(&self) -> u64 {
        match self {
            SCPStatementType::Nominate => 0,
            SCPStatementType::Prepare => 1,
            SCPStatementType::Confirm => 2,
            SCPStatementType::Externalize => 3,
        }
    }
}

impl PartialOrd for SCPStatementType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value().partial_cmp(&other.value())
    }
}

impl<N> SCPStatement<N>
where
    N: NominationValue,
{
    pub fn statement_type(&self) -> SCPStatementType {
        match self {
            SCPStatement::Nominate(_) => SCPStatementType::Nominate,
            SCPStatement::Prepare(_) => SCPStatementType::Prepare,
            SCPStatement::Confirm(_) => SCPStatementType::Confirm,
            SCPStatement::Externalize(_) => SCPStatementType::Externalize,
        }
    }

    pub fn working_ballot(&self) -> SCPBallot<N> {
        // TODO: not important but can return a COW?
        match self {
            SCPStatement::Prepare(st) => st.ballot.to_owned(),
            SCPStatement::Confirm(st) => {
                SCPBallot::<N>::new(st.num_commit.to_owned(), st.ballot.value.to_owned())
            }
            SCPStatement::Externalize(st) => st.commit.to_owned(),
            SCPStatement::Nominate(st) => {
                panic!("Nominate statement does not have a working ballot.")
            }
        }
    }

    pub fn get_nomination_values(&self) -> HashSet<N> {
        // TODO: is this comment right?
        // Called in the ballot protocol phase. Return nomination values in contained in
        // statement which is assumed to be not a nominate statement.

        let mut ret = HashSet::new();

        match self {
            SCPStatement::Prepare(st) => {
                let ballot = &st.ballot;

                if ballot.counter != 0 {
                    ret.insert(ballot.value.to_owned());
                }

                if let Some(prepared) = &st.prepared {
                    ret.insert(prepared.value.to_owned());
                }

                if let Some(prepared_prime) = &st.prepared_prime {
                    ret.insert(prepared_prime.value.to_owned());
                }
            }
            SCPStatement::Confirm(st) => {
                ret.insert(st.ballot.value.to_owned());
            }
            SCPStatement::Externalize(st) => {
                ret.insert(st.commit.value.to_owned());
            }
            SCPStatement::Nominate(st) => {}
        }

        ret
    }

    fn as_nominate_statement(&self) -> &SCPStatementNominate<N> {
        match self {
            SCPStatement::Prepare(_) | SCPStatement::Confirm(_) | SCPStatement::Externalize(_) => {
                panic!()
            }
            SCPStatement::Nominate(self_st) => self_st,
        }
    }

    fn as_prepare_statement(&self) -> &SCPStatementPrepare<N> {
        match self {
            SCPStatement::Nominate(_) | SCPStatement::Confirm(_) | SCPStatement::Externalize(_) => {
                panic!()
            }
            SCPStatement::Prepare(st) => st,
        }
    }

    fn as_confirm_statement(&self) -> &SCPStatementConfirm<N> {
        match self {
            SCPStatement::Nominate(_) | SCPStatement::Externalize(_) | SCPStatement::Prepare(_) => {
                panic!()
            }
            SCPStatement::Confirm(st) => st,
        }
    }

    fn as_externalize_statement(&self) -> &SCPStatementExternalize<N> {
        match self {
            SCPStatement::Prepare(_) | SCPStatement::Confirm(_) | SCPStatement::Nominate(_) => {
                panic!()
            }
            SCPStatement::Externalize(st) => st,
        }
    }

    pub fn is_newer_than(&self, other: &Self) -> bool {
        let self_type = self.statement_type();
        let other_type = other.statement_type();

        if self_type != other_type {
            return self_type > other_type;
        }

        match self_type {
            SCPStatementType::Nominate => {
                let self_st = self.as_nominate_statement();
                let other_st = other.as_nominate_statement();
                !self_st.is_older_than(other_st)
            }
            SCPStatementType::Prepare => {
                let self_st = self.as_prepare_statement();
                let other_st = other.as_prepare_statement();
                self_st.is_newer_than(other_st)
            }
            SCPStatementType::Confirm => {
                let self_st = self.as_confirm_statement();
                let other_st = other.as_confirm_statement();
                self_st.is_newer_than(other_st)
            }
            SCPStatementType::Externalize => {
                // can't have duplicate EXTERNALIZE statements
                false
            }
        }
    }
}

impl<N> SCPStatementPrepare<N>
where
    N: NominationValue,
{
    fn is_newer_than(&self, other: &Self) -> bool {
        // Lexicographical order between PREPARE statements:
        // (b, p, p', h)
        if other.ballot < self.ballot {
            true
        } else if self.ballot == self.ballot {
            if other.prepared < self.prepared {
                true
            } else if other.prepared == self.prepared {
                if other.prepared_prime < self.prepared_prime {
                    true
                } else {
                    other.num_high < self.num_high
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl<N> SCPStatementConfirm<N>
where
    N: NominationValue,
{
    fn is_newer_than(&self, other: &Self) -> bool {
        // sorted by (b, p, p', h) (p' = 0 implicitly)
        if other.ballot < self.ballot {
            true
        } else if other.ballot == self.ballot {
            if other.num_prepared == self.num_prepared {
                other.num_high < self.num_high
            } else {
                other.num_prepared < self.num_prepared
            }
        } else {
            false
        }
    }
}

impl<N> SCPStatementNominate<N>
where
    N: NominationValue,
{
    pub fn new(quorum_set: &QuorumSet, votes: Vec<N>, accepted: Vec<N>) -> Self {
        SCPStatementNominate {
            quorum_set_hash: quorum_set.hash_value(),
            votes,
            accepted,
            quorum_set: Some(quorum_set.clone()),
            node_id: "".into(),
        }
    }

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
        // The set of nomination statements satisfies a partial ordering. An old_st is
        // older than a new_st if the votes of old_st are contained in the votes of
        // new_st. If old_st and new_st have the same set of votes, then old_st is older
        // than new_st if the accepted vector of old_st is contained in the accepted
        // vector of the new_st.
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

    pub fn quorum_set(&self) -> Option<&QuorumSet> {
        match self {
            SCPStatement::Prepare(st) => st.quorum_set.as_ref(),
            SCPStatement::Confirm(st) => st.quorum_set.as_ref(),
            SCPStatement::Externalize(st) => st.commit_quorum_set.as_ref(),
            SCPStatement::Nominate(st) => st.quorum_set.as_ref(),
        }
    }
}
