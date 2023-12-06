use super::{ca_type::Signature, cell::Cell, table::Table};

pub struct MerkleRootOperations {}

pub struct MerkleProof<'a> {
    sibling_hashes: [u32; 32],
    entry_cell: Cell<'a>,
    tree_sig: Signature,
}

pub struct MerkleRootReturn {
    root_hash: [u32; 32],
    tree_sig: Signature,
}

pub struct GetOperation<'a> {
    application_identifier: &'a str,
    full_lookup_key: &'a str,
}

pub enum ReturnValue<'a> {
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
