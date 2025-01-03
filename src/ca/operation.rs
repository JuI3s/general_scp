use ct_merkle::inclusion::InclusionProof;
use sha2::Sha256;

use super::{
    crypto::SCPSignature,
    cell::Cell,
    merkle::MerkleRoot,
    root::RootEntry,
    table::{Table, TableMeta},
};

pub struct MerkleRootOperations {}

pub struct TableMerkleProof {
    pub idx: usize,
    pub sibling_hashes: InclusionProof<Sha256>,
    pub table: TableMeta,
    pub root: MerkleRoot,
}

pub struct CellMerkleProof<'a> {
    pub key: &'a str,
    pub idx: usize,
    pub sibling_hashes: InclusionProof<Sha256>,
    pub entry_cell: Cell,
    pub tree_sig: SCPSignature,
    pub root: MerkleRoot,
}

pub struct GetOperation<'a> {
    application_identifier: &'a str,
    full_lookup_key: &'a str,
}

pub enum GetReturnValue<'a> {
    Cell(ReturnValueCell<'a>),
    Table(ReturnValueTable<'a>),
    Error(ReturnError<'a>),
}

pub struct ReturnValueCell<'a> {
    cell: Cell,
    proof: CellMerkleProof<'a>,
}

pub struct ReturnValueTable<'a> {
    table: Table,
    proof: CellMerkleProof<'a>,
}

pub struct ReturnError<'a> {
    reason: &'a str,
}

pub enum SCPOperation {
    Set(SetOperation),
    SetRoot(SetRootOperation),
}

pub struct SetOperation {
    application_identifier: String,
    full_lookup_key: String,
    cell: Cell,
}

// https://datatracker.ietf.org/doc/html/draft-watson-dinrg-delmap-01#page-7 (p.9)
// struct SetRootOperation {
//     rootentry e;
//     bool remove;
// }

pub struct SetRootOperation {
    pub entry: RootEntry,
    pub remove: bool,
}

pub enum SetReturnValue<'a> {
    Success,
    Error(&'a str),
}

impl<'a> CellMerkleProof<'a> {}
