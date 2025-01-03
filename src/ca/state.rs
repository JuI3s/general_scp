use std::{borrow::BorrowMut, collections::{BTreeMap, HashMap}};

use serde::Serialize;

use crate::scp::nomination_protocol::NominationValue;

use super::{
    ca_type::PublicKey,
    cell::Cell,
    merkle::MerkleTree,
    operation::{CellMerkleProof, SetOperation, TableMerkleProof},
    root::{RootEntry, RootEntryKey, RootListing},
    table::{
        self, find_delegation_cell, find_value_cell, HTable, Table, TableCollection, TableId,
        TableOpError, ROOT_TABLE_ID,
    },
};

pub struct CAState {
    table_tree: MerkleTree,
    root_listing: RootListing,
    tables: HashMap<RootEntryKey, TableCollection>,
}

#[derive(Hash, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Debug)]
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

    pub fn validate_merkle_proof_for_root<'a>(
        &mut self,
        root_key: &RootEntryKey,
        merkle_proof: &CellMerkleProof,
    ) -> CAStateOpResult<()> {
        let root_tables = self
            .tables
            .get(root_key)
            .ok_or(CAStateOpError::MerkleTreeNotPresent)?;
        let table = root_tables
            .0
            .get(&ROOT_TABLE_ID)
            .ok_or(CAStateOpError::MerkleTreeNotPresent)?;

        // Check tree root has not changed.
        if table.merkle_tree.root() != merkle_proof.root {
            return Err(CAStateOpError::MerkleTreeChanged);
        }

        // Check inclusion proof
        if merkle_proof
            .entry_cell
            .to_merkle_hash()
            .is_some_and(|hash| {
                table
                    .merkle_tree
                    .veritfy_inclusion_proof(&hash, merkle_proof.idx, &merkle_proof.sibling_hashes)
                    .is_ok()
            })
        {
            Ok(())
        } else {
            Err(CAStateOpError::InvalidProof)
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
        let root_tables = self
            .tables
            .get_mut(root_entry_key)
            .ok_or(CAStateOpError::RootTableNotFound)?;
        let root_table = root_tables
            .0
            .get_mut(&ROOT_TABLE_ID)
            .ok_or(CAStateOpError::RootTableNotFound)?;

        match root_table.add_entry(cell) {
            Ok(_) => Ok(()),
            Err(err) => Err(CAStateOpError::TableOpError(err)),
        }
    }

    pub fn insert_root_entry(&mut self, root_entry: &RootEntry) {
        todo!()
    }

    pub fn find_delegation_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<&Cell> {
        let root_table = self.tables.get(root_entry_key)?;
        find_delegation_cell(root_table, &ROOT_TABLE_ID, cell_key)
    }

    pub fn find_value_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<&Cell> {
        let root_table = self.tables.get(root_entry_key)?;
        find_value_cell(root_table, &ROOT_TABLE_ID, cell_key)
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
    use crate::ca::state::CAState;

    #[test]
    fn test_ca_state() {
        let mut ca_state = CAState::default();
    }
}
