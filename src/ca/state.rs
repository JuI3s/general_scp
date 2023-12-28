use std::{
    ascii::AsciiExt,
    borrow::BorrowMut,
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    marker::PhantomData,
    os::unix::ffi::OsStrExt,
    sync::{Arc, Mutex},
};

use ct_merkle::inclusion::InclusionProof;
use digest::Key;
use log::Log;
use serde::Serialize;
use sha2::Sha256;
use syn::braced;

use crate::{herder::herder::HerderDriver, scp::nomination_protocol::NominationValue};

use super::{
    ca_type::PublicKey,
    cell::{Cell, CellRef},
    merkle::MerkleTree,
    operation::{CellMerkleProof, SetOperation, TableMerkleProof},
    root::{self, RootEntry, RootEntryKey, RootListing},
    table::{HDelegateEntry, HTable, HValueEntry, Table, TableOpError},
};

pub struct CAState {
    table_tree: MerkleTree,
    root_listing: RootListing,
    tables: BTreeMap<RootEntryKey, BTreeMap<String, HTable>>,
}

#[derive(Hash, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
pub struct CANominationValue {}

pub type CAStateOpResult<T> = std::result::Result<T, CAStateOpError>;
#[derive(PartialEq, Debug)]
pub enum CAStateOpError {
    MerkleTreeNotPresent,
    MerkleTreeChanged,
    MerkleProofInvalid,
    InvalidProof,
    InvalidCell,
    RootTableNotFound,
    TableOpError(TableOpError),
}

impl NominationValue for CANominationValue {}

impl Default for CAState {
    fn default() -> Self {
        Self {
            table_tree: Default::default(),
            root_listing: Default::default(),
            tables: Default::default(),
        }
    }
}

impl CAState {
    pub fn validate_merkle_proof_for_table(
        &self,
        merkle_proof: &TableMerkleProof,
    ) -> CAStateOpResult<()> {
        if let Some(hash) = merkle_proof.table.to_merkle_hash() {
            if merkle_proof.root != self.table_tree.root() {
                return Err(CAStateOpError::MerkleTreeChanged);
            }

            match self.table_tree.veritfy_inclusion_proof(
                &hash,
                merkle_proof.idx,
                &merkle_proof.sibling_hashes,
            ) {
                Ok(_) => Ok(()),
                Err(_) => Err(CAStateOpError::InvalidProof),
            }
        } else {
            Err(CAStateOpError::InvalidProof)
        }
    }

    pub fn validate_merkle_proof_for_cell<'a>(
        &self,
        root_key: &RootEntryKey,
        merkle_proof: &CellMerkleProof,
    ) -> CAStateOpResult<()> {
        if let Some(table) = self.get_table(root_key, merkle_proof.key) {
            // Check tree root has not changed.
            if table.borrow().merkle_tree.root() != merkle_proof.root {
                return Err(CAStateOpError::MerkleTreeChanged);
            }

            // Check inclusion proof
            if merkle_proof
                .entry_cell
                .to_merkle_hash()
                .is_some_and(|hash| {
                    table
                        .borrow()
                        .merkle_tree
                        .veritfy_inclusion_proof(
                            &hash,
                            merkle_proof.idx,
                            &merkle_proof.sibling_hashes,
                        )
                        .is_ok()
                })
            {
                Ok(())
            } else {
                Err(CAStateOpError::InvalidProof)
            }
        } else {
            Err(CAStateOpError::MerkleTreeNotPresent)
        }
    }

    pub fn validate_set_operation(&self, set_opt: &SetOperation) -> bool {
        todo!()
    }

    pub fn insert_cell(
        &mut self,
        root_entry_key: &RootEntryKey,
        cell: Cell,
    ) -> CAStateOpResult<()> {
        if let Some(root_table) = self.get_root_table(root_entry_key) {
            match root_table.as_ref().borrow_mut().add_entry(cell) {
                Ok(_) => Ok(()),
                Err(err) => Err(CAStateOpError::TableOpError(err)),
            }
        } else {
            Err(CAStateOpError::RootTableNotFound)
        }
    }

    pub fn insert_root_entry(&mut self, root_entry: &RootEntry) {
        todo!()
    }

    pub fn find_delegation_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<HDelegateEntry> {
        let root_table = self.get_root_table(root_entry_key)?;
        Table::find_delegation_cell(&root_table, cell_key)
    }

    pub fn find_value_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<HValueEntry> {
        let root_table = self.get_root_table(root_entry_key)?;
        Table::find_value_cell(&root_table, cell_key)
    }

    fn get_root_table(&self, root_entry_key: &RootEntryKey) -> Option<HTable> {
        self.get_table(root_entry_key, "")
    }

    fn get_table(&self, root_entry: &RootEntryKey, table_key: &str) -> Option<HTable> {
        let root_table = self.tables.get(root_entry)?;
        root_table.get(table_key).map(|v| v.clone())
    }

    pub fn contains_root_entry(
        &self,
        namespace_root_key: &PublicKey,
        application_identifier: String,
    ) -> bool {
        self.root_listing
            .get_entry(namespace_root_key, application_identifier)
            .is_some()
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

    #[test]
    fn box_value() {
        let a = Box::new(1);
        let mut b = a.clone();
        let y = &mut b;
        *y = Box::new(2);
        assert_eq!(*a, 1);
    }
}
