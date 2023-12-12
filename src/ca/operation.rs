use super::{ca_type::Signature, cell::Cell, merkle::MerkleHash, root::RootEntry, table::Table};

pub struct MerkleRootOperations {}

pub struct MerkleProof<'a> {
    sibling_hashes: MerkleHash,
    entry_cell: Cell<'a>,
    tree_sig: Signature,
    root_hash: MerkleHash,
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
    proof: MerkleProof<'a>,
}

pub struct ReturnValueTable<'a> {
    table: Table<'a>,
    proof: MerkleProof<'a>,
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
