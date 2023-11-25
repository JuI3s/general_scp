use super::{
    ballot_protocol::SCPBallot, nomination_protocol::NominationValue, scp_driver::HashValue,
};

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
