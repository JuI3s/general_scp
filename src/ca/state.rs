use std::collections::{BTreeMap, BTreeSet};

use super::{
    cell::Cell,
    merkle::MerkleTree,
    operation::{MerkleProof, SetOperation},
    root::{RootEntry, RootListing},
    table::{Table, TableEntry},
};

pub struct CAState<'a> {
    table_tree: MerkleTree,
    value_cell_trees: BTreeMap<TableEntry<'a>, MerkleTree>,
    root_listing: RootListing<'a>,
    tables: BTreeSet<Table<'a>>,
}

impl<'a> Default for CAState<'a> {
    fn default() -> Self {
        Self {
            table_tree: Default::default(),
            value_cell_trees: Default::default(),
            root_listing: Default::default(),
            tables: Default::default(),
        }
    }
}

impl<'a> CAState<'a> {
    pub fn validate_merkle_proof(&self, merkle_proof: &MerkleProof) {
        todo!()
    }

    pub fn validate_set_operation(&self, set_opt: &SetOperation) -> bool {
        todo!()
    }

    pub fn insert_cell(&mut self, cell: &Cell) {
        todo!()
    }

    pub fn insert_root_entry(&mut self, root_entry: &RootEntry) {
        todo!()
    }

    pub fn contains_cell(&self, cell: &Cell) -> bool {
        todo!()
    }

    pub fn contains_root_entry(&self, root_entry: &RootEntry) -> bool {
        todo!()
    }

    pub fn to_toml(&self) {
        todo!()
    }

    pub fn from_toml(&self) {
        todo!()
    }
}

// Creating or updating a cell at a specified path requires once again
// the full lookup key, as well as the new version of the cell to place.
// The new cell must be well-formed under the validation checks
// described in the previous section, else an "ERROR" is returned.  For
// example, updating a cell's owner without a signature by the previous
// owning key should not succeed.  Both value cells and new/updated
// delegations may be created through this method.  Removing cells from
// tables (after their commitment timestamps have expired) can be
// accomplished by replacing the value or delegated namespace with an
// empty value and setting the owner's key to that of the table
// authority.  Asking the consensus layer to approve a new root entry
// follows a similar process, although the application identifier and
// lookup key is unnecessary (see "SetRootOperation").  Nodes can also
// trigger votes to remove entries from the root key listing to redress
// misbehaving applications.

#[cfg(test)]
mod tests {

    use super::*;
}
