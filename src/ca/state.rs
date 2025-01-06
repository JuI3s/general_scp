use std::collections::HashMap;

use serde::Serialize;
use tracing::Span;

use crate::scp::{self, nomination_protocol::NominationValue};

use super::{
    cell::Cell,
    crypto::PublicKey,
    operation::{CAOperation, CellMerkleProof, SCPCAOperation, SetOperation},
    root::{RootEntry, RootEntryKey, RootListing},
    table::{find_delegation_cell, find_value_cell, TableCollection, TableId, TableOpError},
};

pub struct CAState {
    pub root_listing: RootListing,
    pub tables: HashMap<RootEntryKey, TableCollection>,
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
    NoExist,
    AlreadyExists,
}

impl NominationValue for CANominationValue {}

impl Default for CAState {
    fn default() -> Self {
        Self {
            root_listing: Default::default(),
            tables: Default::default(),
        }
    }
}

impl CAState {
    // pub fn validate_merkle_proof_for_table(
    //     &self,
    //     merkle_proof: &TableMerkleProof,
    // ) -> CAStateOpResult<()> {
    //     if let Some(hash) = merkle_proof.table.to_merkle_hash() {
    //         if merkle_proof.root != self.table_tree.root() {
    //             return Err(CAStateOpError::MerkleTreeChanged);
    //         }

    //         match self.table_tree.veritfy_inclusion_proof(
    //             &hash,
    //             merkle_proof.idx,
    //             &merkle_proof.sibling_hashes,
    //         ) {
    //             Ok(_) => Ok(()),
    //             Err(_) => Err(CAStateOpError::InvalidProof),
    //         }
    //     } else {
    //         Err(CAStateOpError::InvalidProof)
    //     }
    // }

    pub fn validate_merkle_proof_for_root<'a>(
        &mut self,
        root_key: &RootEntryKey,
        merkle_proof: &CellMerkleProof,
    ) -> CAStateOpResult<()> {
        // TODO: this implementation is probably not righ

        let root_tables = self
            .tables
            .get(root_key)
            .ok_or(CAStateOpError::MerkleTreeNotPresent)?;
        let table = root_tables
            .0
            .get(&TableId::root())
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
            .get_mut(&TableId::root())
            .ok_or(CAStateOpError::RootTableNotFound)?;

        match root_table.add_entry(cell) {
            Ok(_) => Ok(()),
            Err(err) => Err(CAStateOpError::TableOpError(err)),
        }
    }

    pub fn find_delegation_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<&Cell> {
        let root_table = self.tables.get(root_entry_key)?;
        find_delegation_cell(root_table, &TableId::root(), cell_key)
    }

    pub fn find_value_cell(
        &self,
        root_entry_key: &RootEntryKey,
        cell_key: &String,
    ) -> Option<&Cell> {
        let root_table = self.tables.get(root_entry_key)?;
        find_value_cell(root_table, &TableId::root(), cell_key)
    }

    pub fn contains_root_entry(&self, application_identifier: &String) -> bool {
        self.root_listing.0.get(application_identifier).is_some()
    }

    pub fn on_scp_operation(&mut self, scp_operation: SCPCAOperation) {
        // TODO: consider side effects
        for operation in scp_operation.0 {
            match self.on_ca_operation(operation) {
                Ok(_) => {}
                Err(_) => todo!(),
            }
        }
    }

    pub fn on_ca_operation(&mut self, ca_operation: CAOperation) -> CAStateOpResult<()> {
        match ca_operation {
            CAOperation::Empty => Ok(()),
            CAOperation::Set(set_operation) => {
                todo!()
            }
            CAOperation::SetRoot(set_root_operation) => {
                if set_root_operation.remove {
                    if self.contains_root_entry(&set_root_operation.entry.application_identifier) {
                        self.root_listing
                            .0
                            .remove(&set_root_operation.entry.application_identifier);
                        Ok(())
                    } else {
                        Err(CAStateOpError::NoExist)
                    }
                } else {
                    let entry = set_root_operation.entry;
                    self.root_listing
                        .0
                        .insert(entry.application_identifier.to_owned(), entry);
                    Ok(())
                }
            }
        }
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
