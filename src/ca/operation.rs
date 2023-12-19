use ct_merkle::inclusion::InclusionProof;
use sha2::Sha256;

use super::{
    ca_type::SCPSignature,
    cell::Cell,
    merkle::{MerkleHash, MerkleRoot, MerkleSiblingHashes},
    root::RootEntry,
    table::{Table, TableMeta},
};

pub struct MerkleRootOperations {}

pub struct TableMerkleProof<'a> {
    pub idx: usize,
    pub sibling_hashes: InclusionProof<Sha256>,
    pub table: TableMeta<'a>,
    pub root: MerkleRoot,
}

pub struct CellMerkleProof<'a> {
    pub key: &'static str,
    pub idx: usize,
    pub sibling_hashes: InclusionProof<Sha256>,
    pub entry_cell: Cell<'a>,
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
    cell: Cell<'a>,
    proof: CellMerkleProof<'a>,
}

pub struct ReturnValueTable<'a> {
    table: Table<'a>,
    proof: CellMerkleProof<'a>,
}

pub struct ReturnError<'a> {
    reason: &'a str,
}

pub struct SetOperation<'a> {
    application_identifier: &'a str,
    full_lookup_key: &'a str,
    cell: Cell<'a>,
}

pub struct SetRootOperation<'a> {
    entry: RootEntry<'a>,
    remove: bool,
}

pub enum SetReturnValue<'a> {
    Success,
    Error(&'a str),
}

impl<'a> CellMerkleProof<'a> {}
